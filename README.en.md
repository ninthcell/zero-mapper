# ZeroMapper

[한국어](README.md) | **English** | [日本語](README.ja.md)

[![Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/ninthcell)

A lightweight Windows tray app that turns compact controllers like the 8BitDo Zero or 8BitDo Micro into a **shortcut pad for digital art**.

It detects the active window and automatically switches button mappings per app — Clip Studio, Photoshop, Krita, and more.

<video src="https://github.com/user-attachments/assets/9e17446c-73cf-4b7a-b0ac-685888d883c0" autoplay loop muted playsinline></video>

---

## Why ZeroMapper

**Near-zero CPU usage. ~2 MB memory.**

Background apps shouldn't waste resources when idle. ZeroMapper slows polling to 150 ms when no buttons are pressed, and stops completely when the screen is locked or the system sleeps. Leave it running all day without impact on battery or performance.

- No installer — just an exe and a config.toml
- Automatic profile switching based on the foreground window title
- **Button combos** — map combos like `LB+A`, `RB+DpadLeft` to a single keyboard shortcut. Cover dozens of shortcuts even on tiny controllers
- Tap and hold output modes
- **Overlay HUD** — semi-transparent on-screen overlay showing current mappings and pressed buttons in real time

---

## Installation

1. Download the latest zip from [Releases](../../releases)
2. Extract anywhere (`ZeroMapper.exe` and `config.toml` are inside)
3. Run `ZeroMapper.exe` — a tray icon will appear

> **8BitDo XInput mode**: Hold **X** and press **START** to power on. On models with a mode switch, set it to the **X** position.

---

## Nintendo Layout (A↔B, X↔Y Swap)

Controllers with a Nintendo button layout (8BitDo Zero, Micro, etc.) report swapped face buttons over XInput.

```
        Xbox layout              Nintendo layout (8BitDo)

           [Y]                        [X]
         [X] [B]                    [Y] [A]
           [A]                        [B]
```

The bundled config **defaults to Nintendo layout (`nintendo_layout = true`) based on the 8BitDo Zero 2**. This lets you write mappings using the **printed button labels** on your controller.

```toml
schema_version = 1
controller_player = 1
nintendo_layout = true   # For 8BitDo Zero/Micro and other Nintendo-layout controllers
```

If you're using an Xbox controller, set `nintendo_layout = false` or remove the line.

---

## Default Mappings

The bundled `config.toml` includes profiles for Clip Studio Paint, Photoshop, Aseprite, and Krita.

### Clip Studio Paint

| Button | Key | Action |
|--------|-----|--------|
| A | P | Pen |
| Y | B | Brush |
| B | E | Eraser |
| X (hold) | Space | Pan canvas |
| DpadLeft | Ctrl+Z | Undo |
| DpadRight | Ctrl+Y | Redo |
| LB+Y | I | Color picker |
| LB+A | M | Selection tool |
| LB+B | K | Fill |
| RB+A | Ctrl+T | Transform |
| Start | Ctrl+S | Save |

### Photoshop

| Button | Key | Action |
|--------|-----|--------|
| A | B | Brush |
| Y | I | Eyedropper |
| B | E | Eraser |
| X (hold) | Space | Pan canvas |
| DpadLeft | Ctrl+Z | Undo |
| DpadRight | Ctrl+Shift+Z | Redo |
| LB+A | M | Marquee tool |
| LB+B | V | Move tool |
| LB+DpadLeft | [ | Decrease brush size |
| LB+DpadRight | ] | Increase brush size |
| RB+DpadLeft | Ctrl+- | Zoom out |
| RB+DpadRight | Ctrl+= | Zoom in |
| RB+A | Ctrl+T | Free transform |
| RB+B | X | Swap foreground/background |
| Start | Ctrl+S | Save |

### Aseprite

| Button | Key | Action |
|--------|-----|--------|
| A | B | Brush |
| B | E | Eraser |
| X (hold) | Space | Pan canvas |
| Y (hold) | Alt | Eyedropper |
| LB+Y (hold) | Ctrl | Multi-select |
| DpadLeft | Ctrl+Z | Undo |
| DpadRight | Ctrl+Y | Redo |
| LB+A | M | Selection tool |
| RB+X | Tab | Toggle UI |
| Start | Ctrl+S | Save |

### Krita

| Button | Key | Action |
|--------|-----|--------|
| A | B | Brush |
| B | E | Eraser |
| X (hold) | Space | Pan canvas |
| Y (hold) | Ctrl | Resize brush |
| DpadLeft | Ctrl+Z | Undo |
| DpadRight | Ctrl+Shift+Z | Redo |
| LB+A | Ctrl+R | Reference image |
| LB+B | V | Move tool |
| LB+DpadLeft | [ | Decrease brush size |
| LB+DpadRight | ] | Increase brush size |
| RB+B | X | Swap foreground/background |
| RB+X | D | Default colors |
| RB+Y | F5 | Brush settings |
| Start | Ctrl+S | Save |

---

## Customization

Open `config.toml` via tray menu → "Open config.toml" and edit it. Changes are applied automatically on save. You can also manually reload via tray menu → "Reload config".

```toml
schema_version = 1
controller_player = 1  # XInput player number (1–4)

[profiles.my_app]
name = "My App"           # Name shown in the tray menu
title_regex = "My App"   # Matched against the window title (regex supported)

[profiles.my_app.map]
A = "P"                               # Tap: press P once when A is pressed
X = { mode = "hold", send = "Space" } # Hold: keep Space held while X is held
"LB+A" = "Ctrl+Z"                     # Combo: LB and A together → Ctrl+Z
```

**Combo priority**: If both `LB+A` and `A` are mapped, pressing LB+A only triggers `LB+A` — `A` is suppressed.

### Supported buttons

`A` `B` `X` `Y` `LB` `RB` `LT` `RT` `Back` `Start` `L3` `R3` `DpadUp` `DpadDown` `DpadLeft` `DpadRight`

### Supported keys

`A`–`Z`, `0`–`9`, `F1`–`F24`, `Ctrl`, `Alt`, `Shift`, `Win`, `Enter`, `Esc`, `Space`, `Tab`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`, `CapsLock`, `[` `]` `\` `-` `=` `,` `.` `/` `;` `'`

---

## Overlay HUD

A semi-transparent overlay in the corner of the screen showing button mappings for the active profile.

- Displays the mapped keyboard shortcut next to each button
- Real-time highlight of pressed buttons
- Hold a single button to see its combo mappings (e.g., hold LB → shows LB+A, LB+B mappings)
- Shows "No controller" status when controller is disconnected
- Toggle visibility from the tray menu

```toml
overlay = true                     # Enable overlay (default: false)
overlay_position = "bottom-right"  # top-left / top-right / bottom-left / bottom-right
overlay_opacity = 80               # Opacity 0–100 (default: 80)
```

---

## Build

```
cargo build --release
```

Binary: `target\release\zero_mapper.exe`

---

## Limitation

ZeroMapper does not suppress the original controller input. It sends additional keyboard input on top — the underlying XInput device remains visible to other applications.
