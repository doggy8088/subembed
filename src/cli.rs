use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueHint};

const LONG_ABOUT: &str = "\
subembed burns Traditional Chinese subtitles into a video with a staged ffmpeg-based workflow.

Workflow:
  1. Probe the source video with ffprobe
  2. Convert source subtitles into ASS when needed
  3. Apply ASS styling from environment-driven config
  4. Wrap dialogue lines for readability
  5. Burn subtitles into the output video with ffmpeg
  6. Optionally open the rendered output on the host platform

Tool resolution:
  - Prefer system ffmpeg/ffprobe found on PATH
  - Otherwise try a managed download on supported platforms
  - If provisioning still fails, install ffmpeg/ffprobe manually and ensure
    `ffmpeg -hide_banner -filters` lists `ass` or `subtitles`";

const AFTER_HELP: &str = "\
Examples:
  subembed movie.mp4 movie.zh-tw.vtt
  subembed movie.mp4 movie.zh-tw.ass
  subembed movie.mp4 movie.zh-tw.srt custom-output.mp4
  subembed --force --open movie.mp4 movie.zh-tw.vtt
  subembed -v

Inputs and output:
  - VIDEO and SUBTITLE are required unless -v/--version is used.
  - OUTPUT defaults to <video-stem>.zh-tw.burned.mp4 in the same directory.
  - Use --force to overwrite an existing output file.

Styling environment:
  FONT_NAME      Subtitle font family (default: LINE Seed TW_OTF Regular)
  FONT_DIR       Directory to search for fonts (default: /Library/Fonts)
  FONT_SIZE      Override computed subtitle font size in ASS points
  MARGIN_V       Override bottom subtitle margin in ASS pixels
  OUTLINE_SIZE   Override outline thickness in ASS pixels
  SHADOW_SIZE    Override shadow thickness in ASS pixels
  - When omitted, size-related values are derived from the input video height.

FFmpeg provisioning:
  - System ffmpeg/ffprobe on PATH are used first.
  - Managed downloads are attempted on macOS arm64/x64, Linux x64, and Windows x64.
  - Managed tools are cached in the platform cache directory for reuse.
  - If both system detection and managed download fail, install ffmpeg and ffprobe
    manually and confirm `ffmpeg -hide_banner -filters` exposes `ass` or `subtitles`.

Notes:
  - The subembed CLI always requires explicit input files for normal burns.
  - Subtitle inputs are staged into ASS before rendering; common inputs include
    .ass, .srt, and .vtt, plus other formats ffmpeg can decode.
  - --open uses the platform default opener (`open`, `xdg-open`, or `start`).";

#[derive(Debug, Clone, Parser)]
#[command(
    name = "subembed",
    version,
    disable_version_flag = true,
    about = "Burn Traditional Chinese subtitles into a video.",
    long_about = LONG_ABOUT,
    after_help = AFTER_HELP,
    override_usage = "subembed [OPTIONS] <VIDEO> <SUBTITLE> [OUTPUT]\n       subembed -v|--version"
)]
pub(crate) struct Cli {
    #[arg(
        short = 'v',
        long = "version",
        action = ArgAction::SetTrue,
        exclusive = true,
        help = "Print version information and exit"
    )]
    pub(crate) version: bool,

    #[arg(
        short = 'f',
        long = "force",
        help = "Overwrite the output file if it already exists"
    )]
    pub(crate) force: bool,

    #[arg(
        short = 'o',
        long = "open",
        help = "Open the rendered output with the platform default app after success"
    )]
    pub(crate) open: bool,

    #[arg(
        value_name = "VIDEO",
        value_hint = ValueHint::FilePath,
        required_unless_present = "version",
        help = "Source video file to burn subtitles into"
    )]
    pub(crate) video: Option<PathBuf>,

    #[arg(
        value_name = "SUBTITLE",
        value_hint = ValueHint::FilePath,
        required_unless_present = "version",
        help = "Subtitle file to convert and burn (for example .ass, .srt, .vtt)"
    )]
    pub(crate) subtitle: Option<PathBuf>,

    #[arg(
        value_name = "OUTPUT",
        value_hint = ValueHint::FilePath,
        help = "Optional output path; defaults to <video-stem>.zh-tw.burned.mp4"
    )]
    pub(crate) output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn parses_required_positionals_and_flags() {
        let cli = Cli::parse_from([
            "subembed",
            "--force",
            "--open",
            "movie.mp4",
            "movie.vtt",
            "out.mp4",
        ]);

        assert!(cli.force);
        assert!(cli.open);
        assert_eq!(cli.video, Some(PathBuf::from("movie.mp4")));
        assert_eq!(cli.subtitle, Some(PathBuf::from("movie.vtt")));
        assert_eq!(cli.output, Some(PathBuf::from("out.mp4")));
    }

    #[test]
    fn rejects_missing_required_inputs() {
        let error = Cli::try_parse_from(["subembed"])
            .expect_err("CLI should require explicit video and subtitle files");

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn help_mentions_environment_and_default_output() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains("FONT_NAME"));
        assert!(help.contains("<video-stem>.zh-tw.burned.mp4"));
        assert!(help.contains("requires explicit input files"));
        assert!(help.contains("<VIDEO> <SUBTITLE> [OUTPUT]"));
        assert!(help.contains("managed download"));
        assert!(help.contains("-v, --version"));
        assert!(!help.contains("not-implemented"));
    }

    #[test]
    fn parses_short_version_without_inputs() {
        let cli = Cli::parse_from(["subembed", "-v"]);

        assert!(cli.version);
        assert_eq!(cli.video, None);
        assert_eq!(cli.subtitle, None);
    }

    #[test]
    fn parses_long_version_without_inputs() {
        let cli = Cli::parse_from(["subembed", "--version"]);

        assert!(cli.version);
        assert_eq!(cli.video, None);
        assert_eq!(cli.subtitle, None);
    }
}
