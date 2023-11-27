use eyre::{Context, Result};
use std::fs;
use std::path::Path;

pub fn remove_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    match path.metadata().map(|m| m.file_type()) {
        Ok(x) if x.is_symlink() || x.is_file() => {
            remove_file(path)?;
        }
        Ok(x) if x.is_dir() => {
            trace!("rm -rf {}", path.display());
            fs::remove_dir_all(path)
                .with_context(|| format!("failed rm -rf: {}", path.display()))?;
        }
        _ => {}
    };
    Ok(())
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    trace!("rm {}", path.display());
    fs::remove_file(path).with_context(|| format!("failed rm: {}", path.display()))
}

pub fn remove_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    trace!("rmdir {}", path.display());
    fs::remove_dir(path).with_context(|| format!("failed rmdir: {}", path.display()))
}

pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    trace!("mv {} {}", from.display(), to.display());
    fs::rename(from, to)
        .with_context(|| format!("failed rename: {} -> {}", from.display(), to.display()))
}

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    trace!("mkdir -p {}", path.display());
    fs::create_dir_all(path).with_context(|| format!("failed create_dir_all: {}", path.display()))
}

pub fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|s| s.to_str())
}
