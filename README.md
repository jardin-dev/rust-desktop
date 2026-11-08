# rustdesktop — desktop widget

**English** · [Français](README.fr.md)

Native desktop widget written in Rust with egui/eframe, inspired by the Rainmeter
skin **Cantarell** for Windows: analog clock, audio visualizer and side shadow
bands, without Rainmeter.

The project is made of three borderless, transparent windows anchored at the
bottom of the window stack:

| Window                   | Role                                                   |
| ------------------------ | ------------------------------------------------------ |
| `rustdesktop`            | Analog clock, date and glass effect                    |
| `rustdesktop visualizer` | 256-band audio spectrum                                |
| `rustdesktop bar`        | Full-screen left and right shadow gradients (`Bar.ini`) |

A fourth window, `rustdesktop host`, stays invisible (1 × 1 pixel off-screen):
it runs the event loop and owns the three visible viewports.

## Features

- **Clock**: dial, hour/minute/second hands, date block and glass texture, using
  the skin's fonts (Arca Majora 2 Heavy + Segoe UI Light).
- **Audio visualizer**: WASAPI loopback capture of the default output, 8192-point
  FFT, 256 logarithmically spaced bands between 15 Hz and 24 kHz, instant attack
  and exponential decay.
- **Shadow bands**: 50 % black → transparent gradients on both sides of the screen.
- **Desktop integration**: undecorated windows, always at the bottom, hidden from
  the taskbar and Alt+Tab; the bar and the visualizer let clicks through.
- **Dragging**: the clock can be dragged with the mouse; its position and the
  visualizer's are saved automatically.
- **Win + D**: widgets stay visible when "Show desktop" is triggered.
- **Wake from sleep**: Win32/DWM frames are checked and repaired automatically if
  Windows redraws them after resuming.
- **Demo mode**: `--demo` animates the visualizer without audio access.
- **Single instance**: a second launch exits immediately.

## Requirements

- Windows 10 or 11.
- A recent stable Rust toolchain (edition 2024, Rust ≥ 1.85).
- A working OpenGL driver (rendering uses the `glow` backend).
- For the visualizer: a default audio output in 32-bit float format (WASAPI
  shared mode is used as-is). Without it, the bands stay at zero.

## Building

```powershell
cd app
cargo build --release
```

The binary is produced at `app\target\release\rustdesktop.exe`.

To run it directly:

```powershell
cargo run --release
cargo run --release -- --demo   # visualizer without audio
```

## Startup installation

```powershell
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\rustdesktop" | Out-Null
Copy-Item app\target\release\rustdesktop.exe "$env:LOCALAPPDATA\rustdesktop\rustdesktop.exe"
```

Then create a shortcut to `%LOCALAPPDATA%\rustdesktop\rustdesktop.exe` in the
`shell:startup` folder to launch it automatically at sign-in.

## Usage

- Drag the clock to move it.
- Right-click the clock → **Quit**.
- Positions are stored in `%APPDATA%\rustdesktop\window.json`. This file is not a
  configuration file: it is rewritten automatically.

## Environment variables (diagnostics)

| Variable                    | Effect                                          |
| --------------------------- | ----------------------------------------------- |
| `RUSTDESKTOP_NO_BAR`        | Hides the shadow bands                          |
| `RUSTDESKTOP_NO_VIZ`        | Hides the visualizer                            |
| `RUSTDESKTOP_NO_STACK`      | Disables Z-order reordering                     |
| `RUSTDESKTOP_NO_DESKTOP`    | Disables "Show desktop" handling                |
| `RUSTDESKTOP_NO_CLOCK_DRAW` | Draws the clock without its content             |
| `RUSTDESKTOP_RED`           | Semi-transparent red background (transparency test) |

## Project layout

```text
app/
├─ src/
│  ├─ main.rs        eframe loop, viewports, position persistence
│  ├─ clock.rs       dial, hands and date rendering
│  ├─ visualizer.rs  audio spectrum rendering
│  ├─ bar.rs         full-screen side gradients
│  ├─ audio.rs       WASAPI loopback capture + FFT analysis
│  ├─ wave.rs        band smoothing
│  ├─ win.rs         Win32 / DWM tweaks (frames, Z-order, sleep)
│  ├─ theme.rs       visual constants of the skin
│  ├─ state.rs       position storage
│  └─ assets.rs      texture and font loading
├─ assets/           textures and font shipped with the skin
├─ examples/         leak_probe, memory benchmark
└─ vendor/           patched copy of egui_glow
```

### `vendor/egui_glow` patch

The `vendor/egui_glow` folder contains a local copy of the dependency, enabled by
`[patch.crates-io]` in `app/Cargo.toml`. The upstream renderer reallocated its
vertex/index buffers every frame (`glBufferData`) and leaked memory with some
NVIDIA drivers. The copy allocates once and then uses `glBufferSubData` (see
`vendor/egui_glow/src/painter.rs`).

## Credits and source

This project is a Rust rewrite of the Rainmeter skin **Cantarell**, created by
[ApexXx-SenSei](https://www.deviantart.com/apexxx-sensei/art/Cantarell-770482237)
and published on 30 October 2018 on DeviantArt.

The textures (`Background.png`, `Glass.png`), the Arca Majora 2 Heavy font and the
skin's proportions come from that skin.

## Notes

- `app/target/` and `app/build.log` are not versioned.
