//! Mock-driven harness for plan unit F50a (typed preflight gate).
//!
//! Covers success, fail-closed failure at each of the seven stages (one
//! table), and the single shared deadline / expected generation.

use std::time::{Duration, Instant};

use engram_indexer::preflight::{
    Build, Failure, Preflight, StageKind, Succeeded, VerificationError, Verifier,
};

const GENERATION: &str = "gen-0042";

const ALL_STAGES: [(StageKind, Failure); 7] = [
    (StageKind::Build, Failure::Build),
    (StageKind::Seal, Failure::Seal),
    (StageKind::Publish, Failure::Publish),
    (StageKind::DaemonVerified, Failure::DaemonVerified),
    (StageKind::HealthVerified, Failure::HealthVerified),
    (StageKind::CliProbeVerified, Failure::CliProbeVerified),
    (StageKind::McpProbeVerified, Failure::McpProbeVerified),
];

/// Records every call and optionally fails at one stage.
#[derive(Default)]
struct MockVerifier {
    fail_at: Option<StageKind>,
    calls: Vec<(StageKind, Instant, String)>,
}

impl Verifier for MockVerifier {
    fn verify(
        &mut self,
        stage: StageKind,
        deadline: Instant,
        expected_generation: &str,
    ) -> Result<(), VerificationError> {
        self.calls
            .push((stage, deadline, expected_generation.to_owned()));
        if self.fail_at == Some(stage) {
            Err(VerificationError::new("injected failure"))
        } else {
            Ok(())
        }
    }
}

fn far_deadline() -> Instant {
    Instant::now() + Duration::from_secs(3600)
}

/// Runs all seven stages in order, returning the typed result.
fn run<V: Verifier>(deadline: Instant, verifier: &mut V) -> Result<Preflight<Succeeded>, Failure> {
    Preflight::<Build>::start(deadline, GENERATION.to_owned())
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)
}

#[test]
fn success_reaches_succeeded_through_all_seven_stages() {
    let deadline = far_deadline();
    let mut mock = MockVerifier::default();

    let done = run(deadline, &mut mock).expect("all stages verify");

    let stages: Vec<StageKind> = mock.calls.iter().map(|call| call.0).collect();
    let expected: Vec<StageKind> = ALL_STAGES.iter().map(|entry| entry.0).collect();
    assert_eq!(stages, expected, "stages verified in declared order");
    assert_eq!(done.deadline(), deadline);
    assert_eq!(done.expected_generation(), GENERATION);
}

#[test]
fn failure_at_each_stage_is_typed_and_fail_closed() {
    for (index, (stage, failure)) in ALL_STAGES.iter().enumerate() {
        let mut mock = MockVerifier {
            fail_at: Some(*stage),
            ..MockVerifier::default()
        };

        let result = run(far_deadline(), &mut mock);

        assert_eq!(
            result.err(),
            Some(*failure),
            "failure at {stage:?} must name that stage and never succeed"
        );
        let called: Vec<StageKind> = mock.calls.iter().map(|call| call.0).collect();
        let expected: Vec<StageKind> = ALL_STAGES[..=index].iter().map(|entry| entry.0).collect();
        assert_eq!(called, expected, "no stage after {stage:?} may be called");
    }
}

#[test]
fn one_deadline_and_one_generation_span_every_stage() {
    let deadline = far_deadline();
    let mut mock = MockVerifier::default();

    let _done = run(deadline, &mut mock).expect("all stages verify");

    assert_eq!(mock.calls.len(), 7);
    for (stage, seen_deadline, seen_generation) in &mock.calls {
        assert_eq!(*seen_deadline, deadline, "deadline at {stage:?}");
        assert_eq!(seen_generation, GENERATION, "generation at {stage:?}");
    }
}

/// Succeeds at every stage but sleeps past the deadline during one stage.
struct SlowVerifier {
    slow_at: StageKind,
    sleep: Duration,
    calls: Vec<StageKind>,
}

impl Verifier for SlowVerifier {
    fn verify(
        &mut self,
        stage: StageKind,
        _deadline: Instant,
        _expected_generation: &str,
    ) -> Result<(), VerificationError> {
        self.calls.push(stage);
        if stage == self.slow_at {
            std::thread::sleep(self.sleep);
        }
        Ok(())
    }
}

#[test]
fn overrun_during_each_stage_fails_that_stage_without_restarting_deadline() {
    for (index, (stage, failure)) in ALL_STAGES.iter().enumerate() {
        let mut slow = SlowVerifier {
            slow_at: *stage,
            sleep: Duration::from_millis(300),
            calls: Vec::new(),
        };
        let deadline = Instant::now() + Duration::from_millis(200);

        let outcome = run(deadline, &mut slow);

        assert_eq!(
            outcome.err(),
            Some(*failure),
            "overrun in {stage:?} must fail that stage"
        );
        let expected: Vec<StageKind> = ALL_STAGES[..=index].iter().map(|entry| entry.0).collect();
        assert_eq!(
            slow.calls, expected,
            "no stage after {stage:?} may be called"
        );
    }
}

#[test]
fn failure_displays_stage_and_is_an_error() {
    for (_, failure) in &ALL_STAGES {
        let error: &dyn std::error::Error = failure;
        assert!(error.to_string().starts_with("preflight failed at stage: "));
    }
    assert!(Failure::Seal.to_string().contains("seal"));
}

#[test]
fn expired_deadline_fails_first_stage_without_calling_verifier() {
    let mut mock = MockVerifier::default();
    let expired = Instant::now();

    let result = run(expired, &mut mock);

    assert_eq!(result.err(), Some(Failure::Build));
    assert!(mock.calls.is_empty(), "verifier must not be called");
}

/// Plan unit F50b stages need the real database backend (cozo).
#[cfg(feature = "cozo-backend")]
mod production_stages {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use engram::daemon::ipc_server::ipc_endpoint;
    use engram::services::generations::candidate_build::{
        ReadServerLayout, current_published_revision, open_generation_store, read_server_layout,
    };
    use engram::services::generations::{GenerationActivator, GenerationRevision};
    use engram::shim::lifecycle::check_health;
    use engram_indexer::preflight::{
        Build, Preflight, ProductionVerifier, StageKind, VerificationError, VerificationErrorKind,
        Verifier,
    };
    use tempfile::TempDir;

    use super::far_deadline;

    // ---------------------------------------------------------------------------
    // Plan unit F50b (142.079-T): ProductionVerifier Build, Seal and Publish stages.
    // ---------------------------------------------------------------------------

    const REAL_GENERATION: &str = "preflight-gen-1";

    /// Spaced git fixture with a two-function `lib.rs`.
    fn real_fixture() -> (TempDir, ReadServerLayout) {
        let dir = tempfile::Builder::new()
            .prefix("preflight stages ")
            .tempdir()
            .expect("tempdir");
        let root = dir.path().join("work space");
        fs::create_dir_all(root.join(".git")).expect("create .git");
        fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
        fs::create_dir_all(root.join("src")).expect("create src");
        fs::write(
            root.join("src").join("lib.rs"),
            "pub fn alpha() -> u32 {\n    1\n}\n\npub fn beta() -> u32 {\n    2\n}\n",
        )
        .expect("write lib.rs");
        let layout = read_server_layout(root.to_str().expect("utf8 path")).expect("layout");
        (dir, layout)
    }

    fn production(layout: &ReadServerLayout, deadline: Instant) -> ProductionVerifier {
        ProductionVerifier::new(layout.clone(), PathBuf::from("engram"), deadline)
            .expect("construct production verifier")
    }

    fn candidate_db(layout: &ReadServerLayout, id: &str) -> PathBuf {
        layout.generations_root.join(id).join("engram.db")
    }

    fn manifest_bytes(layout: &ReadServerLayout) -> Vec<u8> {
        let store = open_generation_store(layout, false).expect("open store");
        fs::read(store.active_manifest_path()).expect("read manifest")
    }

    fn function_count(db: &cozo::DbInstance) -> i64 {
        let rows = db
            .run_script(
                "?[count(id)] := *function_meta { id }",
                BTreeMap::new(),
                cozo::ScriptMutability::Immutable,
            )
            .expect("count script");
        rows.rows[0][0].get_int().expect("integer count")
    }

    fn serving_generation(layout: &ReadServerLayout) -> (String, i64) {
        let store = open_generation_store(layout, false).expect("open store");
        let activator = GenerationActivator::new(
            store,
            layout.runtime_root.clone(),
            layout.expected_identity(),
            layout.activation_deadline,
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let context = runtime
            .block_on(activator.activate_initial())
            .expect("activate");
        (
            context.generation_id().as_str().to_owned(),
            function_count(context.opened_generation().db()),
        )
    }

    /// Publishes `id` through the stage verifier and returns its layout state.
    fn publish_through_stages(layout: &ReadServerLayout, id: &str) {
        let mut verifier = production(layout, far_deadline());
        for stage in [StageKind::Build, StageKind::Seal, StageKind::Publish] {
            verifier
                .verify(stage, far_deadline(), id)
                .unwrap_or_else(|e| panic!("{stage:?} must verify: {e}"));
        }
    }

    #[test]
    fn build_seal_publish_stages_succeed_in_order() {
        let (_dir, layout) = real_fixture();
        let mut verifier = production(&layout, far_deadline());

        verifier
            .verify(StageKind::Build, far_deadline(), REAL_GENERATION)
            .expect("build stage");
        assert!(
            candidate_db(&layout, REAL_GENERATION).is_file(),
            "Build must construct the candidate database"
        );
        let store = open_generation_store(&layout, false).expect("open store");
        assert!(
            !store.active_manifest_path().exists(),
            "Build must not publish"
        );

        verifier
            .verify(StageKind::Seal, far_deadline(), REAL_GENERATION)
            .expect("seal stage");
        assert!(
            !store.active_manifest_path().exists(),
            "Seal must not publish"
        );

        verifier
            .verify(StageKind::Publish, far_deadline(), REAL_GENERATION)
            .expect("publish stage");
        assert_eq!(
            current_published_revision(&store).expect("revision"),
            GenerationRevision::new(1)
        );
        assert_eq!(serving_generation(&layout), (REAL_GENERATION.to_owned(), 2));
    }

    #[test]
    fn stages_cannot_run_out_of_order() {
        let (_dir, layout) = real_fixture();
        let mut verifier = production(&layout, far_deadline());

        for stage in [StageKind::Seal, StageKind::Publish] {
            let err = verifier
                .verify(stage, far_deadline(), REAL_GENERATION)
                .expect_err("stage without its predecessor must fail closed");
            assert_eq!(err.kind(), VerificationErrorKind::Failed);
        }
        if let Ok(store) = open_generation_store(&layout, false) {
            assert!(
                !store.active_manifest_path().exists(),
                "nothing may be published"
            );
        }
    }

    #[test]
    fn each_stage_fails_typed_on_expired_deadline() {
        let (_dir, layout) = real_fixture();
        let mut verifier = production(&layout, far_deadline());
        let expired = Instant::now();

        let err = verifier
            .verify(StageKind::Build, expired, REAL_GENERATION)
            .expect_err("expired build");
        assert_eq!(err.kind(), VerificationErrorKind::DeadlineExpired);
        assert!(
            !candidate_db(&layout, REAL_GENERATION).exists(),
            "expired Build must not construct a candidate"
        );

        verifier
            .verify(StageKind::Build, far_deadline(), REAL_GENERATION)
            .expect("build");
        let err = verifier
            .verify(StageKind::Seal, expired, REAL_GENERATION)
            .expect_err("expired seal");
        assert_eq!(err.kind(), VerificationErrorKind::DeadlineExpired);

        verifier
            .verify(StageKind::Seal, far_deadline(), REAL_GENERATION)
            .expect("seal");
        let err = verifier
            .verify(StageKind::Publish, expired, REAL_GENERATION)
            .expect_err("expired publish");
        assert_eq!(err.kind(), VerificationErrorKind::DeadlineExpired);
    }

    #[test]
    fn constructor_deadline_also_bounds_every_stage() {
        let (_dir, layout) = real_fixture();
        let mut verifier = production(&layout, Instant::now());

        let err = verifier
            .verify(StageKind::Build, far_deadline(), REAL_GENERATION)
            .expect_err("constructor deadline already expired");
        assert_eq!(err.kind(), VerificationErrorKind::DeadlineExpired);
    }

    #[test]
    fn failed_build_leaves_prior_generation_serving() {
        let (_dir, layout) = real_fixture();
        publish_through_stages(&layout, "prior-gen");
        let before = manifest_bytes(&layout);

        // `runtime` is reserved, so Build must fail closed.
        let mut verifier = production(&layout, far_deadline());
        verifier
            .verify(StageKind::Build, far_deadline(), "runtime")
            .expect_err("reserved id must fail Build");

        assert_eq!(manifest_bytes(&layout), before, "manifest untouched");
        assert_eq!(serving_generation(&layout), ("prior-gen".to_owned(), 2));
    }

    #[test]
    fn failed_publish_leaves_prior_generation_serving() {
        let (_dir, layout) = real_fixture();
        publish_through_stages(&layout, "prior-gen");
        let before = manifest_bytes(&layout);

        let mut verifier = production(&layout, far_deadline());
        verifier
            .verify(StageKind::Build, far_deadline(), "next-gen")
            .expect("build");
        verifier
            .verify(StageKind::Seal, far_deadline(), "next-gen")
            .expect("seal");
        let err = verifier
            .verify(StageKind::Publish, Instant::now(), "next-gen")
            .expect_err("publish past deadline");
        assert_eq!(err.kind(), VerificationErrorKind::DeadlineExpired);

        assert_eq!(manifest_bytes(&layout), before, "manifest untouched");
        let store = open_generation_store(&layout, false).expect("open store");
        assert_eq!(
            current_published_revision(&store).expect("revision"),
            GenerationRevision::new(1)
        );
        assert_eq!(serving_generation(&layout), ("prior-gen".to_owned(), 2));
    }

    #[test]
    fn production_verifier_refuses_to_run_inside_a_runtime() {
        let (_dir, layout) = real_fixture();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");

        let outcome = runtime.block_on(async {
            ProductionVerifier::new(layout.clone(), PathBuf::from("engram"), far_deadline())
        });

        let err = outcome.expect_err("must not nest inside another runtime");
        assert!(err.reason().contains("another runtime"), "{err}");
    }

    #[test]
    fn stages_are_bound_to_the_one_expected_generation() {
        let (_dir, layout) = real_fixture();
        let mut verifier = production(&layout, far_deadline());
        verifier
            .verify(StageKind::Build, far_deadline(), REAL_GENERATION)
            .expect("build");
        verifier
            .verify(StageKind::Seal, far_deadline(), "some-other-gen")
            .expect_err("seal must not accept a different generation");

        let (_dir2, layout2) = real_fixture();
        let mut verifier = production(&layout2, far_deadline());
        verifier
            .verify(StageKind::Build, far_deadline(), REAL_GENERATION)
            .expect("rebuild");
        verifier
            .verify(StageKind::Seal, far_deadline(), REAL_GENERATION)
            .expect("seal");
        verifier
            .verify(StageKind::Publish, far_deadline(), "some-other-gen")
            .expect_err("publish must not accept a different generation");
        if let Ok(store) = open_generation_store(&layout2, false) {
            assert!(
                !store.active_manifest_path().exists(),
                "a mismatched publish must not publish"
            );
        }
    }

    #[test]
    fn verify_inside_a_runtime_fails_typed_and_dropping_is_safe() {
        let (_dir, layout) = real_fixture();
        let mut verifier = production(&layout, far_deadline());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime");

        runtime.block_on(async {
            let err = verifier
                .verify(StageKind::Build, far_deadline(), REAL_GENERATION)
                .expect_err("must not nest a runtime");
            assert_eq!(err.kind(), VerificationErrorKind::Failed);
            assert!(err.reason().contains("another runtime"), "{err}");
            drop(verifier);
        });
        assert!(
            !candidate_db(&layout, REAL_GENERATION).exists(),
            "nothing may be built"
        );
    }

    // -----------------------------------------------------------------------
    // Plan unit F50c (142.054-T): real-chain probe stages. These drive the
    // production verifier with the real engram executable against a real
    // spawned read-server daemon.
    // -----------------------------------------------------------------------

    const READY_TIMEOUT: Duration = Duration::from_secs(120);
    const PROBE_BUDGET: Duration = Duration::from_secs(120);
    const SHORT_BUDGET: Duration = Duration::from_secs(8);
    const PROBE_STAGES: [StageKind; 4] = [
        StageKind::DaemonVerified,
        StageKind::HealthVerified,
        StageKind::CliProbeVerified,
        StageKind::McpProbeVerified,
    ];

    fn engram_exe() -> PathBuf {
        PathBuf::from(env!("CARGO_BIN_EXE_engram"))
    }

    /// Spaced git fixture whose `.engram/config.toml` carries `mode`.
    fn chain_fixture(mode: &str) -> (TempDir, ReadServerLayout) {
        let (dir, layout) = real_fixture();
        let engram_dir = layout.workspace.join(".engram");
        fs::create_dir_all(&engram_dir).expect("create .engram");
        fs::write(
            engram_dir.join("config.toml"),
            format!("log_level = \"debug\"\nmode = \"{mode}\"\n"),
        )
        .expect("write config");
        (dir, layout)
    }

    /// A spawned daemon, killed (exact child) on drop.
    struct Daemon(Child);

    impl Daemon {
        fn spawn(layout: &ReadServerLayout) -> Self {
            let child = Command::new(engram_exe())
                .args(["daemon", "--workspace"])
                .arg(&layout.workspace)
                .env_remove("ENGRAM_DATA_DIR")
                .env_remove("ENGRAM_WORKSPACE")
                .env_remove("ENGRAM_DIRECT")
                .env("ENGRAM_IDLE_TIMEOUT_MS", "0")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn engram daemon");
            let daemon = Self(child);
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            let endpoint = ipc_endpoint(&layout.workspace).expect("ipc endpoint");
            runtime.block_on(async {
                let end = tokio::time::Instant::now() + READY_TIMEOUT;
                while !check_health(&endpoint).await {
                    assert!(
                        tokio::time::Instant::now() < end,
                        "daemon never became ready"
                    );
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            });
            daemon
        }
    }

    impl Drop for Daemon {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Runs the four probe stages in order, stopping at the first failure.
    fn run_probe_stages(
        verifier: &mut ProductionVerifier,
        budget: Duration,
        generation: &str,
    ) -> Result<(), (StageKind, VerificationError)> {
        let deadline = Instant::now() + budget;
        for stage in PROBE_STAGES {
            verifier
                .verify(stage, deadline, generation)
                .map_err(|error| (stage, error))?;
        }
        Ok(())
    }

    /// [054-T AC8; 054.003-ST V1, V3]: the full machine over a spaced
    /// read-server fixture reaches `Succeeded` with the real executable.
    #[test]
    fn real_chain_read_server_fixture_reaches_succeeded() {
        let (_dir, layout) = chain_fixture("read_server");
        let _daemon = Daemon::spawn(&layout);
        let deadline = Instant::now() + PROBE_BUDGET;
        let mut verifier = ProductionVerifier::new(layout.clone(), engram_exe(), deadline)
            .expect("construct production verifier");

        let done = Preflight::<Build>::start(deadline, REAL_GENERATION.to_owned())
            .verify(&mut verifier)
            .and_then(|state| state.verify(&mut verifier))
            .and_then(|state| state.verify(&mut verifier))
            .and_then(|state| state.verify(&mut verifier))
            .and_then(|state| state.verify(&mut verifier))
            .and_then(|state| state.verify(&mut verifier))
            .and_then(|state| state.verify(&mut verifier))
            .expect("all seven stages verify against the real chain");

        assert_eq!(done.expected_generation(), REAL_GENERATION);
        assert_eq!(done.deadline(), deadline);
        let store = open_generation_store(&layout, false).expect("open store");
        assert_eq!(
            current_published_revision(&store).expect("revision"),
            GenerationRevision::new(1),
            "verification is observational: it must not publish again"
        );
    }

    /// [054.003-ST AC5; R48]: the probe stages never mutate the manifest.
    #[test]
    fn probe_stages_do_not_touch_the_manifest() {
        let (_dir, layout) = chain_fixture("read_server");
        publish_through_stages(&layout, REAL_GENERATION);
        let before = manifest_bytes(&layout);
        let _daemon = Daemon::spawn(&layout);
        let mut verifier =
            ProductionVerifier::new(layout.clone(), engram_exe(), Instant::now() + PROBE_BUDGET)
                .expect("construct production verifier");

        run_probe_stages(&mut verifier, PROBE_BUDGET, REAL_GENERATION)
            .unwrap_or_else(|(stage, error)| panic!("{stage:?} must verify: {error}"));

        assert_eq!(manifest_bytes(&layout), before, "manifest untouched");
    }

    /// [054-T AC8]: a managed-mode fixture fails at `DaemonVerified`.
    #[test]
    fn real_chain_managed_mode_fails_at_daemon_verified() {
        let (_dir, layout) = chain_fixture("managed");
        publish_through_stages(&layout, REAL_GENERATION);
        let _daemon = Daemon::spawn(&layout);
        let mut verifier =
            ProductionVerifier::new(layout.clone(), engram_exe(), Instant::now() + PROBE_BUDGET)
                .expect("construct production verifier");

        let (stage, _error) = run_probe_stages(&mut verifier, SHORT_BUDGET, REAL_GENERATION)
            .expect_err("a managed-mode daemon must not verify");

        assert_eq!(stage, StageKind::DaemonVerified);
    }

    /// [054.003-ST V2]: a daemon serving an older revision never reaches
    /// `DaemonVerified` for a newer expected generation.
    #[test]
    fn daemon_serving_an_older_revision_does_not_verify() {
        let (_dir, layout) = chain_fixture("read_server");
        publish_through_stages(&layout, "older-gen");
        let _daemon = Daemon::spawn(&layout);
        let mut verifier =
            ProductionVerifier::new(layout.clone(), engram_exe(), Instant::now() + PROBE_BUDGET)
                .expect("construct production verifier");

        let (stage, _error) = run_probe_stages(&mut verifier, SHORT_BUDGET, "newer-gen")
            .expect_err("an older revision must not satisfy the expected generation");

        assert_eq!(stage, StageKind::DaemonVerified);
    }

    /// Fail closed: every probe stage rejects an expired deadline typed.
    #[test]
    fn probe_stages_fail_typed_on_expired_deadline() {
        let (_dir, layout) = chain_fixture("read_server");
        let mut verifier = production(&layout, far_deadline());

        for stage in PROBE_STAGES {
            let err = verifier
                .verify(stage, Instant::now(), REAL_GENERATION)
                .expect_err("expired deadline");
            assert_eq!(err.kind(), VerificationErrorKind::DeadlineExpired);
        }
    }
}
