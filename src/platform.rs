use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenPlan {
    pub(crate) should_open_output: bool,
    pub(crate) open_command_hint: &'static str,
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub(crate) enum OpenError {
    #[error("failed to launch `{command}` for {path}: {source}")]
    Launch {
        command: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("`{command}` failed while opening {path}: {details}")]
    Failed {
        command: &'static str,
        path: PathBuf,
        details: String,
    },
}

pub(crate) fn plan(should_open_output: bool) -> OpenPlan {
    OpenPlan {
        should_open_output,
        open_command_hint: if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(target_os = "windows") {
            "start"
        } else {
            "xdg-open"
        },
    }
}

pub(crate) fn open_output(plan: &OpenPlan, path: &Path) -> Result<(), OpenError> {
    if !plan.should_open_output {
        return Ok(());
    }

    #[cfg(test)]
    {
        let _ = plan;
        let _ = path;
        return Ok(());
    }

    #[cfg(not(test))]
    {
        let mut command = if cfg!(target_os = "windows") {
            let mut command = Command::new("cmd");
            command.arg("/C").arg("start").arg("").arg(path);
            command
        } else {
            let mut command = Command::new(plan.open_command_hint);
            command.arg(path);
            command
        };

        let output = command.output().map_err(|source| OpenError::Launch {
            command: plan.open_command_hint,
            path: path.to_path_buf(),
            source,
        })?;

        if output.status.success() {
            return Ok(());
        }

        let mut details = match output.status.code() {
            Some(code) => format!("exit code {code}"),
            None => "terminated by signal".to_owned(),
        };

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if !stderr.is_empty() {
            details.push_str(": ");
            details.push_str(&stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !stdout.is_empty() {
            if !stderr.is_empty() {
                details.push_str(" | ");
            } else {
                details.push_str(": ");
            }
            details.push_str(&stdout);
        }

        Err(OpenError::Failed {
            command: plan.open_command_hint,
            path: path.to_path_buf(),
            details,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_uses_platform_open_command() {
        let plan = super::plan(true);

        if cfg!(target_os = "macos") {
            assert_eq!(plan.open_command_hint, "open");
        } else if cfg!(target_os = "windows") {
            assert_eq!(plan.open_command_hint, "start");
        } else {
            assert_eq!(plan.open_command_hint, "xdg-open");
        }
        assert!(plan.should_open_output);
    }

    #[test]
    fn disabled_open_plan_is_a_no_op() {
        let plan = super::plan(false);

        assert!(!plan.should_open_output);
        assert!(super::open_output(&plan, Path::new("ignored.mp4")).is_ok());
    }
}
