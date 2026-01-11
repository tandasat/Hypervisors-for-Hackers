//! A build and test assist program. To show the usage, run
//!
//! ```shell
//! cargo xtask
//! ```

mod config;
mod vmware;

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Build the driver with a specified profile.
    #[arg(short, long)]
    release: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a VMware VM
    Vmware,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Vmware => vmware::run(Profile::from(cli.release)),
    }
}

/// Returns the workspace root directory path.
fn workspace_root_dir() -> PathBuf {
    // Get the path to the xtask directory and resolve its parent directory.
    let root_dir = Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf();
    fs::canonicalize(root_dir).unwrap()
}

#[derive(Copy, Clone, Debug)]
enum Profile {
    Dev,
    Release,
}

impl From<bool> for Profile {
    fn from(release: bool) -> Self {
        if release { Self::Release } else { Self::Dev }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Profile::Dev => write!(f, "debug"),
            Profile::Release => write!(f, "release"),
        }
    }
}
