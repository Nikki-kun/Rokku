# Rokku

A desktop log viewer for ArduPilot and PX4 flight logs. Built with Rust and [iced](https://iced.rs/) for a native, fast UI.

## Features

- **ArduPilot Dataflash** (`.bin`) and **PX4 ULog** (`.ulg`) support
- Drag & drop or file picker to load logs
- Parameter browser with search
- Interactive charts with cursor and tooltip
- Combined chart view when multiple parameters are selected
- Clean, minimal Ferra theme

## Requirements

- [Rust](https://rustup.rs/) (1.70+)

## Build & Run

```bash
cargo build --release
cargo run --release
```

## Usage

1. Launch Rokku
2. Drag a `.bin` or `.ulg` file onto the window, or click to open the file picker
3. Expand parameter groups in the left panel and click **+** to add a chart
4. Move the cursor over charts to see values at any time point
5. Click the trash icon to remove a chart