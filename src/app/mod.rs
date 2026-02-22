use crate::config::Config;
use eframe::egui::{Color32, TextureHandle};

pub mod ui;
pub mod vnc_handler;

pub use ui::get_app_icon;

#[derive(Clone, Copy, PartialEq)]
pub enum AppState {
    Connect,
    Viewing,
}

pub struct VncApp {
    pub state: AppState,

    // Connection params
    pub host: String,
    pub port: String,
    pub password: String,
    pub shared: bool,

    // VNC Client
    pub vnc_client: Option<vnc::Client>,
    pub vnc_rx: Option<std::sync::mpsc::Receiver<Result<vnc::Client, String>>>,

    // Screen data
    pub screen_texture: Option<TextureHandle>,
    pub screen_size: (u16, u16),
    pub pixels: Vec<Color32>,

    // Icons
    pub icons: std::collections::HashMap<String, TextureHandle>,

    // Status
    pub status_text: String,

    // Options
    pub view_only: bool,
    pub zoom_fit: bool,
    pub scale: f32,
    pub preferred_encoding: String,
    pub compression_level: u8,
    pub quality_level: u8,
    pub allow_copyrect: bool,
    pub disable_clipboard: bool,

    // Input throttling
    pub last_pointer_pos: Option<(u16, u16)>,
    pub last_buttons: u8,

    // Dialogs
    pub show_options: bool,
    pub show_info: bool,
}

impl Default for VncApp {
    fn default() -> Self {
        let config = if let Ok(content) = std::fs::read_to_string("vnc_config.json") {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Config {
                host: "localhost".to_string(),
                port: "5900".to_string(),
                ..Default::default()
            }
        };

        Self {
            state: AppState::Connect,
            host: config.host,
            port: config.port,
            password: config.password,
            shared: config.shared,
            vnc_client: None,
            vnc_rx: None,
            screen_texture: None,
            screen_size: (0, 0),
            pixels: Vec::new(),
            icons: std::collections::HashMap::new(),
            status_text: "Ready".to_string(),
            view_only: config.view_only,
            zoom_fit: config.zoom_fit,
            scale: config.scale,
            preferred_encoding: config.preferred_encoding,
            compression_level: config.compression_level,
            quality_level: config.quality_level,
            allow_copyrect: config.allow_copyrect,
            disable_clipboard: config.disable_clipboard,
            last_pointer_pos: None,
            last_buttons: 0,
            show_options: false,
            show_info: false,
        }
    }
}
