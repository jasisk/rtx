use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

use eyre::Result;
use once_cell::sync::OnceCell;
use serde_derive::Deserialize;
use tempfile::TempDir;

use rtx_common::http::Client;
use rtx_common::{file, hash, http, tar, Context};
use url::Url;

use crate::built_info;
use crate::Product;

#[derive(Debug)]
pub struct Node {
    compile: bool,
    verify: bool,
    ninja: bool,
    node_build: bool,
    mirror: Url,
    build_path: Option<PathBuf>,

    concurrency: Option<usize>,
    cc: Option<String>,
    node_cflags: Option<String>,
    configure_opts: Option<String>,
    make: String,
    make_opts: Option<String>,
    make_install_opts: Option<String>,

    http: Client,
    tmpdir: OnceCell<TempDir>,
}

impl Product for Node {
    fn name(&self) -> &'static str {
        "node"
    }

    fn default_output_path(&self, version: &str) -> PathBuf {
        slug(version, os(), arch()).into()
    }
    fn list_versions(&self, _ctx: &Context) -> Result<Vec<String>> {
        let versions = self
            .http
            .get(self.mirror.join("index.json")?)?
            .json::<Vec<NodeVersion>>()?
            .into_iter()
            .map(|v| {
                if regex!(r"^v\d+\.").is_match(&v.version) {
                    v.version.strip_prefix('v').unwrap().to_string()
                } else {
                    v.version
                }
            })
            .rev()
            .collect();
        Ok(versions)
    }

    fn install(&self, ctx: &Context, version: &str, output: &Path) -> Result<()> {
        debug!("installing node {version} to {}", output.display());
        if self.node_build {
            unimplemented!("node-build not yet supported");
            // self.ensure_node_build(ctx)?;
            // self.install_node_build(ctx)?;
        }
        if !self.compile {
            match self.install_binary(ctx, version, output) {
                Err(e) if matches!(http::error_code(&e), Some(404)) => {}
                e => return e,
            }
        }
        self.install_source(ctx, version, output)?;
        Ok(())
    }
}

impl Node {
    pub fn new(ctx: &Context) -> Result<Self> {
        Ok(Self {
            compile: false,
            verify: true,
            ninja: which("ninja").is_ok(),
            mirror: Url::parse("https://nodejs.org/dist/").unwrap(),
            node_build: false,
            build_path: None,

            concurrency: None,
            cc: None,
            node_cflags: None,
            configure_opts: None,
            make: "make".into(),
            make_opts: None,
            make_install_opts: None,

            http: Client::new(ctx)?,
            tmpdir: OnceCell::new(),
        })
    }

    pub fn compile(&mut self, compile: bool) -> &mut Self {
        self.compile = compile;
        self
    }

    pub fn verify(&mut self, verify: bool) -> &mut Self {
        self.verify = verify;
        self
    }

    pub fn node_build(&mut self, node_build: bool) -> &mut Self {
        self.node_build = node_build;
        self
    }
    pub fn ninja(&mut self, ninja: bool) -> &mut Self {
        self.ninja = ninja;
        self
    }

    pub fn mirror(&mut self, mirror: Url) -> &mut Self {
        self.mirror = mirror;
        self
    }

    pub fn build_path(&mut self, build_path: Option<PathBuf>) -> &mut Self {
        self.build_path = build_path;
        self
    }
    pub fn concurrency(&mut self, concurrency: usize) -> &mut Self {
        self.concurrency = Some(concurrency);
        self
    }
    pub fn cc(&mut self, cc: String) -> &mut Self {
        self.cc = Some(cc);
        self
    }
    pub fn node_cflags(&mut self, node_cflags: String) -> &mut Self {
        self.node_cflags = Some(node_cflags);
        self
    }
    pub fn configure_opts(&mut self, configure_opts: String) -> &mut Self {
        self.configure_opts = Some(configure_opts);
        self
    }
    pub fn make(&mut self, make: String) -> &mut Self {
        self.make = make;
        self
    }
    pub fn make_opts(&mut self, make_opts: String) -> &mut Self {
        self.make_opts = Some(make_opts);
        self
    }
    pub fn make_install_opts(&mut self, make_install_opts: String) -> &mut Self {
        self.make_install_opts = Some(make_install_opts);
        self
    }

    pub fn install_binary(&self, ctx: &Context, version: &str, output: &Path) -> Result<()> {
        let tarball_name = binary_tarball_name(version);
        let build_path = self.build_cache_path()?;
        let tmp_tarball = build_path.join(&tarball_name);
        if tmp_tarball.exists() {
            ctxprintln!(ctx, "Using cached tarball {tarball_name}");
        } else {
            let url = self.binary_tarball_url(version)?;
            ctxprintln!(ctx, "Downloading {url}");
            self.http.download_file(url, &tmp_tarball)?;
        }
        if self.verify {
            ctxprintln!(ctx, "Verifying {tarball_name}");
            self.verify_tarball(version, &tmp_tarball)?;
        }
        ctxprintln!(ctx, "Extracting {tarball_name}");
        self.extract_tarball(version, &tmp_tarball, output)?;
        ctxprintln!(ctx, "installed node {version} to {}", output.display());
        Ok(())
    }
    pub fn install_source(&self, ctx: &Context, version: &str, output: &Path) -> Result<()> {
        debug!("installing from source");
        let tarball_name = source_tarball_name(version);
        let cache_path = self.build_cache_path()?;
        let build_path = cache_path.join(format!("node-v{}", version));
        let tmp_tarball = cache_path.join(&tarball_name);
        if tmp_tarball.exists() {
            ctxprintln!(ctx, "Using cached tarball {tarball_name}");
        } else {
            let url = self.source_tarball_url(version)?;
            ctxprintln!(ctx, "Downloading {url}");
            self.http.download_file(url, &tmp_tarball)?;
        }
        if self.verify {
            ctxprintln!(ctx, "Verifying {tarball_name}");
            self.verify_tarball(version, &tmp_tarball)?;
        }
        ctxprintln!(ctx, "Extracting source from {tarball_name}");
        file::remove_all(&build_path)?;
        tar::untar(&tmp_tarball, cache_path)?;

        ctxprintln!(ctx, "Compiling node");
        self.exec_configure(ctx, &build_path, output)?;
        self.exec_make(ctx, &build_path)?;
        self.exec_make_install(ctx, &build_path)?;
        ctxprintln!(ctx, "Installed node@{version} to {}", output.display());
        Ok(())
    }

    fn verify_tarball(&self, version: &str, tarball: &Path) -> Result<()> {
        // TODO: gpg verify shasums
        let tarball_name = file::file_name(tarball).unwrap();
        // TODO: fetch shasums in parallel
        let text = self.http.get(self.shasums_url(version)?)?.text()?;
        let shasums = hash::parse_shasums(&text);
        let hash = shasums.get(tarball_name).unwrap();
        hash::ensure_checksum_sha256(tarball, hash)
    }

    fn extract_tarball(&self, version: &str, tarball: &Path, output: &Path) -> Result<()> {
        // TODO: extract directly to output
        let tmp = tempfile::tempdir_in(output.parent().unwrap())?;
        tar::untar(tarball, tmp.path())?;
        if output.exists() {
            file::remove_all(output)?;
        }
        file::rename(tmp.path().join(slug(version, os(), arch())), output)?;
        Ok(())
    }

    fn build_cache_path(&self) -> Result<&Path> {
        if let Some(ref bp) = self.build_path {
            debug!("build path specified {}", bp.display());
            file::create_dir_all(bp)?;
            return Ok(bp);
        }
        let tmpdir = self.tmpdir.get_or_try_init(tempfile::tempdir)?;
        Ok(tmpdir.path())
    }
    fn exec_configure(&self, ctx: &Context, build_path: &Path, output: &Path) -> Result<()> {
        let mut cmd = Command::new("bash");
        cmd.current_dir(build_path);
        cmd.arg("-c");
        cmd.arg(
            format!(
                "./configure --prefix={} {} {}",
                output.display(),
                self.configure_opts.as_deref().unwrap_or_default(),
                if self.ninja { "--ninja" } else { "" },
            )
            .trim(),
        );
        self.exec_common_env(&mut cmd);
        ctx.exec(cmd)
    }
    fn exec_make(&self, ctx: &Context, build_path: &Path) -> Result<()> {
        let mut cmd = Command::new("sh");
        cmd.current_dir(build_path);
        cmd.arg("-c");
        cmd.arg(
            format!(
                "{} {} {}",
                self.make,
                self.concurrency
                    .map(|c| format!("-j{}", c))
                    .unwrap_or_default(),
                self.make_opts.as_deref().unwrap_or_default(),
            )
            .trim(),
        );
        self.exec_common_env(&mut cmd);
        ctx.exec(cmd)
    }
    fn exec_make_install(&self, ctx: &Context, build_path: &Path) -> Result<()> {
        let mut cmd = Command::new("sh");
        cmd.current_dir(build_path);
        cmd.arg("-c");
        cmd.arg(
            format!(
                "{} install {}",
                self.make,
                self.make_install_opts.as_deref().unwrap_or_default(),
            )
            .trim(),
        );
        self.exec_common_env(&mut cmd);
        ctx.exec(cmd)
    }

    fn exec_common_env(&self, cmd: &mut Command) {
        if let Some(cc) = &self.cc {
            cmd.env("CC", cc);
        }
        if let Some(node_cflags) = &self.node_cflags {
            cmd.env("CFLAGS", node_cflags);
        }
    }

    fn source_tarball_url(&self, v: &str) -> Result<Url> {
        let url = self
            .mirror
            .join(&format!("v{v}/{}", source_tarball_name(v)))?;
        Ok(url)
    }
    fn binary_tarball_url(&self, v: &str) -> Result<Url> {
        let url = self
            .mirror
            .join(&format!("v{v}/{}", binary_tarball_name(v)))?;
        Ok(url)
    }
    fn shasums_url(&self, v: &str) -> Result<Url> {
        let url = self.mirror.join(&format!("v{v}/SHASUMS256.txt"))?;
        Ok(url)
    }
}

#[derive(Debug, Deserialize)]
struct NodeVersion {
    version: String,
}

fn os() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "win"
    } else {
        built_info::CFG_OS
    }
}

fn arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "arm") {
        "armv7l"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        built_info::CFG_TARGET_ARCH
    }
}

fn slug(v: &str, os: &str, arch: &str) -> String {
    format!("node-v{v}-{os}-{arch}")
}
fn source_tarball_name(v: &str) -> String {
    format!("node-v{v}.tar.gz")
}
fn binary_tarball_name(v: &str) -> String {
    format!("{}.tar.gz", slug(v, os(), arch()))
}
