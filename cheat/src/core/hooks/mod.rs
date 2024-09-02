use anyhow::{bail, Context};
use gher::mem::{Address, MemoryRegion};

use crate::{create_hook, cs2, get_original_fn};

#[cfg(windows)]
mod windows;

pub type CreateMove = extern "system" fn(*mut f32, u64, i8, u64, u64, u64);

unsafe extern "system" fn hk_create_move(
    a1: *mut f32,
    a2: u64,
    a3: i8,
    a4: u64,
    a5: u64,
    a6: u64,
) -> u64 {
    get_original_fn!(hk_create_move, original_fn, (*mut f32, u64, i8, u64, u64, u64), u64);

    tracing::info!("create move called");

    original_fn(a1, a2, a3, a4, a5, a6)
}

/// Initializes hooks for various game functions.
///
/// This function initializes `MinHook` and sets up hooks for the following game functions:
/// - `hk_create_move`: A hook for the game's create move function.
/// - `hk_present`: A hook for the game's present function.
/// - `hk_resize_buffers`: A hook for the game's resize buffers function.
///
/// # Errors
///
/// If `MinHook` fails to initialize, an error is returned with a message indicating the failure.
pub fn initialize_hooks() -> anyhow::Result<()> {
    trait DLExt {
        fn mem_or_die(&self) -> MemoryRegion;
    }
    impl DLExt for gher::DL {
        fn mem_or_die(&self) -> MemoryRegion {
            self.get_info()
                .with_context(|| format!("failed to get library info for {:?}", self.name()))
                .unwrap()
                .memory
        }
    }

    trait MemoryRegionExt {
        fn scan_or_err(&self, pattern_name: &str, pattern: &str) -> anyhow::Result<Address>;
    }
    impl MemoryRegionExt for MemoryRegion {
        fn scan_or_err(&self, pattern_name: &str, pattern: &str) -> anyhow::Result<Address> {
            match self.pattern_scan(pattern) {
                Ok(addr) => addr,
                Err(err) => {
                    tracing::error!("invalid {pattern_name} pattern: {err}");
                    bail!("invalid {pattern_name} pattern: {err}");
                }
            }
            .with_context(|| {
                tracing::error!("failed to find {pattern_name} pattern");
                format!("failed to find {pattern_name} pattern")
            })
        }
    }

    #[cfg(all(windows, feature = "directx11"))]
    {
        use windows::dx11::*;

        // Get the memory location of all of our modules.
        // Find the target addresses for the DirectX functions
        let gameoverlayrenderer64 = cs2::modules::gameoverlayrenderer64().mem_or_die();
        let present_target = gameoverlayrenderer64
            .scan_or_err("present",
                "48 89 5C 24 ?? 48 89 6C 24 ?? 48 89 74 24 ?? 57 41 56 41 57 48 83 EC 20 41 8B E8",
            )?.get();

        let resize_buffers_target = gameoverlayrenderer64
            .scan_or_err("resize buffers",
                "48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 41 56 41 57 48 83 EC 30 44",
            )?.get();

        // Create hooks for the graphics API(s)
        create_hook!(present_target, hk_present);
        create_hook!(resize_buffers_target, hk_resize_buffers);
    }

    // Get the memory location of all of our modules.
    let client = cs2::modules::client().mem_or_die();

    // Find the target addresses for the game functions
    let create_move_target = client.scan_or_err("create move", "48 8B C4 4C 89 48 20 55")?.get();

    // Create hooks for the game functions
    create_hook!(create_move_target, hk_create_move);

    Ok(())
}
