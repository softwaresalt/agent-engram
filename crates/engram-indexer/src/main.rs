#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use engram::services::generations::{GenerationId, GenerationStore};

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
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
