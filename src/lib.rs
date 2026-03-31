use std::fs;
use zed_extension_api::{self as zed, Result};

const BINARY_NAME: &str = "md-fold-server";

struct MarkdownFoldExtension {
    cached_binary_path: Option<String>,
}

impl MarkdownFoldExtension {
    fn find_binary(&mut self, worktree: &zed::Worktree) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        // Check worktree PATH first
        if let Some(path) = worktree.which(BINARY_NAME) {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        // Check extension's working directory (CWD = extensions/work/markdown-fold/)
        let local_path = BINARY_NAME.to_string();
        if fs::metadata(&local_path).map_or(false, |stat| stat.is_file()) {
            self.cached_binary_path = Some(local_path.clone());
            return Ok(local_path);
        }

        Err(format!(
            "{} not found. Copy it to the extension work directory.",
            BINARY_NAME
        ))
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
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: self.find_binary(worktree)?,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(MarkdownFoldExtension);
