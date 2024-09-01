use crate::utils::find_window;
use anyhow::Context;

pub mod dx11;
pub mod fonts;
#[cfg(windows)]
pub mod win32;

pub fn setup() -> anyhow::Result<()> {
    let window = find_window().context("could not find window")?;

    fonts::setup().context("failed to setup fonts")?;
    #[cfg(windows)]
    win32::setup(window).context("failed to setup WNDPROC hook")?;

    Ok(())
}
