use crate::app::{AppState, VncApp};
use eframe::egui::{self, Color32};
use log::{error, info};
use std::thread;
use vnc::{Encoding, PixelFormat, Rect};

impl VncApp {
    pub fn connect(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.vnc_rx = Some(rx);

        let host = self.host.clone();
        let port_str = self.port.clone();
        let password = self.password.clone();
        let shared = self.shared;

        self.status_text = format!("Connecting to {}:{}...", host, port_str);

        // Save config
        self.config.last_host = self.host.clone();
        self.config.hosts.insert(
            self.host.clone(),
            crate::config::HostConfig {
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
            },
        );

        if let Ok(content) = serde_json::to_string_pretty(&self.config) {
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

    pub fn handle_vnc_events(&mut self, ctx: &egui::Context) {
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

    pub fn copy_pixels(&mut self, src: Rect, dst: Rect) {
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

    pub fn update_pixels(&mut self, rect: Rect, pixels: &[u8], format: PixelFormat) {
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

    pub fn update_texture(&mut self, ctx: &egui::Context) {
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
}
