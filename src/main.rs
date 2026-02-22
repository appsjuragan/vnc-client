#![windows_subsystem = "windows"]

mod app;
mod config;
mod keys;

use app::{get_app_icon, VncApp};

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(eframe::egui::vec2(800.0, 600.0)),
        icon_data: get_app_icon(),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "VNC Remote Desktop",
        options,
        Box::new(|_cc| Box::new(VncApp::default())),
    );
}
