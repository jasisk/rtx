use rtx_common::Context;

pub fn gen_context<'a>() -> Context<'a> {
    let mut ctx = Context::new().user_agent(&format!("rtx/{}", env!("CARGO_PKG_VERSION")));
    ctx.on_exec = Box::new(|cmd| {
        dbg!(cmd);
        Ok(())
    });
    ctx
}
