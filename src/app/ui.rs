use crate::app::{AppState, VncApp};
use crate::keys;
use eframe::egui::{self, Color32, Vec2};
use log::warn;

pub fn setup_custom_style(ctx: &egui::Context) {
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

pub fn get_app_icon() -> Option<eframe::IconData> {
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
    pub fn load_icons(&mut self, ctx: &egui::Context) {
        let icon_data: [(&str, &[u8]); 10] = [
            (
                "button-options",
                include_bytes!("../../assets/svg/button-options.svg"),
            ),
            (
                "button-info",
                include_bytes!("../../assets/svg/button-info.svg"),
            ),
            (
                "button-refresh",
                include_bytes!("../../assets/svg/button-refresh.svg"),
            ),
            (
                "button-zoom-out",
                include_bytes!("../../assets/svg/button-zoom-out.svg"),
            ),
            (
                "button-zoom-in",
                include_bytes!("../../assets/svg/button-zoom-in.svg"),
            ),
            (
                "button-zoom-100",
                include_bytes!("../../assets/svg/button-zoom-100.svg"),
            ),
            (
                "button-zoom-fit",
                include_bytes!("../../assets/svg/button-zoom-fit.svg"),
            ),
            (
                "button-zoom-fullscreen",
                include_bytes!("../../assets/svg/button-zoom-fullscreen.svg"),
            ),
            (
                "button-ctrl-alt-del",
                include_bytes!("../../assets/svg/button-ctrl-alt-del.svg"),
            ),
            (
                "button-win",
                include_bytes!("../../assets/svg/button-win.svg"),
            ),
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

    pub fn handle_input(&mut self, ui: &egui::Ui, response: &egui::Response) {
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
                    let _ = vnc.send_pointer_event(buttons, x, y);
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
                            let _ = vnc.send_key_event(*pressed, keysym);
                        }
                    }
                    egui::Event::Text(text) => {
                        for c in text.chars() {
                            let keysym = 0x01000000 + c as u32;
                            let _ = vnc.send_key_event(true, keysym);
                            let _ = vnc.send_key_event(false, keysym);
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
                            ui.add_space(40.0);

                            // Card UI
                            egui::Frame::window(ui.style())
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
                                            vnc::Rect {
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
                                        vnc::Rect {
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
                    .frame(egui::Frame::none().fill(if ctx.style().visuals.dark_mode {
                        Color32::from_rgb(30, 30, 30)
                    } else {
                        Color32::WHITE
                    }))
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
                            ui.checkbox(&mut self.zoom_fit, "Scale to window size");
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
                                        "ZRLE" => encs.push(vnc::Encoding::Zrle),
                                        "Hextile" => encs.push(vnc::Encoding::Hextile),
                                        _ => (),
                                    }
                                    if self.allow_copyrect {
                                        encs.push(vnc::Encoding::CopyRect);
                                    }
                                    encs.push(vnc::Encoding::Raw);
                                    encs.push(vnc::Encoding::Cursor);
                                    encs.push(vnc::Encoding::DesktopSize);
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
