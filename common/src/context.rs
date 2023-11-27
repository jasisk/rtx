use std::path::PathBuf;
use std::process::Command;

use eyre::{bail, Result};

pub struct Context<'a> {
    pub user_agent: String,
    pub home_dir: PathBuf,
    pub xdg_cache_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub on_stdout: Box<dyn Fn(String) + 'a>,
    pub on_stderr: Box<dyn Fn(String) + 'a>,
    pub on_exec: Box<dyn Fn(Command) -> Result<()> + 'a>,
}

impl<'a> Context<'a> {
    pub fn new() -> Self {
        let home_dir = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let xdg_cache_dir = dirs_next::cache_dir().unwrap_or_else(|| home_dir.join(".cache"));
        Self {
            user_agent: format!("rtx/{}", env!("CARGO_PKG_VERSION")),
            cache_dir: xdg_cache_dir.join("rtx"),
            home_dir,
            xdg_cache_dir,
            on_stdout: Box::new(|line| println!("{}", line)),
            on_stderr: Box::new(|line| eprintln!("{}", line)),
            on_exec: Box::new(|mut cmd| {
                let status = cmd.status()?;
                if !status.success() {
                    bail!("command failed: {}", status);
                }
                Ok(())
            }),
        }
    }

    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = user_agent.to_string();
        self
    }

    pub fn on_stdout<F: Fn(String) + 'static>(mut self, f: F) -> Self {
        self.on_stdout = Box::new(f);
        self
    }

    pub fn on_stderr<F: Fn(String) + 'static>(mut self, f: F) -> Self {
        self.on_stderr = Box::new(f);
        self
    }

    pub fn println(&self, line: String) {
        (self.on_stdout)(line)
    }

    pub fn eprintln(&self, line: String) {
        (self.on_stderr)(line)
    }

    pub fn exec(&self, cmd: Command) -> Result<()> {
        (self.on_exec)(cmd)
    }
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! ctxprintln {
    ($ctx:ident, $($arg:tt)*) => {
        $ctx.println(format!($($arg)*));
    };
}
#[macro_export]
macro_rules! ctxeprintln {
    ($ctx:ident, $($arg:tt)*) => {
        $ctx.eprintln(format!($($arg)*));
    };
}
