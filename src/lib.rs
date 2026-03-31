use std::fs;
use zed_extension_api::{self as zed, Result};

const BINARY_NAME: &str = "md-fold-server";
const GITHUB_REPO: &str = "rsomani95/zed-markdown-fold";

struct MarkdownFoldExtension {
    cached_binary_path: Option<String>,
}

impl MarkdownFoldExtension {
    fn language_server_binary(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // 1. Check cached path from a previous invocation
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        // 2. Check system PATH (works for both local and remote)
        if let Some(path) = worktree.which(BINARY_NAME) {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        // 3. Download from GitHub releases (or use previously downloaded)
        if let Ok(path) = self.ensure_binary_from_release(language_server_id) {
            return Ok(path);
        }

        // 4. Fallback: check extension working directory (manual install)
        if fs::metadata(BINARY_NAME).map_or(false, |m| m.is_file()) {
            self.cached_binary_path = Some(BINARY_NAME.to_string());
            return Ok(BINARY_NAME.to_string());
        }

        Err(format!(
            "{BINARY_NAME} not found. Create a GitHub release at {GITHUB_REPO} \
             or place the binary in the extension work directory."
        ))
    }

    fn ensure_binary_from_release(
        &mut self,
        language_server_id: &zed::LanguageServerId,
    ) -> Result<String> {
        let (os, arch) = zed::current_platform();
        let platform = match (os, arch) {
            (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
            (zed::Os::Mac, zed::Architecture::X8664) => "x86_64-apple-darwin",
            (zed::Os::Linux, zed::Architecture::Aarch64) => "aarch64-unknown-linux-musl",
            (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-musl",
            _ => return Err("unsupported platform".into()),
        };

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = format!("{BINARY_NAME}-{platform}.gz");
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no release asset '{asset_name}' found"))?;

        let version_dir = format!("{BINARY_NAME}-{}", release.version);
        let binary_path = format!("{version_dir}/{BINARY_NAME}");

        if fs::metadata(&binary_path).map_or(false, |m| m.is_file()) {
            self.cached_binary_path = Some(binary_path.clone());
            return Ok(binary_path);
        }

        // Download the binary
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        fs::create_dir_all(&version_dir)
            .map_err(|e| format!("failed to create directory '{version_dir}': {e}"))?;

        zed::download_file(
            &asset.download_url,
            &binary_path,
            zed::DownloadedFileType::Gzip,
        )?;

        zed::make_file_executable(&binary_path)?;

        // Clean up old version directories
        if let Ok(entries) = fs::read_dir(".") {
            let prefix = format!("{BINARY_NAME}-");
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix) && name != version_dir {
                    let _ = fs::remove_dir_all(&name);
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for MarkdownFoldExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: self.language_server_binary(language_server_id, worktree)?,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(MarkdownFoldExtension);
