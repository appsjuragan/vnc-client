use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct HostConfig {
    pub port: String,
    pub password: String,
    pub shared: bool,
    pub view_only: bool,
    pub zoom_fit: bool,
    pub scale: f32,
    pub preferred_encoding: String,
    pub compression_level: u8,
    pub quality_level: u8,
    pub allow_copyrect: bool,
    pub disable_clipboard: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub last_host: String,
    pub hosts: HashMap<String, HostConfig>,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            port: "5900".to_string(),
            password: "".to_string(),
            shared: true,
            view_only: false,
            zoom_fit: false,
            scale: 1.0,
            preferred_encoding: "ZRLE".to_string(),
            compression_level: 6,
            quality_level: 6,
            allow_copyrect: true,
            disable_clipboard: false,
        }
    }
}
