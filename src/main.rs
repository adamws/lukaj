use lukaj::{app, SvgBackend};

use clap::{ArgAction, Parser, ValueEnum};
use log::debug;
use std::env;
use std::path::PathBuf;

#[cfg(not(any(feature = "use-rsvg", feature = "use-usvg")))]
compile_error!("Either feature \"use-rsvg\" or \"use-usvg\" must be enabled for this crate.");

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Files to compare
    #[arg(required = true, num_args = 2..=2, action = ArgAction::Append)]
    file: Vec<PathBuf>,

    /// Sets a scaling factor
    #[arg(short, long, value_name = "VALUE")]
    scale: Option<f64>,

    /// Preferred backend
    #[arg(long, value_enum, default_value_t=SvgBackend::value_variants()[0])]
    backend: SvgBackend,
}

fn main() -> Result<(), String> {
    env_logger::init();
    let test_tmpdir = env::var("CARGO_TARGET_TMPDIR");
    if test_tmpdir.is_ok() {
        debug!(
            "Running in test mode with CARGO_TARGET_TMPDIR: {:?}",
            test_tmpdir
        );
    }

    let cli = Cli::parse();

    let scale = cli.scale.unwrap_or(1.0);
    let backend = cli.backend;

    let left = cli.file[0].to_owned();
    let right = cli.file[1].to_owned();

    app(left, right, scale, backend, test_tmpdir.ok())?;

    Ok(())
}
