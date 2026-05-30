#![allow(dead_code)]

use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::config::AppConfig;

mod managed;

pub(crate) use managed::ManagedInstallError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FfmpegPlan {
    pub(crate) requires_ffmpeg: bool,
    pub(crate) requires_ffprobe: bool,
    pub(crate) probe_before_render: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubtitleFilter {
    Ass,
    Subtitles,
}

impl SubtitleFilter {
    pub(crate) fn as_ffmpeg_name(self) -> &'static str {
        match self {
            Self::Ass => "ass",
            Self::Subtitles => "subtitles",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FfmpegToolset {
    pub(crate) ffmpeg: PathBuf,
    pub(crate) ffprobe: PathBuf,
    pub(crate) subtitle_filter: SubtitleFilter,
    pub(crate) origin: ToolOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VideoDimensions {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Error)]
pub(crate) enum FfmpegExecutionError {
    #[error("ffprobe failed to read video dimensions from {video}: {details}")]
    ProbeDimensions { video: PathBuf, details: String },
    #[error("ffprobe returned invalid video dimensions for {video}: {output}")]
    InvalidDimensions { video: PathBuf, output: String },
    #[error("ffmpeg failed to convert subtitle {input} to ASS at {output}: {details}")]
    ConvertSubtitle {
        input: PathBuf,
        output: PathBuf,
        details: String,
    },
    #[error("ffmpeg failed to burn subtitles from {subtitle} into {output}: {details}")]
    BurnSubtitles {
        video: PathBuf,
        subtitle: PathBuf,
        output: PathBuf,
        details: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolOrigin {
    System,
    Managed(ManagedToolOrigin),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedToolOrigin {
    pub(crate) provider: &'static str,
    pub(crate) install_dir: PathBuf,
    pub(crate) platform: RuntimePlatform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimePlatform {
    MacosArm64,
    MacosX64,
    LinuxX64,
    WindowsX64,
    Unsupported { os: String, arch: String },
}

impl RuntimePlatform {
    pub(crate) fn current() -> Self {
        Self::detect(env::consts::OS, env::consts::ARCH)
    }

    fn detect(os: &str, arch: &str) -> Self {
        match (os, arch) {
            ("macos", "aarch64") => Self::MacosArm64,
            ("macos", "x86_64") => Self::MacosX64,
            ("linux", "x86_64") => Self::LinuxX64,
            ("windows", "x86_64") => Self::WindowsX64,
            _ => Self::Unsupported {
                os: os.to_owned(),
                arch: arch.to_owned(),
            },
        }
    }

    fn cache_segment(&self) -> String {
        match self {
            Self::MacosArm64 => "macos-arm64".to_owned(),
            Self::MacosX64 => "macos-x64".to_owned(),
            Self::LinuxX64 => "linux-x64".to_owned(),
            Self::WindowsX64 => "windows-x64".to_owned(),
            Self::Unsupported { os, arch } => format!("{os}-{arch}"),
        }
    }

    fn supports_managed_downloads(&self) -> bool {
        matches!(
            self,
            Self::MacosArm64 | Self::MacosX64 | Self::LinuxX64 | Self::WindowsX64
        )
    }

    fn executable_name(&self, base: &str) -> String {
        if self.is_windows() {
            format!("{base}.exe")
        } else {
            base.to_owned()
        }
    }

    fn is_windows(&self) -> bool {
        matches!(self, Self::WindowsX64)
            || matches!(self, Self::Unsupported { os, .. } if os == "windows")
    }
}

impl fmt::Display for RuntimePlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MacosArm64 => f.write_str("macOS arm64"),
            Self::MacosX64 => f.write_str("macOS x64"),
            Self::LinuxX64 => f.write_str("Linux x64"),
            Self::WindowsX64 => f.write_str("Windows x64"),
            Self::Unsupported { os, arch } => write!(f, "{os}/{arch}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolStatus {
    Missing {
        command: String,
    },
    Unusable {
        command: String,
        path: PathBuf,
        details: String,
    },
    Ready {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubtitleFilterIssue {
    pub(crate) ffmpeg_path: PathBuf,
    pub(crate) available_supported_filters: Vec<SubtitleFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemDiscoveryError {
    pub(crate) ffmpeg_status: ToolStatus,
    pub(crate) ffprobe_status: ToolStatus,
    pub(crate) subtitle_filter_issue: Option<SubtitleFilterIssue>,
}

impl fmt::Display for SystemDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut issues = Vec::new();

        match &self.ffmpeg_status {
            ToolStatus::Missing { command } => {
                issues.push(format!("`{command}` was not found on PATH"))
            }
            ToolStatus::Unusable {
                command,
                path,
                details,
            } => issues.push(format!(
                "`{command}` at {} could not be used: {details}",
                path.display()
            )),
            ToolStatus::Ready { .. } => {}
        }

        match &self.ffprobe_status {
            ToolStatus::Missing { command } => {
                issues.push(format!("`{command}` was not found on PATH"))
            }
            ToolStatus::Unusable {
                command,
                path,
                details,
            } => issues.push(format!(
                "`{command}` at {} could not be used: {details}",
                path.display()
            )),
            ToolStatus::Ready { .. } => {}
        }

        if let Some(issue) = &self.subtitle_filter_issue {
            let supported = if issue.available_supported_filters.is_empty() {
                "none".to_owned()
            } else {
                issue
                    .available_supported_filters
                    .iter()
                    .map(|filter| format!("`{}`", filter.as_ffmpeg_name()))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            issues.push(format!(
                "`ffmpeg` at {} does not expose the required `ass` or `subtitles` filter (supported subtitle filters seen: {supported})",
                issue.ffmpeg_path.display()
            ));
        }

        if issues.is_empty() {
            f.write_str("system ffmpeg discovery failed for an unknown reason")
        } else {
            f.write_str(&issues.join("; "))
        }
    }
}

impl std::error::Error for SystemDiscoveryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManualInstallAdvice {
    message: String,
}

impl ManualInstallAdvice {
    fn for_platform(platform: &RuntimePlatform) -> Self {
        let platform_hint = match platform {
            RuntimePlatform::MacosArm64 | RuntimePlatform::MacosX64 => {
                "On macOS, Homebrew (`brew install ffmpeg`) is usually the quickest manual path."
            }
            RuntimePlatform::LinuxX64 => {
                "On Linux, use your distribution package manager or a vetted static build."
            }
            RuntimePlatform::WindowsX64 => {
                "On Windows, install a ZIP build and add its `bin` directory to PATH."
            }
            RuntimePlatform::Unsupported { .. } => {
                "Managed downloads are only implemented for macOS arm64/x64, Linux x64, and Windows x64."
            }
        };

        Self {
            message: format!(
                "Install both `ffmpeg` and `ffprobe`, ensure they are on PATH, and verify `ffmpeg -hide_banner -filters` lists either `ass` or `subtitles`. {platform_hint} Upstream install guidance: https://ffmpeg.org/download.html"
            ),
        }
    }
}

impl fmt::Display for ManualInstallAdvice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug, Error)]
pub(crate) enum ProvisioningError {
    #[error(
        "usable system ffmpeg/ffprobe were not found for {platform}; system probe failed: {system_error}. {manual_install}"
    )]
    SystemOnlyFailed {
        platform: RuntimePlatform,
        system_error: SystemDiscoveryError,
        manual_install: ManualInstallAdvice,
    },
    #[error(
        "usable system ffmpeg/ffprobe were not found for {platform}; system probe failed: {system_error}; managed install failed: {managed_error}. {manual_install}"
    )]
    ProvisioningFailed {
        platform: RuntimePlatform,
        system_error: SystemDiscoveryError,
        managed_error: ManagedInstallError,
        manual_install: ManualInstallAdvice,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
enum ProbeError {
    #[error("{0}")]
    CommandFailed(String),
    #[error("ffmpeg does not expose the `ass` or `subtitles` filter")]
    MissingSubtitleFilter {
        available_supported_filters: Vec<SubtitleFilter>,
    },
}

pub(crate) fn plan(_config: &AppConfig) -> FfmpegPlan {
    FfmpegPlan {
        requires_ffmpeg: true,
        requires_ffprobe: true,
        probe_before_render: true,
    }
}

pub(crate) fn managed_cache_dir() -> Result<PathBuf, ManagedInstallError> {
    managed::managed_cache_dir(&RuntimePlatform::current())
}

pub(crate) fn locate_system_toolset() -> Result<FfmpegToolset, Box<SystemDiscoveryError>> {
    locate_system_toolset_for_platform(&RuntimePlatform::current())
}

pub(crate) fn provision_managed_toolset() -> Result<FfmpegToolset, ManagedInstallError> {
    managed::install_managed_toolset(&RuntimePlatform::current())
}

pub(crate) fn resolve_toolset() -> Result<FfmpegToolset, Box<ProvisioningError>> {
    let platform = RuntimePlatform::current();

    match locate_system_toolset_for_platform(&platform) {
        Ok(toolset) => Ok(toolset),
        Err(system_error) if platform.supports_managed_downloads() => {
            let manual_install = ManualInstallAdvice::for_platform(&platform);
            match managed::install_managed_toolset(&platform) {
                Ok(toolset) => Ok(toolset),
                Err(managed_error) => Err(Box::new(ProvisioningError::ProvisioningFailed {
                    platform,
                    system_error: *system_error,
                    managed_error,
                    manual_install,
                })),
            }
        }
        Err(system_error) => Err(Box::new(ProvisioningError::SystemOnlyFailed {
            platform: platform.clone(),
            system_error: *system_error,
            manual_install: ManualInstallAdvice::for_platform(&platform),
        })),
    }
}

pub(crate) fn probe_video_dimensions(
    toolset: &FfmpegToolset,
    video: &Path,
) -> Result<VideoDimensions, FfmpegExecutionError> {
    let output = run_command(
        &toolset.ffprobe,
        [
            OsString::from("-v"),
            OsString::from("error"),
            OsString::from("-select_streams"),
            OsString::from("v:0"),
            OsString::from("-show_entries"),
            OsString::from("stream=width,height"),
            OsString::from("-of"),
            OsString::from("csv=s=x:p=0"),
            video.as_os_str().to_os_string(),
        ],
    )
    .map_err(|details| FfmpegExecutionError::ProbeDimensions {
        video: video.to_path_buf(),
        details,
    })?;

    parse_video_dimensions(&output).ok_or_else(|| FfmpegExecutionError::InvalidDimensions {
        video: video.to_path_buf(),
        output: output.trim().to_owned(),
    })
}

pub(crate) fn convert_subtitles_to_ass(
    toolset: &FfmpegToolset,
    input: &Path,
    output: &Path,
) -> Result<(), FfmpegExecutionError> {
    run_command(
        &toolset.ffmpeg,
        [
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-y"),
            OsString::from("-i"),
            input.as_os_str().to_os_string(),
            output.as_os_str().to_os_string(),
        ],
    )
    .map(|_| ())
    .map_err(|details| FfmpegExecutionError::ConvertSubtitle {
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        details,
    })
}

pub(crate) fn embed_subtitles(
    toolset: &FfmpegToolset,
    video: &Path,
    subtitle: &Path,
    font_dir: &Path,
    output: &Path,
) -> Result<(), FfmpegExecutionError> {
    let video_filter = subtitle_filter_expression(toolset, subtitle, font_dir);

    run_command(
        &toolset.ffmpeg,
        [
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-y"),
            OsString::from("-i"),
            video.as_os_str().to_os_string(),
            OsString::from("-vf"),
            OsString::from(video_filter),
            OsString::from("-map"),
            OsString::from("0:v:0"),
            OsString::from("-map"),
            OsString::from("0:a?"),
            OsString::from("-c:v"),
            OsString::from("libx264"),
            OsString::from("-preset"),
            OsString::from("medium"),
            OsString::from("-crf"),
            OsString::from("18"),
            OsString::from("-pix_fmt"),
            OsString::from("yuv420p"),
            OsString::from("-c:a"),
            OsString::from("copy"),
            OsString::from("-movflags"),
            OsString::from("+faststart"),
            output.as_os_str().to_os_string(),
        ],
    )
    .map(|_| ())
    .map_err(|details| FfmpegExecutionError::BurnSubtitles {
        video: video.to_path_buf(),
        subtitle: subtitle.to_path_buf(),
        output: output.to_path_buf(),
        details,
    })
}

fn subtitle_filter_expression(toolset: &FfmpegToolset, subtitle: &Path, font_dir: &Path) -> String {
    format!(
        "{}=filename='{}':fontsdir='{}'",
        toolset.subtitle_filter.as_ffmpeg_name(),
        escape_filter_value(&subtitle.to_string_lossy()),
        escape_filter_value(&font_dir.to_string_lossy())
    )
}

fn escape_filter_value(value: &str) -> String {
    value.replace('\\', r"\\").replace('\'', r"\'")
}

fn locate_system_toolset_for_platform(
    platform: &RuntimePlatform,
) -> Result<FfmpegToolset, Box<SystemDiscoveryError>> {
    let ffmpeg_command = platform.executable_name("ffmpeg");
    let ffprobe_command = platform.executable_name("ffprobe");

    let (ffmpeg_status, subtitle_filter, subtitle_filter_issue) =
        probe_ffmpeg_on_path(&ffmpeg_command);
    let ffprobe_status = probe_tool_on_path(&ffprobe_command, &["-hide_banner", "-version"]);

    let ffmpeg_path = match &ffmpeg_status {
        ToolStatus::Ready { path } => Some(path.clone()),
        _ => None,
    };
    let ffprobe_path = match &ffprobe_status {
        ToolStatus::Ready { path } => Some(path.clone()),
        _ => None,
    };

    match (ffmpeg_path, ffprobe_path, subtitle_filter) {
        (Some(ffmpeg), Some(ffprobe), Some(filter)) => Ok(FfmpegToolset {
            ffmpeg,
            ffprobe,
            subtitle_filter: filter,
            origin: ToolOrigin::System,
        }),
        _ => Err(Box::new(SystemDiscoveryError {
            ffmpeg_status,
            ffprobe_status,
            subtitle_filter_issue,
        })),
    }
}

fn probe_ffmpeg_on_path(
    command: &str,
) -> (
    ToolStatus,
    Option<SubtitleFilter>,
    Option<SubtitleFilterIssue>,
) {
    match locate_program(command) {
        Some(path) => match probe_ffmpeg(&path) {
            Ok(filter) => (ToolStatus::Ready { path }, Some(filter), None),
            Err(ProbeError::CommandFailed(details)) => (
                ToolStatus::Unusable {
                    command: command.to_owned(),
                    path,
                    details,
                },
                None,
                None,
            ),
            Err(ProbeError::MissingSubtitleFilter {
                available_supported_filters,
            }) => {
                let issue = SubtitleFilterIssue {
                    ffmpeg_path: path.clone(),
                    available_supported_filters,
                };
                (ToolStatus::Ready { path }, None, Some(issue))
            }
        },
        None => (
            ToolStatus::Missing {
                command: command.to_owned(),
            },
            None,
            None,
        ),
    }
}

fn probe_tool_on_path(command: &str, args: &[&str]) -> ToolStatus {
    match locate_program(command) {
        Some(path) => match run_command(&path, args) {
            Ok(_) => ToolStatus::Ready { path },
            Err(details) => ToolStatus::Unusable {
                command: command.to_owned(),
                path,
                details,
            },
        },
        None => ToolStatus::Missing {
            command: command.to_owned(),
        },
    }
}

fn probe_ffmpeg(path: &Path) -> Result<SubtitleFilter, ProbeError> {
    let output =
        run_command(path, ["-hide_banner", "-filters"]).map_err(ProbeError::CommandFailed)?;

    match parse_subtitle_filter(&output) {
        Some(filter) => Ok(filter),
        None => Err(ProbeError::MissingSubtitleFilter {
            available_supported_filters: collect_supported_filters(&output),
        }),
    }
}

fn probe_ffprobe(path: &Path) -> Result<(), ProbeError> {
    run_command(path, ["-hide_banner", "-version"])
        .map(|_| ())
        .map_err(ProbeError::CommandFailed)
}

fn locate_program(command: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;

    env::split_paths(&path_var)
        .map(|entry| entry.join(command))
        .find(|candidate| candidate.is_file())
}

fn run_command<I, S>(path: &Path, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(path)
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute: {error}"))?;

    if !output.status.success() {
        let mut details = String::new();
        if let Some(code) = output.status.code() {
            details.push_str(&format!("exit code {code}"));
        } else {
            details.push_str("terminated by signal");
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if !stderr.is_empty() {
            details.push_str(": ");
            details.push_str(&stderr);
        }

        return Err(details);
    }

    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        combined.push('\n');
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    Ok(combined)
}

fn parse_video_dimensions(output: &str) -> Option<VideoDimensions> {
    let line = output.lines().find(|line| !line.trim().is_empty())?.trim();
    let (width, height) = line.split_once('x')?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    Some(VideoDimensions { width, height })
}

fn parse_subtitle_filter(output: &str) -> Option<SubtitleFilter> {
    let supported = collect_supported_filters(output);
    if supported.contains(&SubtitleFilter::Ass) {
        Some(SubtitleFilter::Ass)
    } else if supported.contains(&SubtitleFilter::Subtitles) {
        Some(SubtitleFilter::Subtitles)
    } else {
        None
    }
}

fn collect_supported_filters(output: &str) -> Vec<SubtitleFilter> {
    let mut filters = Vec::new();

    for line in output.lines() {
        let mut fields = line.split_whitespace();
        let _flags = fields.next();
        let Some(name) = fields.next() else {
            continue;
        };

        match name {
            "ass" if !filters.contains(&SubtitleFilter::Ass) => filters.push(SubtitleFilter::Ass),
            "subtitles" if !filters.contains(&SubtitleFilter::Subtitles) => {
                filters.push(SubtitleFilter::Subtitles)
            }
            _ => {}
        }
    }

    filters
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn plan_requires_ffmpeg_and_ffprobe() {
        let config = AppConfig {
            video: PathBuf::from("video.mp4"),
            subtitle: PathBuf::from("subtitle.ass"),
            output: PathBuf::from("output.mp4"),
            overwrite_output: false,
            open_output: false,
            style: crate::config::StyleConfig {
                font_name: "Test".to_owned(),
                font_dir: PathBuf::from("/fonts"),
                font_size: None,
                margin_v: None,
                outline_size: None,
                shadow_size: None,
            },
        };

        let plan = super::plan(&config);

        assert!(plan.requires_ffmpeg);
        assert!(plan.requires_ffprobe);
        assert!(plan.probe_before_render);
    }

    #[test]
    fn parse_subtitle_filter_prefers_ass() {
        let output = " TSC ass              V->V       Render ASS subtitles\n TSC subtitles        V->V       Render text subtitles";

        assert_eq!(parse_subtitle_filter(output), Some(SubtitleFilter::Ass));
    }

    #[test]
    fn parse_subtitle_filter_accepts_subtitles_when_ass_is_missing() {
        let output = " TSC subtitles        V->V       Render text subtitles";

        assert_eq!(
            parse_subtitle_filter(output),
            Some(SubtitleFilter::Subtitles)
        );
    }

    #[test]
    fn parse_subtitle_filter_returns_none_without_supported_filters() {
        let output = " .. scale            V->V       Scale video";

        assert_eq!(parse_subtitle_filter(output), None);
        assert!(collect_supported_filters(output).is_empty());
    }

    #[test]
    fn manual_install_advice_mentions_path_and_required_filters() {
        let advice = ManualInstallAdvice::for_platform(&RuntimePlatform::WindowsX64).to_string();

        assert!(advice.contains("ffmpeg"));
        assert!(advice.contains("ffprobe"));
        assert!(advice.contains("ass` or `subtitles"));
        assert!(advice.contains("PATH"));
    }

    #[test]
    fn system_discovery_error_display_includes_all_issues() {
        let error = SystemDiscoveryError {
            ffmpeg_status: ToolStatus::Missing {
                command: "ffmpeg".to_owned(),
            },
            ffprobe_status: ToolStatus::Unusable {
                command: "ffprobe".to_owned(),
                path: PathBuf::from("/usr/local/bin/ffprobe"),
                details: "exit code 1".to_owned(),
            },
            subtitle_filter_issue: Some(SubtitleFilterIssue {
                ffmpeg_path: PathBuf::from("/usr/local/bin/ffmpeg"),
                available_supported_filters: Vec::new(),
            }),
        };

        let display = error.to_string();
        assert!(display.contains("`ffmpeg` was not found"));
        assert!(display.contains("`ffprobe` at /usr/local/bin/ffprobe could not be used"));
        assert!(display.contains("does not expose the required `ass` or `subtitles` filter"));
    }

    #[test]
    fn runtime_platform_detect_maps_supported_matrix() {
        assert_eq!(
            RuntimePlatform::detect("macos", "aarch64"),
            RuntimePlatform::MacosArm64
        );
        assert_eq!(
            RuntimePlatform::detect("macos", "x86_64"),
            RuntimePlatform::MacosX64
        );
        assert_eq!(
            RuntimePlatform::detect("linux", "x86_64"),
            RuntimePlatform::LinuxX64
        );
        assert_eq!(
            RuntimePlatform::detect("windows", "x86_64"),
            RuntimePlatform::WindowsX64
        );
        assert_eq!(
            RuntimePlatform::detect("linux", "aarch64"),
            RuntimePlatform::Unsupported {
                os: "linux".to_owned(),
                arch: "aarch64".to_owned(),
            }
        );
    }

    #[test]
    fn managed_release_selection_is_explicit_about_platform_sources() {
        assert!(
            managed::managed_release(&RuntimePlatform::MacosArm64)
                .expect("macOS arm64 should be supported")
                .provider
                .contains("osxexperts")
        );
        assert!(
            managed::managed_release(&RuntimePlatform::MacosX64)
                .expect("macOS x64 should be supported")
                .provider
                .contains("evermeet")
        );
        assert!(
            managed::managed_release(&RuntimePlatform::LinuxX64)
                .expect("linux x64 should be supported")
                .provider
                .contains("johnvansickle")
        );
        assert!(
            managed::managed_release(&RuntimePlatform::WindowsX64)
                .expect("windows x64 should be supported")
                .provider
                .contains("gyan.dev")
        );
    }

    #[test]
    fn managed_cache_dir_prefers_platform_conventions() {
        let linux_cache = managed::cache_base_dir_from_env(&RuntimePlatform::LinuxX64, |key| {
            BTreeMap::from([
                ("XDG_CACHE_HOME".to_owned(), "/cache".to_owned()),
                ("HOME".to_owned(), "/home/will".to_owned()),
            ])
            .get(key)
            .cloned()
            .map(PathBuf::from)
        })
        .expect("linux cache dir should resolve");
        assert_eq!(linux_cache, PathBuf::from("/cache"));

        let mac_cache = managed::cache_base_dir_from_env(&RuntimePlatform::MacosArm64, |key| {
            BTreeMap::from([("HOME".to_owned(), "/Users/will".to_owned())])
                .get(key)
                .cloned()
                .map(PathBuf::from)
        })
        .expect("macOS cache dir should resolve");
        assert_eq!(mac_cache, PathBuf::from("/Users/will/Library/Caches"));
    }

    #[test]
    fn parse_video_dimensions_reads_first_valid_line() {
        assert_eq!(
            parse_video_dimensions("\n1920x1080\n"),
            Some(VideoDimensions {
                width: 1920,
                height: 1080
            })
        );
        assert_eq!(parse_video_dimensions("not-a-dimension"), None);
    }

    #[test]
    fn subtitle_filter_expression_quotes_paths_for_ffmpeg() {
        let toolset = FfmpegToolset {
            ffmpeg: PathBuf::from("/usr/local/bin/ffmpeg"),
            ffprobe: PathBuf::from("/usr/local/bin/ffprobe"),
            subtitle_filter: SubtitleFilter::Ass,
            origin: ToolOrigin::System,
        };

        let filter = subtitle_filter_expression(
            &toolset,
            Path::new("/Users/will/it's.ass"),
            Path::new("/Users/will/Font Folder"),
        );

        assert_eq!(
            filter,
            "ass=filename='/Users/will/it\\'s.ass':fontsdir='/Users/will/Font Folder'"
        );
    }
}
