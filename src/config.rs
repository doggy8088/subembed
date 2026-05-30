use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

use crate::cli::Cli;

const FONT_NAME_KEY: &str = "FONT_NAME";
const FONT_DIR_KEY: &str = "FONT_DIR";
const FONT_SIZE_KEY: &str = "FONT_SIZE";
const MARGIN_V_KEY: &str = "MARGIN_V";
const OUTLINE_SIZE_KEY: &str = "OUTLINE_SIZE";
const SHADOW_SIZE_KEY: &str = "SHADOW_SIZE";

const DEFAULT_FONT_NAME: &str = "LINE Seed TW_OTF Regular";
const DEFAULT_FONT_DIR: &str = "/Library/Fonts";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppConfig {
    pub(crate) video: PathBuf,
    pub(crate) subtitle: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) overwrite_output: bool,
    pub(crate) open_output: bool,
    pub(crate) style: StyleConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StyleConfig {
    pub(crate) font_name: String,
    pub(crate) font_dir: PathBuf,
    pub(crate) font_size: Option<u32>,
    pub(crate) margin_v: Option<u32>,
    pub(crate) outline_size: Option<u32>,
    pub(crate) shadow_size: Option<u32>,
}

impl AppConfig {
    pub(crate) fn from_env_and_cli(cli: &Cli) -> Result<Self> {
        let env = capture_env_vars()?;
        Self::from_env_map(cli, &env)
    }

    fn from_env_map(cli: &Cli, env: &BTreeMap<String, String>) -> Result<Self> {
        let video = cli
            .video
            .clone()
            .context("video input is required unless --version is used")?;
        let subtitle = cli
            .subtitle
            .clone()
            .context("subtitle input is required unless --version is used")?;

        Ok(Self {
            video: video.clone(),
            subtitle,
            output: cli
                .output
                .clone()
                .unwrap_or_else(|| default_output_path(&video)),
            overwrite_output: cli.force,
            open_output: cli.open,
            style: StyleConfig::from_env_map(env)?,
        })
    }
}

impl StyleConfig {
    fn from_env_map(env: &BTreeMap<String, String>) -> Result<Self> {
        Ok(Self {
            font_name: env_string(env, FONT_NAME_KEY)
                .unwrap_or_else(|| DEFAULT_FONT_NAME.to_owned()),
            font_dir: PathBuf::from(
                env_string(env, FONT_DIR_KEY).unwrap_or_else(|| DEFAULT_FONT_DIR.to_owned()),
            ),
            font_size: env_u32(env, FONT_SIZE_KEY)?,
            margin_v: env_u32(env, MARGIN_V_KEY)?,
            outline_size: env_u32(env, OUTLINE_SIZE_KEY)?,
            shadow_size: env_u32(env, SHADOW_SIZE_KEY)?,
        })
    }
}

fn capture_env_vars() -> Result<BTreeMap<String, String>> {
    [
        FONT_NAME_KEY,
        FONT_DIR_KEY,
        FONT_SIZE_KEY,
        MARGIN_V_KEY,
        OUTLINE_SIZE_KEY,
        SHADOW_SIZE_KEY,
    ]
    .into_iter()
    .filter_map(|key| match env::var(key) {
        Ok(value) => Some(Ok((key.to_owned(), value))),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => Some(Err(anyhow!(
            "environment variable {key} contains non-Unicode data"
        ))),
    })
    .collect()
}

fn env_string(env: &BTreeMap<String, String>, key: &str) -> Option<String> {
    env.get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
}

fn env_u32(env: &BTreeMap<String, String>, key: &str) -> Result<Option<u32>> {
    match env_string(env, key) {
        Some(raw) => raw
            .parse::<u32>()
            .with_context(|| format!("{key} must be an unsigned integer"))
            .map(Some),
        None => Ok(None),
    }
}

fn default_output_path(video: &Path) -> PathBuf {
    let video = video.to_string_lossy();
    let stem = match video.rfind('.') {
        Some(index) => &video[..index],
        None => video.as_ref(),
    };

    PathBuf::from(format!("{stem}.zh-tw.burned.mp4"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    #[test]
    fn derives_default_output_from_video_path() {
        let cli = Cli::parse_from(["burn-in-zh-subtitles", "clips/demo.mp4", "subs/demo.vtt"]);
        let env = BTreeMap::new();

        let config = AppConfig::from_env_map(&cli, &env).expect("config should build");

        assert_eq!(config.output, PathBuf::from("clips/demo.zh-tw.burned.mp4"));
        assert!(!config.overwrite_output);
        assert!(!config.open_output);
    }

    #[test]
    fn uses_style_overrides_from_environment() {
        let cli = Cli::parse_from(["burn-in-zh-subtitles", "demo.mp4", "demo.vtt"]);
        let env = BTreeMap::from([
            (FONT_NAME_KEY.to_owned(), "Noto Sans CJK TC".to_owned()),
            (FONT_DIR_KEY.to_owned(), "/custom/fonts".to_owned()),
            (FONT_SIZE_KEY.to_owned(), "64".to_owned()),
            (MARGIN_V_KEY.to_owned(), "48".to_owned()),
            (OUTLINE_SIZE_KEY.to_owned(), "5".to_owned()),
            (SHADOW_SIZE_KEY.to_owned(), "2".to_owned()),
        ]);

        let config = AppConfig::from_env_map(&cli, &env).expect("config should build");

        assert_eq!(config.style.font_name, "Noto Sans CJK TC");
        assert_eq!(config.style.font_dir, PathBuf::from("/custom/fonts"));
        assert_eq!(config.style.font_size, Some(64));
        assert_eq!(config.style.margin_v, Some(48));
        assert_eq!(config.style.outline_size, Some(5));
        assert_eq!(config.style.shadow_size, Some(2));
    }

    #[test]
    fn rejects_invalid_numeric_overrides() {
        let cli = Cli::parse_from(["burn-in-zh-subtitles", "demo.mp4", "demo.vtt"]);
        let env = BTreeMap::from([(FONT_SIZE_KEY.to_owned(), "big".to_owned())]);

        let error = AppConfig::from_env_map(&cli, &env).expect_err("FONT_SIZE should be numeric");

        assert!(error.to_string().contains("FONT_SIZE"));
    }

    #[test]
    fn empty_string_overrides_fall_back_to_defaults() {
        let cli = Cli::parse_from(["burn-in-zh-subtitles", "demo.mp4", "demo.vtt"]);
        let env = BTreeMap::from([
            (FONT_NAME_KEY.to_owned(), "   ".to_owned()),
            (FONT_DIR_KEY.to_owned(), "".to_owned()),
        ]);

        let config = AppConfig::from_env_map(&cli, &env).expect("config should build");

        assert_eq!(config.style.font_name, DEFAULT_FONT_NAME);
        assert_eq!(config.style.font_dir, PathBuf::from(DEFAULT_FONT_DIR));
    }
}
