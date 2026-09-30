use anyhow::{Result, bail, ensure};
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// Archive and Unity paths use forward slashes independently of the host OS.
pub fn normalize(path: &str) -> Result<String> {
    ensure!(!path.is_empty(), "empty path");
    let path = path.replace('\\', "/");
    ensure!(!path.starts_with('/'), "absolute path: {path}");
    ensure!(
        !path.contains(':') && !path.chars().any(|c| c.is_control()),
        "invalid path: {path:?}"
    );
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => bail!("path traversal: {path}"),
            part => parts.push(part),
        }
    }
    ensure!(!parts.is_empty(), "empty normalized path: {path}");
    Ok(parts.join("/"))
}

pub fn fs_key(path: &str) -> String {
    // Conservative on macOS/Windows: do not silently overwrite case/normalization aliases.
    if cfg!(any(target_os = "windows", target_os = "macos")) {
        path.nfc().flat_map(char::to_lowercase).collect()
    } else {
        path.to_owned()
    }
}

pub fn check_host_path(path: &str) -> Result<()> {
    normalize(path)?;
    if cfg!(windows) {
        for part in path.split('/') {
            ensure!(
                !part.ends_with(['.', ' ']) && !part.contains(['<', '>', '"', '|', '?', '*']),
                "Windows cannot represent path component: {part}"
            );
            let stem = part.split('.').next().unwrap_or("").to_uppercase();
            let reserved = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];
            let numbered = (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.len() == 4
                && matches!(stem.as_bytes()[3], b'1'..=b'9');
            ensure!(
                !reserved.contains(&stem.as_str()) && !numbered,
                "Windows reserved filename: {part}"
            );
        }
    }
    Ok(())
}

pub fn reject_symlinks(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) => {
                ensure!(
                    !meta.file_type().is_symlink(),
                    "refusing symlink path: {}",
                    current.display()
                );
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    ensure!(
                        meta.file_attributes() & 0x400 == 0,
                        "refusing reparse point: {}",
                        current.display()
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
