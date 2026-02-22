#![windows_subsystem = "windows"]
use eframe::egui;
use egui::{Color32, TextureHandle, Vec2};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::thread;
use vnc::{Encoding, PixelFormat, Rect};

mod keys;

#[derive(Clone, Copy, PartialEq)]
enum AppState {
    Connect,
    Viewing,
}

#[derive(Serialize, Deserialize)]
struct Config {
    host: String,
    port: String,
    password: String,
    shared: bool,
    view_only: bool,
    zoom_fit: bool,
    scale: f32,
    preferred_encoding: String,
    compression_level: u8,
    quality_level: u8,
    allow_copyrect: bool,
    disable_clipboard: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
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

struct VncApp {
    state: AppState,

    // Connection params
    host: String,
    port: String,
    password: String,
    shared: bool,

    // VNC Client
    vnc_client: Option<vnc::Client>,
    vnc_rx: Option<std::sync::mpsc::Receiver<Result<vnc::Client, String>>>,

    // Screen data
    screen_texture: Option<TextureHandle>,
    screen_size: (u16, u16),
    pixels: Vec<Color32>,

    // Icons
    icons: std::collections::HashMap<String, TextureHandle>,

    // Status
    status_text: String,

    // Options
    view_only: bool,
    zoom_fit: bool,
    scale: f32,
    preferred_encoding: String,
    compression_level: u8,
    quality_level: u8,
    allow_copyrect: bool,
    disable_clipboard: bool,

    // Input throttling
    last_pointer_pos: Option<(u16, u16)>,
    last_buttons: u8,

    // Dialogs
    show_options: bool,
    show_info: bool,
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

        let app = Self {
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
        };
        app
    }
}

fn setup_custom_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Premium dark theme
    style.visuals = egui::Visuals::dark();
    style.visuals.window_rounding = 12.0.into();
    style.visuals.window_shadow.extrusion = 20.0;

    // Widget colors
    style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(20, 20, 25);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 55);
    style.visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(200, 200, 210));

    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(60, 60, 80);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, Color32::WHITE);

    style.visuals.widgets.active.bg_fill = Color32::from_rgb(0, 110, 200);

    // Spacing
    style.spacing.item_spacing = Vec2::new(12.0, 12.0);
    style.spacing.window_margin = egui::Margin::same(24.0);
    style.spacing.button_padding = Vec2::new(16.0, 8.0);

    ctx.set_style(style);
}

fn get_app_icon() -> Option<eframe::IconData> {
    if let Ok(image_data) = std::fs::read("assets/app-icon.png") {
        if let Ok(image) = image::load_from_memory(&image_data) {
            let image = image.to_rgba8();
            let (width, height) = image.dimensions();
            return Some(eframe::IconData {
                rgba: image.into_raw(),
                width,
                height,
            });
        }
    }
    None
}

impl VncApp {
    fn load_icons(&mut self, ctx: &egui::Context) {
        let icon_data: [(&str, &[u8]); 10] = [
            (
                "button-options",
                include_bytes!("../assets/svg/button-options.svg"),
            ),
            (
                "button-info",
                include_bytes!("../assets/svg/button-info.svg"),
            ),
            (
                "button-refresh",
                include_bytes!("../assets/svg/button-refresh.svg"),
            ),
            (
                "button-zoom-out",
                include_bytes!("../assets/svg/button-zoom-out.svg"),
            ),
            (
                "button-zoom-in",
                include_bytes!("../assets/svg/button-zoom-in.svg"),
            ),
            (
                "button-zoom-100",
                include_bytes!("../assets/svg/button-zoom-100.svg"),
            ),
            (
                "button-zoom-fit",
                include_bytes!("../assets/svg/button-zoom-fit.svg"),
            ),
            (
                "button-zoom-fullscreen",
                include_bytes!("../assets/svg/button-zoom-fullscreen.svg"),
            ),
            (
                "button-ctrl-alt-del",
                include_bytes!("../assets/svg/button-ctrl-alt-del.svg"),
            ),
            ("button-win", include_bytes!("../assets/svg/button-win.svg")),
        ];

        for (name, data) in icon_data {
            match egui_extras::image::load_svg_bytes(data) {
                Ok(color_image) => {
                    let handle = ctx.load_texture(name, color_image, Default::default());
                    self.icons.insert(name.to_string(), handle);
                }
                Err(e) => warn!("Failed to load embedded SVG {}: {}", name, e),
            }
        }
    }

    fn connect(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.vnc_rx = Some(rx);

        let host = self.host.clone();
        let port_str = self.port.clone();
        let password = self.password.clone();
        let shared = self.shared;

        self.status_text = format!("Connecting to {}:{}...", host, port_str);

        // Save config
        let config = Config {
            host: self.host.clone(),
            port: self.port.clone(),
            password: self.password.clone(),
            shared: self.shared,
            view_only: self.view_only,
            zoom_fit: self.zoom_fit,
            scale: self.scale,
            preferred_encoding: self.preferred_encoding.clone(),
            compression_level: self.compression_level,
            quality_level: self.quality_level,
            allow_copyrect: self.allow_copyrect,
            disable_clipboard: self.disable_clipboard,
        };
        if let Ok(content) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write("vnc_config.json", content);
        }

        thread::spawn(move || {
            let port: u16 = port_str.parse().unwrap_or(5900);
            let addr = format!("{}:{}", host, port);
            match std::net::TcpStream::connect(&addr) {
                Ok(stream) => {
                    let client = vnc::Client::from_tcp_stream(stream, shared, |methods| {
                        for method in methods {
                            match method {
                                vnc::client::AuthMethod::None => {
                                    return Some(vnc::client::AuthChoice::None);
                                }
                                vnc::client::AuthMethod::Password => {
                                    let mut pw = [0u8; 8];
                                    for (i, b) in password.as_bytes().iter().take(8).enumerate() {
                                        pw[i] = *b;
                                    }
                                    return Some(vnc::client::AuthChoice::Password(pw));
                                }
                                _ => continue,
                            }
                        }
                        None
                    });

                    match client {
                        Ok(vnc) => {
                            let _ = tx.send(Ok(vnc));
                        }
                        Err(e) => {
                            let err_msg = format!("VNC Init Error: {}", e);
                            error!("{}", err_msg);
                            let _ = tx.send(Err(err_msg));
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("Connect Error: {}", e);
                    error!("{}", err_msg);
                    let _ = tx.send(Err(err_msg));
                }
            }
        });
    }

    fn handle_vnc_events(&mut self, ctx: &egui::Context) {
        // Check for new connection
        if let Some(ref rx) = self.vnc_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(mut vnc) => {
                        let (w, h) = vnc.size();
                        info!("Connected: {}x{}", w, h);

                        vnc.set_encodings(&[
                            Encoding::Zrle,
                            Encoding::CopyRect,
                            Encoding::Raw,
                            Encoding::Cursor,
                            Encoding::DesktopSize,
                        ])
                        .unwrap();

                        vnc.request_update(
                            Rect {
                                left: 0,
                                top: 0,
                                width: w,
                                height: h,
                            },
                            false,
                        )
                        .unwrap();

                        self.screen_size = (w, h);
                        self.pixels = vec![Color32::BLACK; (w as usize) * (h as usize)];
                        self.vnc_client = Some(vnc);
                        self.state = AppState::Viewing;
                        self.status_text = "Connected".to_string();
                    }
                    Err(e) => {
                        self.status_text = e;
                    }
                }
                self.vnc_rx = None;
            }
        }

        if let Some(mut vnc) = self.vnc_client.take() {
            let mut updated = false;

            while let Some(event) = vnc.poll_event() {
                match event {
                    vnc::client::Event::Disconnected(e) => {
                        error!("Disconnected: {:?}", e);
                        self.state = AppState::Connect;
                        self.vnc_client = None;
                        return;
                    }
                    vnc::client::Event::Resize(w, h) => {
                        info!("Resize: {}x{}", w, h);
                        self.screen_size = (w, h);
                        self.pixels = vec![Color32::BLACK; (w as usize) * (h as usize)];
                        updated = true;
                    }
                    vnc::client::Event::PutPixels(rect, pixels) => {
                        let format = vnc.format();
                        self.update_pixels(rect, &pixels, format);
                        updated = true;
                    }
                    vnc::client::Event::CopyPixels { src, dst } => {
                        self.copy_pixels(src, dst);
                        updated = true;
                    }
                    vnc::client::Event::EndOfFrame => {
                        ctx.request_repaint();
                        vnc.request_update(
                            Rect {
                                left: 0,
                                top: 0,
                                width: self.screen_size.0,
                                height: self.screen_size.1,
                            },
                            true,
                        )
                        .unwrap();
                    }
                    _ => {}
                }
            }

            if updated {
                self.update_texture(ctx);
                ctx.request_repaint();
            }
            self.vnc_client = Some(vnc);
        }
    }

    fn copy_pixels(&mut self, src: Rect, dst: Rect) {
        let width = src.width as usize;
        let height = src.height as usize;
        let screen_w = self.screen_size.0 as usize;

        if dst.top < src.top {
            // Copy from top to bottom
            for y in 0..height {
                let src_y = src.top as usize + y;
                let dst_y = dst.top as usize + y;
                for x in 0..width {
                    let src_idx = src_y * screen_w + (src.left as usize + x);
                    let dst_idx = dst_y * screen_w + (dst.left as usize + x);
                    self.pixels[dst_idx] = self.pixels[src_idx];
                }
            }
        } else {
            // Copy from bottom to top to handle overlap correctly
            for y in (0..height).rev() {
                let src_y = src.top as usize + y;
                let dst_y = dst.top as usize + y;
                for x in 0..width {
                    let src_idx = src_y * screen_w + (src.left as usize + x);
                    let dst_idx = dst_y * screen_w + (dst.left as usize + x);
                    self.pixels[dst_idx] = self.pixels[src_idx];
                }
            }
        }
    }

    fn update_pixels(&mut self, rect: Rect, pixels: &[u8], format: PixelFormat) {
        let bpp = format.bits_per_pixel as usize / 8;
        let mut i = 0;

        let r_max = format.red_max as u32;
        let g_max = format.green_max as u32;
        let b_max = format.blue_max as u32;

        for y in 0..rect.height {
            let row_start =
                ((rect.top + y) as usize * self.screen_size.0 as usize) + rect.left as usize;
            for x in 0..rect.width {
                let pixel_idx = row_start + x as usize;
                if pixel_idx < self.pixels.len() && i + bpp <= pixels.len() {
                    let val = match bpp {
                        1 => pixels[i] as u32,
                        2 => {
                            if format.big_endian {
                                (pixels[i] as u32) << 8 | (pixels[i + 1] as u32)
                            } else {
                                (pixels[i + 1] as u32) << 8 | (pixels[i] as u32)
                            }
                        }
                        4 => {
                            if format.big_endian {
                                (pixels[i] as u32) << 24
                                    | (pixels[i + 1] as u32) << 16
                                    | (pixels[i + 2] as u32) << 8
                                    | (pixels[i + 3] as u32)
                            } else {
                                (pixels[i + 3] as u32) << 24
                                    | (pixels[i + 2] as u32) << 16
                                    | (pixels[i + 1] as u32) << 8
                                    | (pixels[i] as u32)
                            }
                        }
                        _ => 0,
                    };
                    i += bpp;

                    let r_raw = (val >> format.red_shift) & r_max;
                    let g_raw = (val >> format.green_shift) & g_max;
                    let b_raw = (val >> format.blue_shift) & b_max;

                    let r = if r_max == 255 {
                        r_raw as u8
                    } else if r_max > 0 {
                        (r_raw * 255 / r_max) as u8
                    } else {
                        0
                    };
                    let g = if g_max == 255 {
                        g_raw as u8
                    } else if g_max > 0 {
                        (g_raw * 255 / g_max) as u8
                    } else {
                        0
                    };
                    let b = if b_max == 255 {
                        b_raw as u8
                    } else if b_max > 0 {
                        (b_raw * 255 / b_max) as u8
                    } else {
                        0
                    };

                    self.pixels[pixel_idx] = Color32::from_rgb(r, g, b);
                }
            }
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if self.pixels.is_empty() {
            return;
        }

        let size = [self.screen_size.0 as usize, self.screen_size.1 as usize];
        let color_image = egui::ColorImage {
            size,
            pixels: self.pixels.clone(),
        };

        if let Some(ref mut handle) = self.screen_texture {
            handle.set(color_image, Default::default());
        } else {
            self.screen_texture =
                Some(ctx.load_texture("vnc_screen", color_image, Default::default()));
        }
    }

    fn handle_input(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if self.view_only {
            return;
        }

        let Some(ref mut vnc) = self.vnc_client else {
            return;
        };

        // Mouse motion and clicks
        if response.hovered() {
            if let Some(pos) = response.hover_pos() {
                let rect = response.rect;
                let x = (((pos.x - rect.min.x) / rect.width()) * self.screen_size.0 as f32) as u16;
                let y = (((pos.y - rect.min.y) / rect.height()) * self.screen_size.1 as f32) as u16;

                let mut buttons = 0u8;
                ui.input(|i| {
                    if i.pointer.button_down(egui::PointerButton::Primary) {
                        buttons |= 0x01;
                    }
                    if i.pointer.button_down(egui::PointerButton::Middle) {
                        buttons |= 0x02;
                    }
                    if i.pointer.button_down(egui::PointerButton::Secondary) {
                        buttons |= 0x04;
                    }
                });
                if self.last_pointer_pos != Some((x, y)) || self.last_buttons != buttons {
                    vnc.send_pointer_event(buttons, x, y).unwrap();
                    self.last_pointer_pos = Some((x, y));
                    self.last_buttons = buttons;
                }
            }
        }

        // Keyboard
        ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Key { key, pressed, .. } => {
                        if let Some(keysym) = keys::map_key(*key) {
                            vnc.send_key_event(*pressed, keysym).unwrap();
                        }
                    }
                    egui::Event::Text(text) => {
                        for c in text.chars() {
                            let keysym = 0x01000000 + c as u32;
                            vnc.send_key_event(true, keysym).unwrap();
                            vnc.send_key_event(false, keysym).unwrap();
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}

impl eframe::App for VncApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        setup_custom_style(ctx);
        if self.icons.is_empty() {
            self.load_icons(ctx);
        }

        self.handle_vnc_events(ctx);

        match self.state {
            AppState::Connect => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::from_rgb(15, 15, 18)))
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0); // Add a small margin from the top instead of large offset

                            // Card UI
                            egui::Frame::window(&ui.style())
                                .fill(Color32::from_rgb(30, 30, 35))
                                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 70)))
                                .show(ui, |ui| {
                                    ui.set_width(400.0);
                                    ui.add_space(10.0);

                                    ui.vertical_centered(|ui| {
                                        ui.heading(
                                            egui::RichText::new("VNC Remote Desktop")
                                                .size(24.0)
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("Connect to your remote session")
                                                .color(Color32::from_rgb(150, 150, 160)),
                                        );
                                    });

                                    ui.add_space(20.0);

                                    egui::Grid::new("connect_grid")
                                        .num_columns(2)
                                        .spacing([15.0, 15.0])
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new("Remote Host:").strong());
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.host)
                                                    .hint_text("127.0.0.1"),
                                            );
                                            ui.end_row();

                                            ui.label(egui::RichText::new("Port:").strong());
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.port)
                                                    .hint_text("5900"),
                                            );
                                            ui.end_row();

                                            ui.label(egui::RichText::new("Password:").strong());
                                            ui.add(
                                                egui::TextEdit::singleline(&mut self.password)
                                                    .password(true)
                                                    .hint_text("Optional"),
                                            );
                                            ui.end_row();
                                        });

                                    ui.add_space(15.0);
                                    ui.checkbox(&mut self.shared, "Request shared session");

                                    ui.add_space(25.0);

                                    ui.vertical_centered_justified(|ui| {
                                        let connect_btn = ui.add_sized(
                                            [ui.available_width(), 40.0],
                                            egui::Button::new(
                                                egui::RichText::new("Connect Now")
                                                    .size(16.0)
                                                    .strong(),
                                            )
                                            .fill(Color32::from_rgb(0, 120, 215)),
                                        );

                                        if connect_btn.clicked() {
                                            self.connect();
                                        }
                                    });

                                    ui.add_space(20.0);
                                    ui.horizontal(|ui| {
                                        ui.style_mut().visuals.widgets.inactive.bg_fill =
                                            Color32::from_rgb(40, 40, 50);
                                        if ui.button("Options...").clicked() {
                                            self.show_options = true;
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("Close").clicked() {
                                                    frame.close();
                                                }
                                                if ui.button("Clear history").clicked() {
                                                    let _ = std::fs::remove_file("vnc_config.json");
                                                    self.host = "localhost".to_string();
                                                    self.port = "5900".to_string();
                                                    self.password = String::new();
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(10.0);
                                });
                        });
                    });

                egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                    ui.label(&self.status_text);
                });
            }
            AppState::Viewing => {
                egui::TopBottomPanel::top("toolbar")
                    .frame(egui::Frame::none().fill(Color32::from_rgb(10, 10, 12)))
                    .show(ctx, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                        ui.spacing_mut().button_padding = Vec2::new(4.0, 4.0);
                        ui.horizontal(|ui| {
                            if let Some(icon) = self.icons.get("button-info") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Info")
                                    .clicked()
                                {
                                    self.show_info = !self.show_info;
                                }
                            } else if ui.button("ℹ").on_hover_text("Info").clicked() {
                                self.show_info = !self.show_info;
                            }

                            if let Some(icon) = self.icons.get("button-refresh") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Refresh")
                                    .clicked()
                                {
                                    if let Some(ref mut vnc) = self.vnc_client {
                                        let _ = vnc.request_update(
                                            Rect {
                                                left: 0,
                                                top: 0,
                                                width: self.screen_size.0,
                                                height: self.screen_size.1,
                                            },
                                            false,
                                        );
                                    }
                                }
                            } else if ui.button("🔄").on_hover_text("Refresh").clicked() {
                                if let Some(ref mut vnc) = self.vnc_client {
                                    let _ = vnc.request_update(
                                        Rect {
                                            left: 0,
                                            top: 0,
                                            width: self.screen_size.0,
                                            height: self.screen_size.1,
                                        },
                                        false,
                                    );
                                }
                            }

                            ui.add(egui::Separator::default().vertical().spacing(2.0));

                            if let Some(icon) = self.icons.get("button-zoom-out") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Zoom Out")
                                    .clicked()
                                {
                                    self.scale *= 0.8;
                                    self.zoom_fit = false;
                                    ctx.request_repaint();
                                }
                            } else if ui.button("➖").on_hover_text("Zoom Out").clicked() {
                                self.scale *= 0.8;
                                self.zoom_fit = false;
                            }

                            if let Some(icon) = self.icons.get("button-zoom-in") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Zoom In")
                                    .clicked()
                                {
                                    self.scale *= 1.25;
                                    self.zoom_fit = false;
                                    ctx.request_repaint();
                                }
                            } else if ui.button("➕").on_hover_text("Zoom In").clicked() {
                                self.scale *= 1.25;
                                self.zoom_fit = false;
                            }

                            if let Some(icon) = self.icons.get("button-zoom-100") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Zoom 100%")
                                    .clicked()
                                {
                                    self.scale = 1.0;
                                    self.zoom_fit = false;
                                    ctx.request_repaint();
                                }
                            } else if ui.button("1:1").on_hover_text("Zoom 100%").clicked() {
                                self.scale = 1.0;
                                self.zoom_fit = false;
                            }

                            if let Some(icon) = self.icons.get("button-zoom-fit") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Zoom to Fit")
                                    .clicked()
                                {
                                    self.zoom_fit = !self.zoom_fit;
                                    ctx.request_repaint();
                                }
                            } else if ui.button("⛶").on_hover_text("Zoom to Fit").clicked() {
                                self.zoom_fit = !self.zoom_fit;
                            }

                            if let Some(icon) = self.icons.get("button-zoom-fullscreen") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Full Screen")
                                    .clicked()
                                {
                                    let fullscreen = frame.info().window_info.fullscreen;
                                    frame.set_fullscreen(!fullscreen);
                                }
                            } else if ui.button("Full").on_hover_text("Full Screen").clicked() {
                                let fullscreen = frame.info().window_info.fullscreen;
                                frame.set_fullscreen(!fullscreen);
                            }

                            ui.add(egui::Separator::default().vertical().spacing(2.0));

                            if let Some(icon) = self.icons.get("button-ctrl-alt-del") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Send Ctrl-Alt-Del")
                                    .clicked()
                                {
                                    if let Some(ref mut vnc) = self.vnc_client {
                                        let _ = vnc.send_key_event(true, 0xFFE3); // Ctrl
                                        let _ = vnc.send_key_event(true, 0xFFE9); // Alt
                                        let _ = vnc.send_key_event(true, 0xFFFF); // Del
                                        let _ = vnc.send_key_event(false, 0xFFFF);
                                        let _ = vnc.send_key_event(false, 0xFFE9);
                                        let _ = vnc.send_key_event(false, 0xFFE3);
                                    }
                                }
                            } else if ui
                                .button("CAD")
                                .on_hover_text("Send Ctrl-Alt-Del")
                                .clicked()
                            {
                                if let Some(ref mut vnc) = self.vnc_client {
                                    let _ = vnc.send_key_event(true, 0xFFE3); // Ctrl
                                    let _ = vnc.send_key_event(true, 0xFFE9); // Alt
                                    let _ = vnc.send_key_event(true, 0xFFFF); // Del
                                    let _ = vnc.send_key_event(false, 0xFFFF);
                                    let _ = vnc.send_key_event(false, 0xFFE9);
                                    let _ = vnc.send_key_event(false, 0xFFE3);
                                }
                            }

                            if let Some(icon) = self.icons.get("button-win") {
                                if ui
                                    .add(
                                        egui::ImageButton::new(icon, Vec2::splat(18.0))
                                            .tint(Color32::WHITE),
                                    )
                                    .on_hover_text("Send Win Key")
                                    .clicked()
                                {
                                    if let Some(ref mut vnc) = self.vnc_client {
                                        let _ = vnc.send_key_event(true, 0xFFE3); // Ctrl
                                        let _ = vnc.send_key_event(true, 0xFF1B); // Esc
                                        let _ = vnc.send_key_event(false, 0xFF1B);
                                        let _ = vnc.send_key_event(false, 0xFFE3);
                                    }
                                }
                            } else if ui.button("Win").on_hover_text("Send Win Key").clicked() {
                                if let Some(ref mut vnc) = self.vnc_client {
                                    let _ = vnc.send_key_event(true, 0xFFE3); // Ctrl
                                    let _ = vnc.send_key_event(true, 0xFF1B); // Esc
                                    let _ = vnc.send_key_event(false, 0xFF1B);
                                    let _ = vnc.send_key_event(false, 0xFFE3);
                                }
                            }

                            // Move right-aligned items into the SAME horizontal row
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if let Some(icon) = self.icons.get("button-options") {
                                        let is_active = self.show_options;
                                        let button =
                                            egui::ImageButton::new(icon, Vec2::splat(18.0))
                                                .tint(Color32::WHITE)
                                                .selected(is_active)
                                                .tint(if is_active {
                                                    Color32::from_rgb(0, 150, 255)
                                                } else {
                                                    Color32::WHITE
                                                });

                                        if ui
                                            .add(button)
                                            .on_hover_text("Connection Options")
                                            .clicked()
                                        {
                                            self.show_options = !self.show_options;
                                        }
                                    } else if ui
                                        .button("Opt")
                                        .on_hover_text("Connection Options")
                                        .clicked()
                                    {
                                        self.show_options = !self.show_options;
                                    }
                                    ui.add(egui::Separator::default().vertical().spacing(2.0));
                                    ui.label(format!(
                                        "Scale: {:.2} {}",
                                        self.scale,
                                        if self.zoom_fit { "(Fit)" } else { "" }
                                    ));
                                },
                            );
                        });
                    });

                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::none().fill(
                            ctx.style()
                                .visuals
                                .dark_mode
                                .then(|| Color32::from_rgb(30, 30, 30))
                                .unwrap_or(Color32::WHITE),
                        ),
                    )
                    .show(ctx, |ui| {
                        let available_size = ui.available_size();
                        let texture_size =
                            Vec2::new(self.screen_size.0 as f32, self.screen_size.1 as f32);

                        let display_size = if self.zoom_fit {
                            let ratio = (available_size.x / texture_size.x)
                                .min(available_size.y / texture_size.y);
                            texture_size * ratio.max(0.1)
                        } else {
                            texture_size * self.scale.max(0.1)
                        };

                        egui::ScrollArea::both()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                // Center the image in the available space
                                let (rect, _response) = ui.allocate_at_least(
                                    Vec2::new(
                                        display_size.x.max(ui.available_width()),
                                        display_size.y.max(ui.available_height()),
                                    ),
                                    egui::Sense::hover(),
                                );

                                let image_rect = egui::Rect::from_min_size(rect.min, display_size);

                                // We need a response specifically for the image area for input
                                let image_response = ui.interact(
                                    image_rect,
                                    ui.id().with("vnc_img"),
                                    egui::Sense::click_and_drag(),
                                );
                                self.handle_input(ui, &image_response);

                                if let Some(ref texture) = self.screen_texture {
                                    let mut mesh = egui::Mesh::with_texture(texture.id());
                                    mesh.add_rect_with_uv(
                                        image_rect,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );
                                    ui.painter().add(egui::Shape::mesh(mesh));
                                } else {
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "Waiting for first frame...",
                                        egui::FontId::proportional(20.0),
                                        ui.visuals().text_color(),
                                    );
                                }
                            });
                    });
            }
        }

        if self.show_options && self.state == AppState::Viewing {
            egui::SidePanel::right("options_panel")
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.heading("Connection Options");
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Format and Encodings").strong());
                            ui.separator();

                            egui::Grid::new("enc_grid").num_columns(2).show(ui, |ui| {
                                ui.label("Preferred encoding:");
                                egui::ComboBox::from_id_source("encoding_pref")
                                    .selected_text(&self.preferred_encoding)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.preferred_encoding,
                                            "ZRLE".to_string(),
                                            "ZRLE",
                                        );
                                        ui.selectable_value(
                                            &mut self.preferred_encoding,
                                            "Hextile".to_string(),
                                            "Hextile",
                                        );
                                        ui.selectable_value(
                                            &mut self.preferred_encoding,
                                            "Raw".to_string(),
                                            "Raw",
                                        );
                                    });
                                ui.end_row();
                            });

                            ui.add_space(10.0);
                            ui.label(format!("Compression level: {}", self.compression_level));
                            ui.add(egui::Slider::new(&mut self.compression_level, 1..=9));

                            ui.add_space(5.0);
                            ui.label(format!("JPEG quality level: {}", self.quality_level));
                            ui.add(egui::Slider::new(&mut self.quality_level, 1..=9));

                            ui.add_space(10.0);
                            ui.checkbox(&mut self.allow_copyrect, "Allow CopyRect encoding");
                        });

                        ui.add_space(10.0);
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Restrictions").strong());
                            ui.separator();
                            ui.checkbox(&mut self.view_only, "View only (inputs ignored)");
                            ui.checkbox(&mut self.disable_clipboard, "Disable clipboard transfer");
                        });

                        ui.add_space(10.0);
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Display").strong());
                            ui.separator();
                            ui.checkbox(&mut !(self.zoom_fit), "Scale to window size");
                            ui.add(
                                egui::Slider::new(&mut self.scale, 0.1..=4.0).text("Manual Scale"),
                            );
                        });
                    });

                    ui.add_space(20.0);
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() {
                                // Apply encoding settings if connected
                                if let Some(ref mut vnc) = self.vnc_client {
                                    let mut encs = Vec::new();
                                    match self.preferred_encoding.as_str() {
                                        "ZRLE" => encs.push(Encoding::Zrle),
                                        "Hextile" => encs.push(Encoding::Hextile),
                                        _ => (),
                                    }
                                    if self.allow_copyrect {
                                        encs.push(Encoding::CopyRect);
                                    }
                                    encs.push(Encoding::Raw);
                                    encs.push(Encoding::Cursor);
                                    encs.push(Encoding::DesktopSize);
                                    let _ = vnc.set_encodings(&encs);
                                }
                            }
                            if ui.button("Close").clicked() {
                                self.show_options = false;
                            }
                        });
                    });
                });
        }

        // Only show floating options during Connect state fallback
        if self.show_options && self.state == AppState::Connect {
            egui::Window::new("Options")
                .collapsible(false)
                .resizable(false)
                .fixed_size([300.0, 400.0])
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.view_only, "View-only mode");
                    ui.checkbox(&mut self.zoom_fit, "Scale to window size");
                    if ui.button("Close").clicked() {
                        self.show_options = false;
                    }
                });
        }

        if self.show_info {
            egui::Window::new("Connection Info").show(ctx, |ui| {
                ui.label(format!("Host: {}", self.host));
                ui.label(format!(
                    "Resolution: {}x{}",
                    self.screen_size.0, self.screen_size.1
                ));
                if let Some(ref vnc) = self.vnc_client {
                    ui.label(format!("Name: {}", vnc.name()));
                }
                if ui.button("Close").clicked() {
                    self.show_info = false;
                }
            });
        }
    }
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        icon_data: get_app_icon(),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "VNC Remote Desktop",
        options,
        Box::new(|_cc| Box::new(VncApp::default())),
    );
}
