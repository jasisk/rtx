use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use clap::ValueHint;
use color_eyre::eyre::{eyre, Result};
use duct::IntoExecutablePath;

use crate::cli::args::tool::{ToolArg, ToolArgParser};
#[cfg(test)]
use crate::cmd;
use crate::config::Config;
use crate::env;
use crate::output::Output;
use crate::toolset::ToolsetBuilder;

/// Execute a command with tool(s) set
///
/// use this to avoid modifying the shell session or running ad-hoc commands with rtx tools set.
///
/// Tools will be loaded from .rtx.toml/.tool-versions, though they can be overridden with <RUNTIME> args
/// Note that only the plugin specified will be overridden, so if a `.tool-versions` file
/// includes "node 20" but you run `rtx exec python@3.11`; it will still load node@20.
///
/// The "--" separates runtimes from the commands to pass along to the subprocess.
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "x", verbatim_doc_comment, after_long_help = AFTER_LONG_HELP)]
pub struct Exec {
    /// Tool(s) to start
    /// e.g.: node@20 python@3.10
    #[clap(value_name = "TOOL@VERSION", value_parser = ToolArgParser)]
    pub tool: Vec<ToolArg>,

    /// Command string to execute (same as --command)
    #[clap(conflicts_with = "c", required_unless_present = "c", last = true)]
    pub command: Option<Vec<OsString>>,

    /// Command string to execute
    #[clap(short, long = "command", value_hint = ValueHint::CommandString, conflicts_with = "command")]
    pub c: Option<OsString>,

    /// Change to this directory before executing the command
    #[clap(short = 'C', value_hint = ValueHint::DirPath, long)]
    pub cd: Option<PathBuf>,
}

impl Exec {
    pub fn run(self, mut config: Config, _out: &mut Output) -> Result<()> {
        let mut ts = ToolsetBuilder::new()
            .with_args(&self.tool)
            .build(&mut config)?;
        ts.install_arg_versions(&mut config)?;

        let (program, args) = parse_command(&env::SHELL, &self.command, &self.c);
        let env = ts.env_with_path(&config);

        self.exec(program, args, env)
    }

    #[cfg(not(test))]
    fn exec<T, U, E>(&self, program: T, args: U, env: BTreeMap<E, E>) -> Result<()>
    where
        T: IntoExecutablePath,
        U: IntoIterator,
        U::Item: Into<OsString>,
        E: AsRef<OsStr>,
    {
        for (k, v) in env.iter() {
            env::set_var(k, v);
        }
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let program = program.to_executable();
        if let Some(cd) = &self.cd {
            env::set_current_dir(cd)?;
        }
        let err = exec::Command::new(program.clone()).args(&args).exec();
        Err(eyre!("{:?} {}", program.to_string_lossy(), err.to_string()))
    }

    #[cfg(test)]
    fn exec<T, U, E>(&self, program: T, args: U, env: BTreeMap<E, E>) -> Result<()>
    where
        T: IntoExecutablePath,
        U: IntoIterator,
        U::Item: Into<OsString>,
        E: AsRef<OsStr>,
    {
        let mut cmd = cmd::cmd(program, args);
        if let Some(cd) = &self.cd {
            cmd = cmd.dir(cd);
        }
        for (k, v) in env.iter() {
            cmd = cmd.env(k, v);
        }
        let res = cmd.unchecked().run()?;
        match res.status.code().unwrap_or(1) {
            0 => Ok(()),
            code => Err(eyre!("command failed with exit code {}", code)),
        }
    }
}

fn parse_command(
    shell: &str,
    command: &Option<Vec<OsString>>,
    c: &Option<OsString>,
) -> (OsString, Vec<OsString>) {
    match (&command, &c) {
        (Some(command), _) => {
            let (program, args) = command.split_first().unwrap();

            (program.clone(), args.into())
        }
        _ => (shell.into(), vec!["-c".into(), c.clone().unwrap()]),
    }
}

static AFTER_LONG_HELP: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</underline></bold>
  $ <bold>rtx exec node@20 -- node ./app.js</bold>  # launch app.js using node-20.x
  $ <bold>rtx x node@20 -- node ./app.js</bold>     # shorter alias

  # Specify command as a string:
  $ <bold>rtx exec node@20 python@3.11 --command "node -v && python -V"</bold>

  # Run a command in a different directory:
  $ <bold>rtx x -C /path/to/project node@20 -- node ./app.js</bold>
"#
);

#[cfg(test)]
mod tests {
    use crate::assert_cli;
    use crate::cli::tests::cli_run;

    #[test]
    fn test_exec_ok() {
        assert_cli!("exec", "--", "echo");
    }

    #[test]
    fn test_exec_fail() {
        let _ = cli_run(
            &vec!["rtx", "exec", "--", "exit", "1"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<String>>(),
        )
        .unwrap_err();
    }

    #[test]
    fn test_exec_cd() {
        assert_cli!("exec", "-C", "/tmp", "--", "pwd");
    }
}
