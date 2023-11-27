use std::path::PathBuf;

use clap::Args;
use eyre::Result;

use rtx::{Node as NodeProduct, Product};
use rtx_common::Context;

use crate::cli::Command;

#[derive(Args, Debug)]
pub struct Node {
    /// Always compile from source
    /// Defaults to false if a prebuilt binary is available
    #[clap(short, long, verbatim_doc_comment)]
    compile: bool,

    /// Skip verification of downloaded tarball
    #[clap(long)]
    no_verify: bool,

    /// Specify path for temporary build directory
    /// Tarballs will be cached here as well for potential reuse
    #[clap(long, env = "GIMME_NODE_BUILD_PATH", verbatim_doc_comment)]
    build_path: Option<PathBuf>,

    /// Use make instead of ninja
    #[clap(long)]
    no_ninja: bool,

    /// Build with node-build
    /// Legacy support for migrating away from node-build internally
    #[clap(long, verbatim_doc_comment, conflicts_with_all = & ["no_verify", "build_path"])]
    node_build: bool,

    /// Asset mirror to download tarballs from
    #[clap(
        long,
        env = "GIMME_MIRROR_URL",
        default_value = "https://nodejs.org/dist/",
        verbatim_doc_comment
    )]
    mirror: String,

    /// Number of concurrent jobs to use during compilation
    /// (default: number of CPUs)
    #[clap(long, env = "GIMME_NODE_CONCURRENCY", verbatim_doc_comment)]
    concurrency: Option<usize>,

    /// Path to C compiler
    #[clap(long, env = "CC", verbatim_doc_comment)]
    cc: Option<String>,

    /// Extra flags to pass to node during compilation
    /// (e.g. to override `-O3`)
    #[clap(long, env = "NODE_CFLAGS", verbatim_doc_comment)]
    node_cflags: Option<String>,

    /// Additional `./configure` flags during compilation
    #[clap(long, env = "CONFIGURE_OPTS", verbatim_doc_comment)]
    configure_opts: Option<String>,

    /// Custom `make` command (e.g. gmake)
    #[clap(long, env = "MAKE", verbatim_doc_comment)]
    make: Option<String>,

    /// Additional `make` options
    #[clap(long, env = "MAKE_OPTS", verbatim_doc_comment)]
    make_opts: Option<String>,

    /// Additional `make install` options
    #[clap(long, env = "MAKE_INSTALL_OPTS", verbatim_doc_comment)]
    make_install_opts: Option<String>,

    /// List available versions
    #[clap(short, long, conflicts_with_all = & ["output", "compile", "no_verify", "no_ninja", "node_build", "build_path"])]
    list: bool,

    /// Patch source code before compiling with a patch file
    /// Can specify multiple
    /// TODO: make this work
    #[clap(short, long, verbatim_doc_comment)]
    patch: Vec<PathBuf>,

    /// Node version to install
    /// Can be a partial version (e.g. 20) or a full version (e.g. 20.10.0)
    /// TODO: support git refs
    #[clap(verbatim_doc_comment)]
    version: Option<String>,

    /// Directory to install node into
    /// (default: node-v[VERSION]-[OS]-[ARCH])
    #[clap(verbatim_doc_comment)]
    output: Option<PathBuf>,
}

impl Command for Node {
    fn run(&self, ctx: Context) -> Result<()> {
        let mut node = NodeProduct::new(&ctx)?;
        if self.build_path.is_some() {
            node.build_path(self.build_path.clone());
        }
        if !self.patch.is_empty() {
            unimplemented!("patches not yet supported");
        }
        if self.compile {
            node.compile(self.compile);
        }
        if self.no_verify {
            node.verify(false);
        }
        if self.no_ninja {
            node.ninja(false);
        }
        if self.node_build {
            node.node_build(true);
        }
        if let Some(concurrency) = self.concurrency {
            node.concurrency(concurrency);
        }
        if let Some(cc) = &self.cc {
            node.cc(cc.clone());
        }
        if let Some(cflags) = &self.node_cflags {
            node.node_cflags(cflags.clone());
        }
        if let Some(configure_opts) = &self.configure_opts {
            node.configure_opts(configure_opts.clone());
        }
        if let Some(make) = &self.make {
            node.make(make.clone());
        }
        if let Some(make_opts) = &self.make_opts {
            node.make_opts(make_opts.clone());
        }
        if let Some(make_install_opts) = &self.make_install_opts {
            node.make_install_opts(make_install_opts.clone());
        }
        if self.list {
            return self.list_versions(&node);
        }
        let version = self.version.clone().unwrap_or_else(|| "latest".to_string());
        let version = node.resolve_version(&version)?;
        let output = self
            .output
            .clone()
            .unwrap_or_else(|| node.default_output_path(&version));
        node.install(&ctx, &version, &output)?;
        Ok(())
    }
}

impl Node {
    fn list_versions(&self, node: &NodeProduct) -> Result<()> {
        for version in node.filtered_versions(&self.version)? {
            println!("{}", version);
        }
        Ok(())
    }
}
