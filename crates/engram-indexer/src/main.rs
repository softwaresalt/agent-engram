#![forbid(unsafe_code)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Instant, SystemTime};

use engram::services::generations::candidate_build::{mint_generation_id, read_server_layout};
use engram::services::generations::{GenerationId, GenerationStore};
use engram_indexer::preflight::ProductionVerifier;
use engram_indexer::preflight_invocation::{self, SetupError};

#[derive(Debug)]
struct Invocation {
    generation_root: PathBuf,
    generation_id: GenerationId,
    target_relative_path: PathBuf,
    data_dir: PathBuf,
}

impl Invocation {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let generation_root = PathBuf::from(required_env_os("ENGRAM_INDEXER_GENERATION_ROOT")?);
        let generation_id = GenerationId::new(env::var("ENGRAM_INDEXER_GENERATION_ID")?)?;
        let target_relative_path =
            PathBuf::from(required_env_os("ENGRAM_INDEXER_TARGET_RELATIVE_PATH")?);
        let data_dir = PathBuf::from(required_env_os("ENGRAM_INDEXER_DATA_DIR")?);

        Ok(Self {
            generation_root,
            generation_id,
            target_relative_path,
            data_dir,
        })
    }
}

fn required_env_os(name: &str) -> Result<OsString, io::Error> {
    env::var_os(name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing required environment variable {name}"),
        )
    })
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() == Some(OsStr::new("preflight")) {
        return run_preflight(args);
    }
    run_legacy()
}

/// Runs the preflight supervisor synchronously, before any tokio runtime exists.
fn run_preflight(args: impl Iterator<Item = OsString>) -> ExitCode {
    let output = preflight_invocation::main_with(args, production_factory);
    // A closed pipe must not turn the verdict exit code into a panic, so write errors are ignored.
    if let Some(stdout) = output.stdout {
        let _ = writeln!(io::stdout().lock(), "{stdout}");
    }
    let _ = io::stderr().lock().write_all(output.stderr.as_bytes());
    ExitCode::from(output.exit_code)
}

/// Builds the production verifier and generation id without writing to disk.
///
/// Every failure maps to the typed [`SetupError::Layout`]; neither the workspace path nor the
/// underlying error text is surfaced.
fn production_factory(
    invocation: &preflight_invocation::Invocation,
    deadline: Instant,
) -> Result<(ProductionVerifier, String), SetupError> {
    let layout = read_server_layout(&invocation.workspace).map_err(|_| SetupError::Layout)?;
    let id = mint_generation_id(SystemTime::now()).map_err(|_| SetupError::Layout)?;
    let verifier = ProductionVerifier::new(layout, invocation.engram.clone(), deadline)
        .map_err(|_| SetupError::Layout)?;
    Ok((verifier, id.as_str().to_owned()))
}

fn run_legacy() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("engram-indexer: {error}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(run()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("engram-indexer: {error}");
            ExitCode::FAILURE
        }
    }
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let invocation = Invocation::from_env()?;
    let store = GenerationStore::new(invocation.generation_root)?;
    let target =
        store.seal_legacy_direct(invocation.generation_id, invocation.target_relative_path)?;
    let _index_result = engram_indexer::run_for_target(&target, &invocation.data_dir).await?;
    Ok(())
}
