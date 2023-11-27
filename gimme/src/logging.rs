use env_logger::{Builder, Env};

pub fn init() {
    let env = Env::default()
        .filter_or("GIMME_LOG", "info")
        .write_style("GIMME_LOG_STYLE");
    Builder::from_env(env).init();
}
