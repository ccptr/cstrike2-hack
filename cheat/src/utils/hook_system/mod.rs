use crate::common;
use anyhow::{bail, Context};
use common::c_void;
use lazy_static::lazy_static;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// Represents a function hook.
pub struct Hook {
    /// A pointer to the target function to be hooked.
    target: *mut c_void,
    /// A pointer to the detour function.
    detour: *mut c_void,
    /// A pointer to the original function.
    original: *mut c_void,
    /// A place to store the RawDetour object so that it won't get dropped.
    _hook: retour::RawDetour,
}

lazy_static! {
    static ref TARGETS: Arc<Mutex<VecDeque<Hook>>> = Arc::new(Mutex::new(VecDeque::new()));
}

unsafe impl Send for Hook {}

impl Hook {
    /// Retrieves the original function pointer for a given detour function.
    ///
    /// # Parameters
    ///
    /// - `func`: A function that returns the detour function pointer.
    ///
    /// # Returns
    ///
    /// An optional original function pointer wrapped in `Option<R>`.
    ///
    /// # Panics
    ///
    /// This function will panic if the `TARGETS` mutex is poisoned when locked. This might occur
    /// if another thread panics while holding the lock, which is an exceptional case in normal use.
    ///
    /// # Errors
    ///
    /// No errors are returned by this function, but note that the presence of `None` in the return type
    /// indicates that the original function was not found.
    pub fn get_proto_original<F, R>(func: F) -> Option<R>
    where
        F: Fn() -> *mut c_void,
        R: From<*mut c_void>,
    {
        let targets =
            TARGETS.lock().inspect_err(|err| tracing::error!("TARGETS is poisoned: {err}")).ok()?;

        targets.iter().find(|hook| hook.detour == func()).map(|hook| R::from(hook.original))
    }

    /// Hooks a target function with a detour function.
    ///
    /// # Parameters
    ///
    /// - `target`: A pointer to the target function.
    /// - `detour`: A pointer to the detour function.
    ///
    /// # Returns
    ///
    /// `true` if the hook was successfully created and enabled, `false` otherwise.
    ///
    /// # Panics
    ///
    /// Panics if it fails to lock the `TARGETS` mutex.
    pub fn hook(target: *const c_void, detour: *const c_void) -> anyhow::Result<()> {
        let Ok(mut targets) = TARGETS.lock() else {
            tracing::error!("failed to lock TARGETS");
            bail!("failed to lock TARGETS");
        };

        let hook = unsafe { retour::RawDetour::new(target as _, detour as _)? };

        unsafe {
            hook.enable()
                .with_context(|| format!("failed to enable hook {target:p} -> {detour:p}"))?;
        }

        let hk = Self {
            target: target.cast_mut(),
            detour: detour.cast_mut(),
            original: hook.trampoline() as *const () as *const c_void as *mut c_void,
            _hook: hook,
        };

        targets.push_back(hk);
        Ok(())
    }
}

#[macro_export]
macro_rules! create_hook {
    ($target_function:ident, $detour_function:ident) => {
        let target_function = $target_function as *const std::ffi::c_void;
        let detour_function_ptr = $detour_function as *const std::ffi::c_void;

        tracing::info!("hooking target function: {target_function:p}");

        if let Err(err) = crate::utils::hook_system::Hook::hook(target_function, detour_function_ptr) {
            anyhow::bail!("failed to create hook: {err}");
        }
    };
}

/// This macro is used to generate a function that retrieves the original function pointer of a hooked function.
///
/// # Parameters
///
/// * `$hook_name:ident` - The identifier of the hook to retrieve the original function pointer from.
/// * `$fn_name:ident` - The identifier of the generated function that will hold the original function pointer.
/// * `($($arg:ty),*)` - The types and names of the function's parameters.
/// * `$ret:ty` - The return type of the function.
///
/// # Return
///
/// The macro generates a function named `$fn_name` that takes the same parameters and return type as the original function.
/// This function retrieves the original function pointer from the specified hook and transmutes it to the appropriate function type.
/// The original function pointer is then returned.
#[macro_export]
macro_rules! get_original_fn {
    ($hook_name:ident, $fn_name:ident, ($($arg:ty),*), $ret:ty) => {
        // Safety: The `hook_system::Hook::get_proto_original` function is assumed to return a valid function pointer
        // for the specified hook. The `transmute` operation is safe here because the pointer is expected to be valid
        // and the type of the function signature matches the expected type.
        // The correctness of this operation depends on the implementation of `Hook::get_proto_original` and
        // the assumption that the function pointer returned is correctly typed and valid.
        let $fn_name: extern "system" fn($($arg),*) -> $ret = unsafe {
            std::mem::transmute::<
                *mut std::ffi::c_void,
                extern "system" fn($($arg),*) -> $ret,
            >(crate::utils::hook_system::Hook::get_proto_original(|| $hook_name as *mut std::ffi::c_void).unwrap())
        };
    };
}
