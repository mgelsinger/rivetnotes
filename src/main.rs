#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unwrap_used)]

mod app;
mod editor;
mod error;
mod logging;
mod platform;

fn main() {
    let verbose = logging::verbose_from_env();
    let _ = logging::init(verbose);
    if let Err(err) = platform::win32::run() {
        platform::win32::show_error("Rivet error", &err.to_string());
    }
}
