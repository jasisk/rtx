use eyre::{bail, Result};
use rtx_common::Context;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

pub trait Product: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn default_output_path(&self, version: &str) -> PathBuf {
        Path::new(&format!("{}-{}", self.name(), version)).to_path_buf()
    }
    fn list_versions(&self, ctx: &Context) -> Result<Vec<String>>;
    fn install(&self, ctx: &Context, version: &str, output: &Path) -> Result<()>;
    fn filtered_versions(&self, filter: &Option<String>) -> Result<Vec<String>> {
        let mut versions = self.list_versions(&Context::new())?;
        if let Some(mut version) = filter.clone() {
            if regex!(r"^v\d+").is_match(&version) {
                version = version.strip_prefix('v').unwrap().to_string();
            }
            let filter = format!("{version}.");
            versions.retain(|v| v == &version || v.starts_with(&filter));
        }
        Ok(versions)
    }
    fn resolve_version(&self, version: &str) -> Result<String> {
        let filter = if version == "latest" {
            None
        } else if let Some(v) = regex!(r"^v?(\d+\.\d+\.\d+)$").captures(version) {
            // do not fetch versions if we have a full version
            return Ok(v[1].to_string());
        } else {
            Some(version.to_string())
        };
        let versions = self.filtered_versions(&filter)?;
        let version = versions.last().cloned();
        match version {
            Some(version) => Ok(version),
            None => bail!("no version found matching {}", version.clone().unwrap()),
        }
    }
}
