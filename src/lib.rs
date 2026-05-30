mod cli;
mod config;
mod ffmpeg;
mod pipeline;
mod platform;
mod subtitle;

use anyhow::Result;
use clap::Parser;

pub fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    run_with_cli(cli)
}

fn run_with_cli(cli: cli::Cli) -> Result<()> {
    if cli.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config = config::AppConfig::from_env_and_cli(&cli)?;
    pipeline::execute(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_with_cli_surfaces_validation_errors_from_pipeline() {
        let cli =
            crate::cli::Cli::parse_from(["burn-in-zh-subtitles", "input.mp4", "subtitles.vtt"]);

        let error = run_with_cli(cli).expect_err("missing video should fail validation");
        assert!(
            error
                .to_string()
                .contains("video input does not exist or is not a file")
        );
    }
}
