use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;

use rtx::{Node as NodeProduct, Product};

use crate::cmd::CmdLineRunner;
use crate::config::{Config, Settings};
use crate::context::gen_context;
use crate::env::{RTX_NODE_CONCURRENCY, RTX_NODE_FORCE_COMPILE};
use crate::plugins::core::CorePlugin;
use crate::plugins::{Plugin, PluginName};
use crate::toolset::{ToolVersion, ToolVersionRequest};
use crate::ui::progress_report::ProgressReport;
use crate::{env, file};

#[derive(Debug)]
pub struct NodePlugin {
    core: CorePlugin,
}

impl NodePlugin {
    pub fn new(name: PluginName) -> Self {
        Self {
            core: CorePlugin::new(name),
        }
    }

    fn fetch_remote_versions(&self) -> Result<Vec<String>> {
        CorePlugin::run_fetch_task_with_timeout(|| {
            let ctx = gen_context();
            let node = NodeProduct::new(&ctx)?;
            node.list_versions(&ctx)
        })
    }

    fn node_path(&self, tv: &ToolVersion) -> PathBuf {
        tv.install_path().join("bin/node")
    }

    fn npm_path(&self, tv: &ToolVersion) -> PathBuf {
        tv.install_path().join("bin/npm")
    }

    fn install_default_packages(
        &self,
        settings: &Settings,
        tv: &ToolVersion,
        pr: &ProgressReport,
    ) -> Result<()> {
        let body = file::read_to_string(&*env::RTX_NODE_DEFAULT_PACKAGES_FILE).unwrap_or_default();
        for package in body.lines() {
            let package = package.split('#').next().unwrap_or_default().trim();
            if package.is_empty() {
                continue;
            }
            pr.set_message(format!("installing default package: {}", package));
            let npm = self.npm_path(tv);
            CmdLineRunner::new(settings, npm)
                .with_pr(pr)
                .arg("install")
                .arg("--global")
                .arg(package)
                .env("PATH", CorePlugin::path_env_with_tv_path(tv)?)
                .execute()?;
        }
        Ok(())
    }

    fn install_npm_shim(&self, tv: &ToolVersion) -> Result<()> {
        file::remove_file(self.npm_path(tv)).ok();
        file::write(self.npm_path(tv), include_str!("assets/node_npm_shim"))?;
        file::make_executable(&self.npm_path(tv))?;
        Ok(())
    }

    fn test_node(&self, config: &Config, tv: &ToolVersion, pr: &ProgressReport) -> Result<()> {
        pr.set_message("node -v");
        CmdLineRunner::new(&config.settings, self.node_path(tv))
            .with_pr(pr)
            .arg("-v")
            .execute()
    }

    fn test_npm(&self, config: &Config, tv: &ToolVersion, pr: &ProgressReport) -> Result<()> {
        pr.set_message("npm -v");
        CmdLineRunner::new(&config.settings, self.npm_path(tv))
            .env("PATH", CorePlugin::path_env_with_tv_path(tv)?)
            .with_pr(pr)
            .arg("-v")
            .execute()
    }
}

impl Plugin for NodePlugin {
    fn name(&self) -> &PluginName {
        &self.core.name
    }

    fn list_remote_versions(&self, _settings: &Settings) -> Result<Vec<String>> {
        self.core
            .remote_version_cache
            .get_or_try_init(|| self.fetch_remote_versions())
            .cloned()
    }

    fn get_aliases(&self, _settings: &Settings) -> Result<BTreeMap<String, String>> {
        let aliases = [
            ("lts/argon", "4"),
            ("lts/boron", "6"),
            ("lts/carbon", "8"),
            ("lts/dubnium", "10"),
            ("lts/erbium", "12"),
            ("lts/fermium", "14"),
            ("lts/gallium", "16"),
            ("lts/hydrogen", "18"),
            ("lts/iron", "20"),
            ("lts-argon", "4"),
            ("lts-boron", "6"),
            ("lts-carbon", "8"),
            ("lts-dubnium", "10"),
            ("lts-erbium", "12"),
            ("lts-fermium", "14"),
            ("lts-gallium", "16"),
            ("lts-hydrogen", "18"),
            ("lts-iron", "20"),
            ("lts", "20"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        Ok(aliases)
    }

    fn legacy_filenames(&self, _settings: &Settings) -> Result<Vec<String>> {
        Ok(vec![".node-version".into(), ".nvmrc".into()])
    }

    fn parse_legacy_file(&self, path: &Path, _settings: &Settings) -> Result<String> {
        let body = file::read_to_string(path)?;
        // trim "v" prefix
        let body = body.trim().strip_prefix('v').unwrap_or(&body);
        // replace lts/* with lts
        let body = body.replace("lts/*", "lts");
        Ok(body.to_string())
    }

    fn install_version(
        &self,
        config: &Config,
        tv: &ToolVersion,
        pr: &ProgressReport,
    ) -> Result<()> {
        let mut ctx = gen_context();
        ctx.on_stderr = Box::new(|line| pr.println(line));
        ctx.on_stdout = Box::new(|line| pr.set_message(line));
        ctx.on_exec = Box::new(|cmd| {
            CmdLineRunner::new_from_cmd(&config.settings, cmd)
                .with_pr(pr)
                .execute()
        });
        let mut node = NodeProduct::new(&ctx)?;
        if matches!(&tv.request, ToolVersionRequest::Ref { .. }) || *RTX_NODE_FORCE_COMPILE {
            if let Some(concurrency) = *RTX_NODE_CONCURRENCY {
                node.concurrency(concurrency);
            }
            node.configure_opts(env::var("CONFIGURE_OPTS").unwrap_or_default());
            node.make_opts(env::var("MAKE_OPTS").unwrap_or_default());
            node.make_install_opts(env::var("MAKE_INSTALL_OPTS").unwrap_or_default());
            node.compile(true);
        }
        node.install(&ctx, &tv.version, &tv.install_path())?;
        self.test_node(config, tv, pr)?;
        self.install_npm_shim(tv)?;
        self.test_npm(config, tv, pr)?;
        self.install_default_packages(&config.settings, tv, pr)?;
        Ok(())
    }
}
