use crate::common::*;

use anyhow::{bail, Context};
use gher::DL;
use once_cell::sync::OnceCell;

pub trait DLExt {
    fn get_interface(&self, interface_name: &CStr) -> Option<*const c_void>;
}

impl DLExt for gher::DL {
    /// Retrieves an interface from the module.
    ///
    /// # Parameters
    /// - `interface_name`: The name of the interface to retrieve.
    ///
    /// # Returns
    /// A pointer to the interface if found, otherwise `None`.
    ///
    /// # Examples
    /// ```
    /// let interface_ptr = module.get_interface("CreateInterface");
    /// ```
    #[must_use]
    fn get_interface(&self, interface_name: &CStr) -> Option<*const c_void> {
        get_interface(self, interface_name)
    }
}

/// Retrieves a pointer to a specific interface from a module.
///
/// This function uses the `CreateInterface` function from the specified module to obtain a pointer to
/// a requested interface. The interface is identified by its name, which is passed as a parameter to
/// the function.
///
/// # Parameters
///
/// * `module_handle`: A handle to the module containing the `CreateInterface` function.
///   This can be obtained using the `get_module_handle` function.
///
/// * `interface_name`: A string representing the name of the interface to retrieve.
///   The name should match the name used by the module to identify the interface.
///
/// # Returns
///
/// * `Some(interface_ptr)`: If the interface is successfully retrieved. The `interface_ptr` is a raw pointer
///   to the requested interface.
///
/// * `None`: If the interface cannot be retrieved or if an error occurs.
///
/// # Note
///
/// The returned pointer is raw and should be used with caution. Ensure that the pointer is valid before
/// dereferencing or using it.
#[must_use]
pub fn get_interface(module: &gher::DL, interface_name: &CStr) -> Option<*const c_void> {
    type CreateInterfaceFn = unsafe extern "C" fn(*const c_char, *const c_int) -> *const c_void;

    // SAFETY: We assume that `get_proc_address` returns a valid function pointer.
    let create_interface_fn =
        module.get_symbol(c"CreateInterface").map(|function| unsafe { transmute(function) });

    let create_interface_fn: CreateInterfaceFn = if let Some(function) = create_interface_fn {
        function
    } else {
        tracing::error!("failed to get function address for CreateInterface");
        return None;
    };

    // SAFETY: We assume that `function` is a valid function pointer and `interface_name_cstr` is valid.
    // TODO: can CreateInterface return null? If so we should probably check and return None
    Some(unsafe { create_interface_fn(interface_name.as_ptr(), null_mut()) })
}

/// A global static variable holding the list of initialized modules.
///
/// This variable is initialized only once and protected by a `Mutex` to ensure thread safety.
static MODULES: OnceCell<Mutex<Vec<gher::DL>>> = OnceCell::new();

/// Initializes the global `MODULES` with the provided module names.
///
/// # Parameters
/// - `names`: A slice of module names to initialize.
///
/// # Returns
/// A `Result` indicating success or failure. If the initialization fails, it returns an error.
///
/// # Errors
/// - Returns an error if modules are already initialized.
/// - Panics if setting the global `MODULES` fails.
///
/// # Panics
/// This function will panic if `MODULES.set(...)` fails or if `MODULES.get()` returns `None`
/// while trying to access the modules. This can happen if the modules were not properly initialized.
///
/// # Examples
/// ```no_run
/// let result = initialize_modules(&["module1.dll", "module2.dll"]);
/// match result {
///     Ok(_) => println!("Modules initialized successfully"),
///     Err(e) => eprintln!("Failed to initialize modules: {:?}", e),
/// }
/// ```
pub fn initialize_modules(names: &[&'static CStr]) -> anyhow::Result<()> {
    if MODULES.get().is_some() {
        bail!("modules are already initialized");
    }

    let mut modules = Vec::with_capacity(names.len());
    for &name in names {
        let dl = DL::open(name).with_context(|| format!("failed to open {name:?}"))?;

        tracing::info!("initialized module: {:?} {:p}", dl.name(), dl.handle());
        modules.push(dl);
    }

    match MODULES.set(Mutex::new(modules)) {
        Ok(_) => {}
        Err(e) => bail!("failed to initialize MODULES: {e:?}"),
    }

    Ok(())
}

/// This macro generates accessor functions for static instances of the `Module` struct.
/// These functions allow easy access to the initialized modules without needing to manually manage their lifetimes.
///
/// # Arguments
///
/// * `$($name:ident),*` - A list of module names (without the ".dll" extension) for which accessor functions will be generated.
///
/// # Example
///
macro_rules! define_module_accessors {
    ($($name:ident),*) => {
        $(
            /// Accessor function for the module.
            ///
            /// # Panics
            /// Panics if the module is not initialized or if the module is not found.
            pub fn $name() -> &'static gher::DL {
                #[cfg(windows)]
                let module_name = concat!(stringify!($name), ".dll");
                #[cfg(not(windows))]
                let module_name = concat!(stringify!($name), ".so");

                let modules_guard = MODULES.get().expect("modules are not initialized").lock();
                let module = modules_guard.iter()
                    .find(|module| module.name().as_bytes() == module_name.as_bytes())
                    .unwrap_or_else(|| panic!("module {} is not found", module_name));

                Box::leak(Box::new(module.clone()))
            }
        )*
    };
}

define_module_accessors!(client, engine2, gameoverlayrenderer64);
