# VNC Client (egui)

A high-performance, cross-platform VNC client written in Rust using the `egui` framework. This client provides a smooth and responsive remote desktop experience with a modern, native-feeling UI.

![VNC Client Preview](https://raw.githubusercontent.com/appsjuragan/vnc-client/master/assets/preview.png) *(Placeholder if you have an image)*

## Features

- **Modern UI**: Built with `egui` for a clean, hardware-accelerated interface.
- **Throttled Input**: Optimized mouse and keyboard event handling for low-latency interaction.
- **Multiple Encodings**: Supports ZRLE, CopyRect, Raw, and more for efficient data transfer.
- **Display Scaling**: Zoom to Fit and custom scaling options.
- **Cross-Platform**: Compiles to Windows, macOS, and Linux.
- **Persistent Config**: Remembers your connection settings and preferences.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- Dependencies for `egui` and `sdl2` (if applicable on your system)

### Running Locally

1. Clone the repository:
   ```bash
   git clone git@github.com:appsjuragan/vnc-client.git
   cd vnc-client
   ```

2. Build and run:
   ```bash
   cargo run --release
   ```

## Usage

1. Enter the **Remote Host** (IP or hostname).
2. Enter the **Port** (default is 5900).
3. Enter the **Password** if required.
4. Click **Connect**.
5. Use the toolbar at the top to adjust scaling, refresh the screen, or send special keys like `Ctrl-Alt-Del`.

## Development

The project is structured with a local `vnc-lib` which contains the core VNC protocol implementation.

- `src/main.rs`: The main application logic and UI.
- `vnc-lib/`: Core protocol handling.
- `assets/`: UI icons and SVGs.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
