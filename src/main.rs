#![cfg_attr(windows, windows_subsystem = "windows")]

use turbo_search::gui;

#[cfg(windows)]
fn hide_console() {
    // Directly use Windows API to hide console window without spawning cmd
    unsafe {
        extern "system" {
            fn GetConsoleWindow() -> *mut std::ffi::c_void;
            fn ShowWindow(hwnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
        }

        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            const SW_HIDE: i32 = 0;
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    #[cfg(windows)]
    hide_console();

    gui::run_gui()
}
