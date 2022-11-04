fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    compile_icon_winres()?;

    Ok(())
}

#[cfg(windows)]
fn compile_icon_winres() -> anyhow::Result<()> {
    use anyhow::Context;

    let mut res = winresource::WindowsResource::new();
    res.set("OriginalFileName", "splot.exe");
    res.set_icon("./misc/splot.ico");
    res.compile()
        .context("Failed to compile winresource resource.")
}
