#[path = "lib.rs"]
mod app_logic;

#[cfg(windows)]
mod windows_app;

#[cfg(windows)]
fn main() -> eframe::Result {
    windows_app::run()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The Factory Atlas desktop UI is currently available on Windows only.");
}
