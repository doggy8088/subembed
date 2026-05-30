#![allow(dead_code)]

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use tar::Archive;
use thiserror::Error;
use xz2::read::XzDecoder;
use zip::ZipArchive;

use super::{
    FfmpegToolset, ManagedToolOrigin, RuntimePlatform, ToolOrigin, probe_ffmpeg, probe_ffprobe,
};

// Managed download assumptions encoded here:
// - macOS x64 uses Evermeet release ZIP downloads.
// - macOS arm64 uses osxexperts.net FFmpeg 6 ZIP downloads.
// - Linux x64 uses johnvansickle.com static tar.xz builds.
// - Windows x64 uses gyan.dev release-essentials ZIP builds.
// These sources are intentionally explicit so future updates can revise them in one place.

#[derive(Debug, Error)]
pub(crate) enum ManagedInstallError {
    #[error("managed downloads are not implemented for {platform}")]
    UnsupportedPlatform { platform: RuntimePlatform },
    #[error("could not determine a cache directory for {platform}: {details}")]
    CacheDirectoryUnavailable {
        platform: RuntimePlatform,
        details: String,
    },
    #[error("could not create cache directory {path}: {details}")]
    CacheDirectoryCreateFailed { path: PathBuf, details: String },
    #[error("failed to remove stale install directory {path}: {details}")]
    StaleInstallCleanupFailed { path: PathBuf, details: String },
    #[error("failed to download {url} from {provider}: {details}")]
    DownloadFailed {
        provider: &'static str,
        url: &'static str,
        details: String,
    },
    #[error("failed to extract {archive_name} from {provider}: {details}")]
    ExtractionFailed {
        provider: &'static str,
        archive_name: &'static str,
        details: String,
    },
    #[error("{archive_name} from {provider} did not contain {member}")]
    ArchiveMemberMissing {
        provider: &'static str,
        archive_name: &'static str,
        member: &'static str,
    },
    #[error("failed to write managed install metadata into {path}: {details}")]
    MetadataWriteFailed { path: PathBuf, details: String },
    #[error("failed to finalize managed install in {install_dir}: {details}")]
    InstallFinalizeFailed {
        install_dir: PathBuf,
        details: String,
    },
    #[error("managed install at {install_dir} is invalid: {details}")]
    InstalledToolsInvalid {
        install_dir: PathBuf,
        details: String,
    },
}

#[derive(Debug)]
pub(super) struct ManagedRelease {
    pub(super) provider: &'static str,
    install_id: &'static str,
    assumption: &'static str,
    archives: &'static [ArchiveSpec],
}

#[derive(Debug)]
struct ArchiveSpec {
    archive_name: &'static str,
    url: &'static str,
    kind: ArchiveKind,
    members: &'static [ArchiveMember],
}

#[derive(Debug, Clone, Copy)]
enum ArchiveKind {
    Zip,
    TarXz,
}

#[derive(Debug)]
struct ArchiveMember {
    suffix: &'static str,
    destination_name: &'static str,
}

const MACOS_X64_RELEASE: ManagedRelease = ManagedRelease {
    provider: "evermeet.cx macOS x64 release ZIPs",
    install_id: "evermeet-getrelease",
    assumption: "Evermeet publishes standalone Intel macOS release ZIPs for ffmpeg and ffprobe.",
    archives: &[
        ArchiveSpec {
            archive_name: "ffmpeg-macos-x64.zip",
            url: "https://evermeet.cx/ffmpeg/getrelease/zip",
            kind: ArchiveKind::Zip,
            members: &[ArchiveMember {
                suffix: "ffmpeg",
                destination_name: "ffmpeg",
            }],
        },
        ArchiveSpec {
            archive_name: "ffprobe-macos-x64.zip",
            url: "https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip",
            kind: ArchiveKind::Zip,
            members: &[ArchiveMember {
                suffix: "ffprobe",
                destination_name: "ffprobe",
            }],
        },
    ],
};

const MACOS_ARM64_RELEASE: ManagedRelease = ManagedRelease {
    provider: "osxexperts.net macOS arm64 FFmpeg 6 ZIPs",
    install_id: "osxexperts-ffmpeg6arm",
    assumption: "osxexperts.net exposes Apple Silicon ZIPs at ffmpeg6arm.zip and ffprobe6arm.zip.",
    archives: &[
        ArchiveSpec {
            archive_name: "ffmpeg-macos-arm64.zip",
            url: "https://www.osxexperts.net/ffmpeg6arm.zip",
            kind: ArchiveKind::Zip,
            members: &[ArchiveMember {
                suffix: "ffmpeg",
                destination_name: "ffmpeg",
            }],
        },
        ArchiveSpec {
            archive_name: "ffprobe-macos-arm64.zip",
            url: "https://www.osxexperts.net/ffprobe6arm.zip",
            kind: ArchiveKind::Zip,
            members: &[ArchiveMember {
                suffix: "ffprobe",
                destination_name: "ffprobe",
            }],
        },
    ],
};

const LINUX_X64_RELEASE: ManagedRelease = ManagedRelease {
    provider: "johnvansickle.com Linux x64 static tar.xz",
    install_id: "johnvansickle-release-amd64-static",
    assumption: "johnvansickle.com publishes a static amd64 tar.xz that contains ffmpeg and ffprobe.",
    archives: &[ArchiveSpec {
        archive_name: "ffmpeg-linux-x64.tar.xz",
        url: "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz",
        kind: ArchiveKind::TarXz,
        members: &[
            ArchiveMember {
                suffix: "ffmpeg",
                destination_name: "ffmpeg",
            },
            ArchiveMember {
                suffix: "ffprobe",
                destination_name: "ffprobe",
            },
        ],
    }],
};

const WINDOWS_X64_RELEASE: ManagedRelease = ManagedRelease {
    provider: "gyan.dev Windows x64 release-essentials ZIP",
    install_id: "gyan-release-essentials",
    assumption: "gyan.dev publishes a Windows release-essentials ZIP that contains ffmpeg.exe and ffprobe.exe under bin/.",
    archives: &[ArchiveSpec {
        archive_name: "ffmpeg-windows-x64.zip",
        url: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
        kind: ArchiveKind::Zip,
        members: &[
            ArchiveMember {
                suffix: "bin/ffmpeg.exe",
                destination_name: "ffmpeg.exe",
            },
            ArchiveMember {
                suffix: "bin/ffprobe.exe",
                destination_name: "ffprobe.exe",
            },
        ],
    }],
};

pub(super) fn managed_release(
    platform: &RuntimePlatform,
) -> Result<&'static ManagedRelease, ManagedInstallError> {
    match platform {
        RuntimePlatform::MacosArm64 => Ok(&MACOS_ARM64_RELEASE),
        RuntimePlatform::MacosX64 => Ok(&MACOS_X64_RELEASE),
        RuntimePlatform::LinuxX64 => Ok(&LINUX_X64_RELEASE),
        RuntimePlatform::WindowsX64 => Ok(&WINDOWS_X64_RELEASE),
        RuntimePlatform::Unsupported { .. } => Err(ManagedInstallError::UnsupportedPlatform {
            platform: platform.clone(),
        }),
    }
}

pub(super) fn managed_cache_dir(
    platform: &RuntimePlatform,
) -> Result<PathBuf, ManagedInstallError> {
    let base = cache_base_dir_from_env(platform, |key| std::env::var_os(key).map(PathBuf::from))?;
    let cache_dir = base
        .join(env!("CARGO_PKG_NAME"))
        .join("tools")
        .join("ffmpeg");

    fs::create_dir_all(&cache_dir).map_err(|error| {
        ManagedInstallError::CacheDirectoryCreateFailed {
            path: cache_dir.clone(),
            details: error.to_string(),
        }
    })?;

    Ok(cache_dir)
}

pub(super) fn cache_base_dir_from_env<F>(
    platform: &RuntimePlatform,
    mut env_var: F,
) -> Result<PathBuf, ManagedInstallError>
where
    F: FnMut(&str) -> Option<PathBuf>,
{
    let home_dir = env_var("HOME").or_else(|| env_var("USERPROFILE"));
    let local_app_data = env_var("LOCALAPPDATA");
    let xdg_cache_home = env_var("XDG_CACHE_HOME");

    let home = || {
        home_dir
            .clone()
            .ok_or_else(|| ManagedInstallError::CacheDirectoryUnavailable {
                platform: platform.clone(),
                details: "HOME/USERPROFILE is not set".to_owned(),
            })
    };

    match platform {
        RuntimePlatform::MacosArm64 | RuntimePlatform::MacosX64 => {
            Ok(home()?.join("Library").join("Caches"))
        }
        RuntimePlatform::WindowsX64 => local_app_data
            .clone()
            .or_else(|| {
                home_dir
                    .clone()
                    .map(|path| path.join("AppData").join("Local"))
            })
            .ok_or_else(|| ManagedInstallError::CacheDirectoryUnavailable {
                platform: platform.clone(),
                details: "LOCALAPPDATA/USERPROFILE is not set".to_owned(),
            }),
        RuntimePlatform::LinuxX64 => Ok(xdg_cache_home.clone().unwrap_or(home()?.join(".cache"))),
        RuntimePlatform::Unsupported { os, .. } if os == "macos" => {
            Ok(home()?.join("Library").join("Caches"))
        }
        RuntimePlatform::Unsupported { os, .. } if os == "windows" => local_app_data
            .clone()
            .or_else(|| {
                home_dir
                    .clone()
                    .map(|path| path.join("AppData").join("Local"))
            })
            .ok_or_else(|| ManagedInstallError::CacheDirectoryUnavailable {
                platform: platform.clone(),
                details: "LOCALAPPDATA/USERPROFILE is not set".to_owned(),
            }),
        RuntimePlatform::Unsupported { .. } => {
            Ok(xdg_cache_home.clone().unwrap_or(home()?.join(".cache")))
        }
    }
}

pub(super) fn install_managed_toolset(
    platform: &RuntimePlatform,
) -> Result<FfmpegToolset, ManagedInstallError> {
    let release = managed_release(platform)?;
    let install_root = managed_cache_dir(platform)?
        .join(platform.cache_segment())
        .join(release.install_id);

    if install_root.exists() {
        match validate_install(platform, release, &install_root) {
            Ok(toolset) => return Ok(toolset),
            Err(_) => fs::remove_dir_all(&install_root).map_err(|error| {
                ManagedInstallError::StaleInstallCleanupFailed {
                    path: install_root.clone(),
                    details: error.to_string(),
                }
            })?,
        }
    }

    install_release(platform, release, &install_root)
}

fn install_release(
    platform: &RuntimePlatform,
    release: &'static ManagedRelease,
    install_root: &Path,
) -> Result<FfmpegToolset, ManagedInstallError> {
    let parent_dir = install_root
        .parent()
        .expect("install_root should always have a parent");
    fs::create_dir_all(parent_dir).map_err(|error| {
        ManagedInstallError::CacheDirectoryCreateFailed {
            path: parent_dir.to_path_buf(),
            details: error.to_string(),
        }
    })?;

    let staging_dir = install_root.with_extension("installing");
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir).map_err(|error| {
            ManagedInstallError::StaleInstallCleanupFailed {
                path: staging_dir.clone(),
                details: error.to_string(),
            }
        })?;
    }
    fs::create_dir_all(&staging_dir).map_err(|error| {
        ManagedInstallError::CacheDirectoryCreateFailed {
            path: staging_dir.clone(),
            details: error.to_string(),
        }
    })?;

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|error| ManagedInstallError::DownloadFailed {
            provider: release.provider,
            url: release.archives[0].url,
            details: error.to_string(),
        })?;

    for archive in release.archives {
        let archive_path = staging_dir.join(archive.archive_name);
        download_to(&client, release.provider, archive, &archive_path)?;
        match archive.kind {
            ArchiveKind::Zip => {
                extract_zip(release.provider, archive, &archive_path, &staging_dir)?
            }
            ArchiveKind::TarXz => {
                extract_tar_xz(release.provider, archive, &archive_path, &staging_dir)?
            }
        }
        let _ = fs::remove_file(&archive_path);
    }

    write_source_metadata(platform, release, &staging_dir)?;

    if install_root.exists() {
        fs::remove_dir_all(install_root).map_err(|error| {
            ManagedInstallError::StaleInstallCleanupFailed {
                path: install_root.to_path_buf(),
                details: error.to_string(),
            }
        })?;
    }

    fs::rename(&staging_dir, install_root).map_err(|error| {
        ManagedInstallError::InstallFinalizeFailed {
            install_dir: install_root.to_path_buf(),
            details: error.to_string(),
        }
    })?;

    validate_install(platform, release, install_root)
}

fn validate_install(
    platform: &RuntimePlatform,
    release: &'static ManagedRelease,
    install_root: &Path,
) -> Result<FfmpegToolset, ManagedInstallError> {
    let ffmpeg_path = install_root.join(platform.executable_name("ffmpeg"));
    let ffprobe_path = install_root.join(platform.executable_name("ffprobe"));

    if !ffmpeg_path.is_file() {
        return Err(ManagedInstallError::InstalledToolsInvalid {
            install_dir: install_root.to_path_buf(),
            details: format!("missing {}", ffmpeg_path.display()),
        });
    }
    if !ffprobe_path.is_file() {
        return Err(ManagedInstallError::InstalledToolsInvalid {
            install_dir: install_root.to_path_buf(),
            details: format!("missing {}", ffprobe_path.display()),
        });
    }

    let subtitle_filter =
        probe_ffmpeg(&ffmpeg_path).map_err(|error| ManagedInstallError::InstalledToolsInvalid {
            install_dir: install_root.to_path_buf(),
            details: error.to_string(),
        })?;
    probe_ffprobe(&ffprobe_path).map_err(|error| ManagedInstallError::InstalledToolsInvalid {
        install_dir: install_root.to_path_buf(),
        details: error.to_string(),
    })?;

    Ok(FfmpegToolset {
        ffmpeg: ffmpeg_path,
        ffprobe: ffprobe_path,
        subtitle_filter,
        origin: ToolOrigin::Managed(ManagedToolOrigin {
            provider: release.provider,
            install_dir: install_root.to_path_buf(),
            platform: platform.clone(),
        }),
    })
}

fn download_to(
    client: &Client,
    provider: &'static str,
    archive: &ArchiveSpec,
    destination: &Path,
) -> Result<(), ManagedInstallError> {
    let mut response = client
        .get(archive.url)
        .header(
            USER_AGENT,
            format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        )
        .send()
        .map_err(|error| ManagedInstallError::DownloadFailed {
            provider,
            url: archive.url,
            details: error.to_string(),
        })?
        .error_for_status()
        .map_err(|error| ManagedInstallError::DownloadFailed {
            provider,
            url: archive.url,
            details: error.to_string(),
        })?;

    let mut file =
        File::create(destination).map_err(|error| ManagedInstallError::DownloadFailed {
            provider,
            url: archive.url,
            details: error.to_string(),
        })?;

    io::copy(&mut response, &mut file).map_err(|error| ManagedInstallError::DownloadFailed {
        provider,
        url: archive.url,
        details: error.to_string(),
    })?;

    Ok(())
}

fn extract_zip(
    provider: &'static str,
    archive: &ArchiveSpec,
    archive_path: &Path,
    destination_dir: &Path,
) -> Result<(), ManagedInstallError> {
    let file = File::open(archive_path).map_err(|error| ManagedInstallError::ExtractionFailed {
        provider,
        archive_name: archive.archive_name,
        details: error.to_string(),
    })?;
    let mut zip = ZipArchive::new(file).map_err(|error| ManagedInstallError::ExtractionFailed {
        provider,
        archive_name: archive.archive_name,
        details: error.to_string(),
    })?;

    for member in archive.members {
        let mut found = false;
        for index in 0..zip.len() {
            let mut entry =
                zip.by_index(index)
                    .map_err(|error| ManagedInstallError::ExtractionFailed {
                        provider,
                        archive_name: archive.archive_name,
                        details: error.to_string(),
                    })?;

            if archive_entry_matches(entry.name(), member.suffix) {
                write_member(&mut entry, &destination_dir.join(member.destination_name)).map_err(
                    |error| ManagedInstallError::ExtractionFailed {
                        provider,
                        archive_name: archive.archive_name,
                        details: error.to_string(),
                    },
                )?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(ManagedInstallError::ArchiveMemberMissing {
                provider,
                archive_name: archive.archive_name,
                member: member.suffix,
            });
        }
    }

    Ok(())
}

fn extract_tar_xz(
    provider: &'static str,
    archive: &ArchiveSpec,
    archive_path: &Path,
    destination_dir: &Path,
) -> Result<(), ManagedInstallError> {
    let file = File::open(archive_path).map_err(|error| ManagedInstallError::ExtractionFailed {
        provider,
        archive_name: archive.archive_name,
        details: error.to_string(),
    })?;
    let decoder = XzDecoder::new(file);
    let mut tar = Archive::new(decoder);
    let mut found = vec![false; archive.members.len()];

    for entry in tar
        .entries()
        .map_err(|error| ManagedInstallError::ExtractionFailed {
            provider,
            archive_name: archive.archive_name,
            details: error.to_string(),
        })?
    {
        let mut entry = entry.map_err(|error| ManagedInstallError::ExtractionFailed {
            provider,
            archive_name: archive.archive_name,
            details: error.to_string(),
        })?;
        let entry_name = entry
            .path()
            .map_err(|error| ManagedInstallError::ExtractionFailed {
                provider,
                archive_name: archive.archive_name,
                details: error.to_string(),
            })?
            .to_string_lossy()
            .into_owned();

        for (index, member) in archive.members.iter().enumerate() {
            if found[index] || !archive_entry_matches(&entry_name, member.suffix) {
                continue;
            }

            write_member(&mut entry, &destination_dir.join(member.destination_name)).map_err(
                |error| ManagedInstallError::ExtractionFailed {
                    provider,
                    archive_name: archive.archive_name,
                    details: error.to_string(),
                },
            )?;
            found[index] = true;
            break;
        }

        if found.iter().all(|matched| *matched) {
            break;
        }
    }

    for (index, member) in archive.members.iter().enumerate() {
        if !found[index] {
            return Err(ManagedInstallError::ArchiveMemberMissing {
                provider,
                archive_name: archive.archive_name,
                member: member.suffix,
            });
        }
    }

    Ok(())
}

fn write_member(reader: &mut dyn Read, destination: &Path) -> io::Result<()> {
    let mut file = File::create(destination)?;
    io::copy(reader, &mut file)?;
    file.flush()?;
    make_executable(destination)?;
    Ok(())
}

fn archive_entry_matches(entry_name: &str, suffix: &str) -> bool {
    let normalized_name = entry_name.replace('\\', "/");
    let normalized_suffix = suffix.trim_start_matches('/');

    normalized_name == normalized_suffix
        || normalized_name.ends_with(&format!("/{normalized_suffix}"))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn write_source_metadata(
    platform: &RuntimePlatform,
    release: &ManagedRelease,
    staging_dir: &Path,
) -> Result<(), ManagedInstallError> {
    let path = staging_dir.join("SOURCE.txt");
    let mut file =
        File::create(&path).map_err(|error| ManagedInstallError::MetadataWriteFailed {
            path: path.clone(),
            details: error.to_string(),
        })?;

    writeln!(
        file,
        "Managed ffmpeg install for {platform}\nProvider: {}\nAssumption: {}\nURLs:",
        release.provider, release.assumption
    )
    .map_err(|error| ManagedInstallError::MetadataWriteFailed {
        path: path.clone(),
        details: error.to_string(),
    })?;

    for archive in release.archives {
        writeln!(file, "- {}", archive.url).map_err(|error| {
            ManagedInstallError::MetadataWriteFailed {
                path: path.clone(),
                details: error.to_string(),
            }
        })?;
    }

    Ok(())
}
