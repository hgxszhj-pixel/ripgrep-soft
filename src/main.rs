#![cfg_attr(windows, windows_subsystem = "windows")]

use turbo_search::gui;

fn main() -> Result<(), eframe::Error> {
    gui::run_gui()
}
