use crate::common::Handle;

pub mod hook_system;
pub mod render;

#[cfg(windows)]
mod windows {
    use windows::Win32::{
        Foundation::{BOOL, FALSE, HWND, LPARAM, TRUE},
        System::{Console::GetConsoleWindow, Threading::GetCurrentProcessId},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
        },
    };

    /// Determines whether a given window is the main window of the current process.
    ///
    /// This function checks if the specified window is a top-level window, has no owner, and is visible.
    /// It is intended to be used as a helper function for identifying the main window of the current process.
    ///
    /// # Parameters
    ///
    /// * `window`: A handle to the window to be checked.
    ///
    /// # Returns
    ///
    /// * `true` if the specified window is the main window of the current process.
    /// * `false` if the specified window is not the main window of the current process.
    unsafe fn is_main_window(window: HWND) -> bool {
        GetWindow(window, GW_OWNER).unwrap_or_default().0
            .is_null() && IsWindowVisible(window) == TRUE
    }

    /// An unsafe extern "system" function used as a callback for the `EnumWindows` function.
    /// This function is intended to identify the main window of the current process.
    ///
    /// # Parameters
    ///
    /// * `window`: A handle to the window being enumerated.
    /// * `lparam`: A pointer to a mutable memory location where the function can store the handle to the main window.
    ///
    /// # Returns
    ///
    /// * `BOOL`: A boolean value indicating whether the enumeration should continue.
    ///   - `TRUE`: The enumeration should continue.
    ///   - `FALSE`: The enumeration should stop. In this case, the function has found the main window and stored its handle in `lparam`.
    unsafe extern "system" fn enum_window(window: HWND, lparam: LPARAM) -> BOOL {
        let mut window_proc_id = 0;
        let _ = GetWindowThreadProcessId(window, Some(&mut window_proc_id));

        if GetCurrentProcessId() != window_proc_id
            || !is_main_window(window)
            || window == GetConsoleWindow()
        {
            return TRUE;
        }

        let lparam_ptr = lparam.0 as *mut HWND;

        std::ptr::write(lparam_ptr, window);

        FALSE
    }

    /// Finds the main window of the current process.
    ///
    /// This function uses the `EnumWindows` function to iterate through all top-level windows in the system.
    /// It then checks each window to determine if it is the main window of the current process.
    /// The main window is defined as a visible window without an owner and is not the console window.
    ///
    /// # Returns
    ///
    /// * `Some(HWND)` - If the main window is found, the function returns the handle to the main window.
    /// * `None` - If no main window is found, the function returns `None`.
    pub fn find_window() -> Option<HWND> {
        let mut hwnd: HWND = Default::default();

        let _ = unsafe {
            EnumWindows(Some(enum_window), LPARAM(std::ptr::from_mut::<HWND>(&mut hwnd) as isize))
        };

        hwnd.into()
    }
}

/// Finds the main window of the current process.
///
/// # Returns
///
/// * `Some(Handle)` - If the main window is found, the function returns the handle to the main window.
/// * `None` - If no main window is found, the function returns `None`.
pub fn find_window() -> Option<Handle> {
    #[cfg(windows)]
    return windows::find_window().map(|hwnd| hwnd.into());
    #[cfg(not(windows))]
    todo!()
}
