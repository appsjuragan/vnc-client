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

    // Persistence
    pub config: Config,
}

impl Default for VncApp {
    fn default() -> Self {
        let config: Config = if let Ok(content) = std::fs::read_to_string("vnc_config.json") {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Config::default()
        };

        let host = if config.last_host.is_empty() {
            "localhost".to_string()
        } else {
            config.last_host.clone()
        };

        let host_config = config
            .hosts
            .get(&host)
            .cloned()
            .unwrap_or_else(|| crate::config::HostConfig::default());

        Self {
            state: AppState::Connect,
            host,
            port: host_config.port,
            password: host_config.password,
            shared: host_config.shared,
            vnc_client: None,
            vnc_rx: None,
            screen_texture: None,
            screen_size: (0, 0),
            pixels: Vec::new(),
            icons: std::collections::HashMap::new(),
            status_text: "Ready".to_string(),
            view_only: host_config.view_only,
            zoom_fit: host_config.zoom_fit,
            scale: host_config.scale,
            preferred_encoding: host_config.preferred_encoding,
            compression_level: host_config.compression_level,
            quality_level: host_config.quality_level,
            allow_copyrect: host_config.allow_copyrect,
            disable_clipboard: host_config.disable_clipboard,
            last_pointer_pos: None,
            last_buttons: 0,
            show_options: false,
            show_info: false,
            config,
        }
    }
}

impl VncApp {
    pub fn load_config_for_host(&mut self, host: &str) {
        if let Some(host_config) = self.config.hosts.get(host) {
            self.port = host_config.port.clone();
            self.password = host_config.password.clone();
            self.shared = host_config.shared;
            self.view_only = host_config.view_only;
            self.zoom_fit = host_config.zoom_fit;
            self.scale = host_config.scale;
            self.preferred_encoding = host_config.preferred_encoding.clone();
            self.compression_level = host_config.compression_level;
            self.quality_level = host_config.quality_level;
            self.allow_copyrect = host_config.allow_copyrect;
            self.disable_clipboard = host_config.disable_clipboard;
        }
    }
}
