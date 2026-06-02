use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};

use crate::{config::AppConfig, ffmpeg, platform, subtitle};

static WORK_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PipelineContext {
    pub(crate) config: AppConfig,
    pub(crate) ffmpeg: ffmpeg::FfmpegPlan,
    pub(crate) subtitle: subtitle::SubtitlePlan,
    pub(crate) platform: platform::OpenPlan,
}

pub(crate) fn execute(config: AppConfig) -> Result<()> {
    if config.open_output && !config.overwrite_output && config.output.exists() {
        let absolute_output = std::fs::canonicalize(&config.output).unwrap_or_else(|_| {
            if config.output.is_absolute() {
                config.output.clone()
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(&config.output)
            } else {
                config.output.clone()
            }
        });
        println!("{}", absolute_output.display());

        let platform_plan = platform::plan(config.open_output);
        platform::open_output(&platform_plan, &config.output)?;
        return Ok(());
    }

    let context = PipelineContext {
        ffmpeg: ffmpeg::plan(&config),
        subtitle: subtitle::plan(&config),
        platform: platform::plan(config.open_output),
        config,
    };

    validate_inputs(&context.config)?;

    let toolset =
        ffmpeg::resolve_toolset().context("failed to locate or provision ffmpeg/ffprobe")?;

    let dimensions = if context.ffmpeg.probe_before_render {
        ffmpeg::probe_video_dimensions(&toolset, &context.config.video)?
    } else {
        unreachable!("burn pipeline requires probing video dimensions before rendering")
    };

    let output_dir = output_dir(&context.config.output);
    let work_dir = WorkDir::create(output_dir)?;
    let raw_ass_path = work_dir.path().join("raw.ass");
    let wrapped_ass_path = work_dir.path().join("wrapped.ass");
    let staged_output_path = work_dir.path().join("burned-output.mp4");

    if context.subtitle.requires_ass_staging {
        ffmpeg::convert_subtitles_to_ass(&toolset, &context.config.subtitle, &raw_ass_path)?;
    } else {
        fs::copy(&context.config.subtitle, &raw_ass_path).with_context(|| {
            format!(
                "failed to copy ASS subtitle file from {} to {}",
                context.config.subtitle.display(),
                raw_ass_path.display()
            )
        })?;
    }

    let raw_ass = fs::read_to_string(&raw_ass_path).with_context(|| {
        format!(
            "failed to read converted ASS subtitles from {}",
            raw_ass_path.display()
        )
    })?;

    let rendered_ass = if context.subtitle.wraps_dialogue_lines {
        subtitle::render_ass(
            &raw_ass,
            &context.config.style,
            dimensions.width,
            dimensions.height,
        )?
    } else {
        raw_ass
    };

    fs::write(&wrapped_ass_path, rendered_ass).with_context(|| {
        format!(
            "failed to write prepared ASS subtitles to {}",
            wrapped_ass_path.display()
        )
    })?;

    ffmpeg::embed_subtitles(
        &toolset,
        &context.config.video,
        &wrapped_ass_path,
        &context.config.style.font_dir,
        &staged_output_path,
    )?;

    finalize_output(
        &staged_output_path,
        &context.config.output,
        context.config.overwrite_output,
    )?;

    let absolute_output = std::fs::canonicalize(&context.config.output).unwrap_or_else(|_| {
        if context.config.output.is_absolute() {
            context.config.output.clone()
        } else if let Ok(cwd) = std::env::current_dir() {
            cwd.join(&context.config.output)
        } else {
            context.config.output.clone()
        }
    });
    println!("{}", absolute_output.display());

    platform::open_output(&context.platform, &context.config.output)?;
    Ok(())
}

fn validate_inputs(config: &AppConfig) -> Result<()> {
    ensure!(
        config.video.is_file(),
        "video input does not exist or is not a file: {}",
        config.video.display()
    );
    ensure!(
        config.subtitle.is_file(),
        "subtitle input does not exist or is not a file: {}",
        config.subtitle.display()
    );
    ensure!(
        config.style.font_dir.is_dir(),
        "font directory does not exist or is not a directory: {}",
        config.style.font_dir.display()
    );

    let output_dir = output_dir(&config.output);
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create output directory {}", output_dir.display()))?;

    ensure!(
        !config.output.exists() || config.overwrite_output,
        "output file already exists: {} (pass --force to overwrite)",
        config.output.display()
    );

    Ok(())
}

fn output_dir(output: &Path) -> &Path {
    output.parent().unwrap_or_else(|| Path::new("."))
}

fn finalize_output(staged_output: &Path, final_output: &Path, overwrite: bool) -> Result<()> {
    ensure!(
        staged_output.is_file(),
        "ffmpeg did not produce the expected output file: {}",
        staged_output.display()
    );

    if overwrite && final_output.exists() {
        fs::remove_file(final_output).with_context(|| {
            format!(
                "failed to remove existing output before overwrite: {}",
                final_output.display()
            )
        })?;
    }

    fs::rename(staged_output, final_output).with_context(|| {
        format!(
            "failed to move rendered video into place: {} -> {}",
            staged_output.display(),
            final_output.display()
        )
    })?;

    Ok(())
}

#[derive(Debug)]
struct WorkDir {
    path: PathBuf,
}

impl WorkDir {
    fn create(parent: &Path) -> Result<Self> {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create working directory parent {}",
                parent.display()
            )
        })?;

        for attempt in 0..16u8 {
            let suffix = WORK_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = parent.join(format!(
                ".subembed-work.{}.{}.{}.{}",
                std::process::id(),
                timestamp,
                suffix,
                attempt
            ));

            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to create working directory {}", path.display())
                    });
                }
            }
        }

        anyhow::bail!(
            "failed to create a unique working directory inside {}",
            parent.display()
        )
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::config::StyleConfig;

    fn style_config(font_dir: PathBuf) -> StyleConfig {
        StyleConfig {
            font_name: "Test Font".to_owned(),
            font_dir,
            font_size: None,
            margin_v: None,
            outline_size: None,
            shadow_size: None,
        }
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("test-artifacts");
            fs::create_dir_all(&base).expect("test artifact base should be created");

            let unique = format!(
                "{}-{}-{}",
                label,
                std::process::id(),
                WORK_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = base.join(unique);
            fs::create_dir_all(&path).expect("test artifact dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn execute_rejects_missing_video_before_invoking_ffmpeg() {
        let test_dir = TestDir::new("pipeline-missing-video");
        let subtitle_path = test_dir.path().join("subtitle.vtt");
        fs::write(&subtitle_path, "WEBVTT\n\n00:00.000 --> 00:01.000\n測試\n")
            .expect("subtitle fixture should be written");

        let error = execute(AppConfig {
            video: test_dir.path().join("missing.mp4"),
            subtitle: subtitle_path,
            output: test_dir.path().join("output.mp4"),
            overwrite_output: false,
            open_output: false,
            style: style_config(test_dir.path().to_path_buf()),
        })
        .expect_err("missing video should fail");

        assert!(
            error
                .to_string()
                .contains("video input does not exist or is not a file")
        );
    }

    #[test]
    fn execute_rejects_existing_output_without_force() {
        let test_dir = TestDir::new("pipeline-existing-output");
        let video_path = test_dir.path().join("video.mp4");
        let subtitle_path = test_dir.path().join("subtitle.vtt");
        let output_path = test_dir.path().join("output.mp4");
        fs::write(&video_path, "video").expect("video fixture should be written");
        fs::write(&subtitle_path, "WEBVTT\n").expect("subtitle fixture should be written");
        fs::write(&output_path, "existing").expect("output fixture should be written");

        let error = execute(AppConfig {
            video: video_path,
            subtitle: subtitle_path,
            output: output_path.clone(),
            overwrite_output: false,
            open_output: false,
            style: style_config(test_dir.path().to_path_buf()),
        })
        .expect_err("existing output should fail");

        assert!(error.to_string().contains(&format!(
            "output file already exists: {}",
            output_path.display()
        )));
    }

    #[test]
    fn execute_opens_existing_output_without_force_when_open_enabled() {
        let test_dir = TestDir::new("pipeline-open-existing");
        let output_path = test_dir.path().join("output.mp4");
        fs::write(&output_path, "existing video contents").expect("output fixture should be written");

        // When open_output is true and overwrite_output is false, and output exists,
        // it should succeed and just open the existing output file, even if video/subtitle don't exist.
        execute(AppConfig {
            video: test_dir.path().join("nonexistent_video.mp4"),
            subtitle: test_dir.path().join("nonexistent_subtitle.vtt"),
            output: output_path,
            overwrite_output: false,
            open_output: true,
            style: style_config(test_dir.path().to_path_buf()),
        })
        .expect("should succeed by opening the existing output instead of validating inputs or failing");
    }
}
