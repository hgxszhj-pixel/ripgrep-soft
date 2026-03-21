#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::Parser;
use turbo_search::cli::{Cli, Commands};
use turbo_search::cli_search;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    turbo_search::logging::init(cli.log_level())?;

    // Check if we should launch GUI or run CLI command
    if cli.should_launch_gui() {
        #[cfg(windows)]
        hide_console();
        gui::run_gui()?;
    } else if let Some(command) = cli.command {
        match command {
            Commands::Search {
                path,
                pattern,
                content,
                regex,
                glob,
                case_sensitive,
                context,
                limit,
            } => {
                cli_search::run_search(
                    path,
                    pattern,
                    content,
                    regex,
                    glob,
                    case_sensitive,
                    context,
                    limit,
                )?;
            }
            Commands::Index { path, rebuild } => {
                cli_search::run_index(path, rebuild)?;
            }
        }
    }

    Ok(())
}
