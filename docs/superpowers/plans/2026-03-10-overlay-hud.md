# Overlay HUD Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a semi-transparent on-screen overlay that shows controller button layout with key mappings, press visualization, and dynamic combo layer switching.

**Architecture:** New `overlay.rs` module handles a WS_EX_LAYERED popup window drawn with GDI to a 32bpp ARGB bitmap. The main tick_controller calls overlay update when pressed_buttons changes. Config adds overlay on/off and position fields.

**Tech Stack:** Win32 GDI (already available via `Win32_Graphics_Gdi` feature), `UpdateLayeredWindow` for per-pixel alpha.

**Spec:** `docs/superpowers/specs/2026-03-10-overlay-hud-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/config.rs` | Modify | Add `overlay`, `overlay_position` fields, `OverlayPosition` enum |
| `src/overlay.rs` | Create | Window creation, layout, GDI rendering, show/hide, state logic |
| `src/main.rs` | Modify | Wire overlay into AppState, tick_controller, tray menu, lifecycle |

---

### Task 1: Config — add overlay fields

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add OverlayPosition enum and config fields**

In `src/config.rs`, add after the `OutputMode` enum:

```rust
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}
```

Add to `CompiledConfig`:
```rust
pub struct CompiledConfig {
    pub controller_player: u32,
    pub nintendo_layout: bool,
    pub overlay: bool,
    pub overlay_position: OverlayPosition,
    pub profiles: Vec<CompiledProfile>,
}
```

Add to `RawConfig`:
```rust
struct RawConfig {
    schema_version: Option<u32>,
    controller_player: Option<u32>,
    nintendo_layout: Option<bool>,
    overlay: Option<bool>,
    overlay_position: Option<OverlayPosition>,
    profiles: IndexMap<String, RawProfile>,
}
```

Update `parse_config` return to include new fields:
```rust
Ok(CompiledConfig {
    controller_player,
    nintendo_layout: raw.nintendo_layout.unwrap_or(false),
    overlay: raw.overlay.unwrap_or(false),
    overlay_position: raw.overlay_position.unwrap_or_default(),
    profiles,
})
```

- [ ] **Step 2: Build to verify**

Run: `cargo build --release 2>&1`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add overlay and overlay_position fields"
```

---

### Task 2: Overlay module — window creation and destruction

**Files:**
- Create: `src/overlay.rs`
- Modify: `src/main.rs` (add `mod overlay;`)

- [ ] **Step 1: Create overlay.rs with OverlayWindow struct and create/destroy**

Create `src/overlay.rs`. The overlay window is a `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW` popup. It registers its own window class (no wndproc needed — just DefWindowProcW). `UpdateLayeredWindow` handles all rendering.

```rust
use std::collections::BTreeSet;

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HWND, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    SelectObject, DIB_RGB_COLORS, HBITMAP, HDC, HFONT, HGDIOBJ,
    SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, ShowWindow, UpdateLayeredWindow, ULW_ALPHA,
    SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SW_HIDE, SW_SHOWNOACTIVATE,
    WS_EX_LAYERED, WS_EX_TRANSPARENT, WS_EX_TOPMOST, WS_EX_TOOLWINDOW,
    WS_POPUP, WNDCLASSW, RegisterClassW, WINDOW_EX_STYLE, DefWindowProcW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetSystemMetrics;
use windows::Win32::UI::WindowsAndMessaging::{
    SM_CXSCREEN, SM_CYSCREEN, SystemParametersInfoW, SPI_GETWORKAREA,
};
use windows::Win32::Foundation::RECT;
use windows::core::PCWSTR;

use crate::config::{CompiledMapping, CompiledProfile, OverlayPosition, PadButton};

const OVERLAY_CLASS: &str = "zero_mapper_overlay";

// Layout constants (pixels)
const OVERLAY_WIDTH: i32 = 320;
const OVERLAY_HEIGHT: i32 = 220;
const OVERLAY_MARGIN: i32 = 16;
const BTN_W: i32 = 38;
const BTN_H: i32 = 28;
const BTN_GAP: i32 = 3;
const LABEL_H: i32 = 14;
const BG_ALPHA: u8 = 200; // ~78% opaque

pub struct OverlayWindow {
    hwnd: HWND,
    visible: bool,
    position: OverlayPosition,
    // Cached GDI resources
    font_button: HFONT,
    font_label: HFONT,
    font_profile: HFONT,
    // Previous state for change detection
    prev_buttons: BTreeSet<PadButton>,
    prev_profile_index: Option<usize>,
}
```

Implement creation:

```rust
impl OverlayWindow {
    pub unsafe fn create(position: OverlayPosition) -> Result<Self> {
        let hinstance = GetModuleHandleW(None)?;
        let class_name = crate::wide_null(OVERLAY_CLASS);

        let wc = WNDCLASSW {
            hInstance: hinstance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: Some(DefWindowProcW),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
        let hwnd = CreateWindowExW(
            ex_style,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            0, 0, OVERLAY_WIDTH, OVERLAY_HEIGHT,
            None, None, hinstance, None,
        )?;

        let font_button = create_font(-12, false);
        let font_label = create_font(-11, true);
        let font_profile = create_font(-10, false);

        let mut overlay = Self {
            hwnd,
            visible: false,
            position,
            font_button,
            font_label,
            font_profile,
            prev_buttons: BTreeSet::new(),
            prev_profile_index: None,
        };
        overlay.reposition()?;
        Ok(overlay)
    }

    pub unsafe fn destroy(&mut self) {
        if !self.hwnd.0.is_null() {
            let _ = DestroyWindow(self.hwnd);
            self.hwnd = HWND::default();
        }
        delete_font(&mut self.font_button);
        delete_font(&mut self.font_label);
        delete_font(&mut self.font_profile);
    }

    pub unsafe fn show(&mut self) {
        if !self.visible {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            self.visible = true;
        }
    }

    pub unsafe fn hide(&mut self) {
        if self.visible {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
            self.visible = false;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub unsafe fn set_position(&mut self, position: OverlayPosition) -> Result<()> {
        self.position = position;
        self.reposition()
    }

    unsafe fn reposition(&mut self) -> Result<()> {
        let mut work_area = RECT::default();
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut work_area as *mut _ as *mut _),
            Default::default(),
        );

        let (x, y) = match self.position {
            OverlayPosition::TopLeft => (
                work_area.left + OVERLAY_MARGIN,
                work_area.top + OVERLAY_MARGIN,
            ),
            OverlayPosition::TopRight => (
                work_area.right - OVERLAY_WIDTH - OVERLAY_MARGIN,
                work_area.top + OVERLAY_MARGIN,
            ),
            OverlayPosition::BottomLeft => (
                work_area.left + OVERLAY_MARGIN,
                work_area.bottom - OVERLAY_HEIGHT - OVERLAY_MARGIN,
            ),
            OverlayPosition::BottomRight => (
                work_area.right - OVERLAY_WIDTH - OVERLAY_MARGIN,
                work_area.bottom - OVERLAY_HEIGHT - OVERLAY_MARGIN,
            ),
        };

        SetWindowPos(
            self.hwnd,
            HWND_TOPMOST,
            x, y, 0, 0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        )?;
        Ok(())
    }
}

unsafe fn create_font(height: i32, bold: bool) -> HFONT {
    CreateFontW(
        height, 0, 0, 0,
        if bold { 700 } else { 400 },
        0, 0, 0, 0, 0, 0, 0, 0,
        PCWSTR(crate::wide_null("Segoe UI").as_ptr()),
    )
}

unsafe fn delete_font(font: &mut HFONT) {
    if !font.is_invalid() {
        let _ = DeleteObject(*font);
        *font = HFONT::default();
    }
}
```

- [ ] **Step 2: Add `mod overlay;` to main.rs**

At the top of `src/main.rs`, add after `mod startup;`:
```rust
mod overlay;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build --release 2>&1`
Expected: Compiles. Some unused warnings are fine at this stage.

- [ ] **Step 4: Commit**

```bash
git add src/overlay.rs src/main.rs
git commit -m "feat(overlay): add window creation and lifecycle"
```

---

### Task 3: Overlay module — layout and rendering

**Files:**
- Modify: `src/overlay.rs`

- [ ] **Step 1: Add layout structs and button position calculation**

Add to `overlay.rs` — defines pixel positions for each button in the overlay:

```rust
struct ButtonSlot {
    button: PadButton,
    x: i32,
    y: i32,
}

fn button_layout() -> Vec<ButtonSlot> {
    // Left side: Dpad (cross pattern)
    // Right side: Face buttons (diamond pattern)
    // Top: LB, RB
    // Bottom: Start
    let dpad_cx = 60;  // center x of dpad group
    let face_cx = 220; // center x of face group
    let row_top = 50;  // y for top of dpad/face area

    vec![
        // Shoulder buttons
        ButtonSlot { button: PadButton::Lb, x: dpad_cx - BTN_W / 2, y: 12 },
        ButtonSlot { button: PadButton::Rb, x: face_cx - BTN_W / 2, y: 12 },
        // Dpad
        ButtonSlot { button: PadButton::DpadUp,    x: dpad_cx - BTN_W / 2, y: row_top },
        ButtonSlot { button: PadButton::DpadLeft,   x: dpad_cx - BTN_W - BTN_W / 2 - BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::DpadRight,  x: dpad_cx + BTN_W / 2 + BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::DpadDown,   x: dpad_cx - BTN_W / 2, y: row_top + 2 * (BTN_H + BTN_GAP) },
        // Face buttons (diamond)
        ButtonSlot { button: PadButton::X, x: face_cx - BTN_W / 2, y: row_top },
        ButtonSlot { button: PadButton::Y, x: face_cx - BTN_W - BTN_W / 2 - BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::A, x: face_cx + BTN_W / 2 + BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::B, x: face_cx - BTN_W / 2, y: row_top + 2 * (BTN_H + BTN_GAP) },
        // Start
        ButtonSlot { button: PadButton::Start, x: (OVERLAY_WIDTH - BTN_W) / 2, y: row_top + 3 * (BTN_H + BTN_GAP) + LABEL_H + 4 },
    ]
}
```

- [ ] **Step 2: Add key label formatting helper**

```rust
fn format_key_label(keys: &[u16]) -> String {
    keys.iter()
        .map(|&vk| match vk {
            0x08 => "Bksp",
            0x09 => "Tab",
            0x0D => "Ent",
            0x10 => "Shft",
            0x11 => "C",     // Ctrl shortened
            0x12 => "Alt",
            0x1B => "Esc",
            0x20 => "Spc",
            0x21 => "PgU",
            0x22 => "PgD",
            0x23 => "End",
            0x24 => "Home",
            0x25 => "←",
            0x26 => "↑",
            0x27 => "→",
            0x28 => "↓",
            0x2D => "Ins",
            0x2E => "Del",
            0x5B => "Win",
            vk @ 0x30..=0x39 => return ((vk as u8) as char).to_string(),  // 0-9
            vk @ 0x41..=0x5A => return ((vk as u8) as char).to_string(),  // A-Z
            vk @ 0x70..=0x87 => return format!("F{}", vk - 0x6F),        // F1-F24
            0xBA => ";",
            0xBB => "=",
            0xBC => ",",
            0xBD => "-",
            0xBE => ".",
            0xBF => "/",
            0xDB => "[",
            0xDC => "\\",
            0xDD => "]",
            0xDE => "'",
            _ => "?",
        }.to_string())
        .collect::<Vec<_>>()
        .join("-")
}
```

- [ ] **Step 3: Add the render method**

The `update` method computes overlay state and redraws. It checks if pressed_buttons changed to avoid unnecessary redraws.

```rust
impl OverlayWindow {
    /// Call each tick. Redraws only if state changed.
    pub unsafe fn update(
        &mut self,
        pressed: &BTreeSet<PadButton>,
        profile: Option<(usize, &CompiledProfile)>,
    ) {
        let profile_index = profile.as_ref().map(|(i, _)| *i);
        if *pressed == self.prev_buttons && profile_index == self.prev_profile_index {
            return;
        }
        self.prev_buttons.clone_from(pressed);
        self.prev_profile_index = profile_index;

        let Some((_, prof)) = profile else {
            return;
        };

        self.render(pressed, prof);
    }

    unsafe fn render(&self, pressed: &BTreeSet<PadButton>, profile: &CompiledProfile) {
        let width = OVERLAY_WIDTH;
        let height = OVERLAY_HEIGHT;

        // Create 32bpp DIB
        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(HWND::default());
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            .expect("CreateDIBSection");
        let old_bmp = SelectObject(hdc_mem, hbmp);

        // Fill background with semi-transparent black
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
        let bg_color = premultiply_argb(BG_ALPHA, 0x10, 0x10, 0x18);
        pixels.fill(bg_color);

        // Setup text rendering
        SetBkMode(hdc_mem, TRANSPARENT);

        // Determine modifier state
        let modifier: Option<PadButton> = if pressed.len() == 1 {
            pressed.iter().next().copied()
        } else {
            None
        };

        let layout = button_layout();
        for slot in &layout {
            let is_pressed = pressed.contains(&slot.button);
            let is_modifier = modifier == Some(slot.button);

            // Determine what key label to show
            let key_label = if let Some(mod_btn) = modifier {
                if slot.button == mod_btn {
                    // Modifier button itself — no label
                    None
                } else {
                    // Look for modifier+button combo
                    find_combo_mapping(profile, mod_btn, slot.button)
                        .map(|m| format_key_label(&m.keys))
                }
            } else {
                // Default: solo mapping
                find_solo_mapping(profile, slot.button)
                    .map(|m| format_key_label(&m.keys))
            };

            let is_dimmed = modifier.is_some()
                && !is_modifier
                && key_label.is_none();

            // Draw button rectangle
            let (bg, text_color, label_color) = if is_modifier {
                (premultiply_argb(BG_ALPHA, 0x2a, 0x4a, 0x8a),
                 0x00FFBB88u32, // #8bf in COLORREF BGR
                 0x00000000u32)
            } else if is_pressed {
                (premultiply_argb(BG_ALPHA, 0x3a, 0x6a, 0xba),
                 0x00FFFFFFu32,
                 0x00FFFFFFu32)
            } else if is_dimmed {
                (premultiply_argb(BG_ALPHA, 0x22, 0x22, 0x22),
                 0x00555555u32,
                 0x00000000u32)
            } else {
                (premultiply_argb(BG_ALPHA, 0x33, 0x33, 0x33),
                 0x00AAAAAAu32,
                 0x00FFAA66u32) // #6af in BGR
            };

            fill_rounded_rect(pixels, width, slot.x, slot.y, BTN_W, BTN_H, 4, bg);

            if is_modifier || is_pressed {
                let border = if is_modifier {
                    premultiply_argb(BG_ALPHA, 0x55, 0x88, 0xcc)
                } else {
                    premultiply_argb(BG_ALPHA, 0x77, 0xaa, 0xff)
                };
                draw_rect_border(pixels, width, height, slot.x, slot.y, BTN_W, BTN_H, border);
            }

            // Draw button name
            let btn_name = button_short_name(slot.button);
            SelectObject(hdc_mem, self.font_button);
            SetTextColor(hdc_mem, windows::Win32::Foundation::COLORREF(text_color));
            draw_text_centered(hdc_mem, btn_name, slot.x, slot.y, BTN_W, BTN_H);

            // Draw key label below button
            if let Some(label) = &key_label {
                SelectObject(hdc_mem, self.font_label);
                SetTextColor(hdc_mem, windows::Win32::Foundation::COLORREF(label_color));
                draw_text_centered(
                    hdc_mem,
                    label,
                    slot.x - 4,
                    slot.y + BTN_H + 1,
                    BTN_W + 8,
                    LABEL_H,
                );
            }
        }

        // Draw profile name at bottom
        SelectObject(hdc_mem, self.font_profile);
        SetTextColor(hdc_mem, windows::Win32::Foundation::COLORREF(0x00555555));
        draw_text_centered(hdc_mem, &profile.name, 0, height - 20, width, 16);

        // Apply to window via UpdateLayeredWindow
        let mut pt_src = POINT { x: 0, y: 0 };
        let size = SIZE { cx: width, cy: height };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let mut pt_dst = POINT::default();
        // Get current window position
        let mut rect = RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(self.hwnd, &mut rect);
        pt_dst.x = rect.left;
        pt_dst.y = rect.top;

        let _ = UpdateLayeredWindow(
            self.hwnd,
            hdc_screen,
            Some(&pt_dst),
            Some(&size),
            hdc_mem,
            Some(&pt_src),
            windows::Win32::Foundation::COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        // Cleanup
        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(hdc_mem);
        let _ = windows::Win32::Graphics::Gdi::ReleaseDC(HWND::default(), hdc_screen);
    }
}
```

- [ ] **Step 4: Add helper drawing functions**

```rust
fn premultiply_argb(a: u8, r: u8, g: u8, b: u8) -> u32 {
    let a32 = a as u32;
    let r32 = (r as u32 * a32 / 255) & 0xFF;
    let g32 = (g as u32 * a32 / 255) & 0xFF;
    let b32 = (b as u32 * a32 / 255) & 0xFF;
    (a32 << 24) | (r32 << 16) | (g32 << 8) | b32
}

fn fill_rounded_rect(pixels: &mut [u32], stride: i32, x: i32, y: i32, w: i32, h: i32, r: i32, color: u32) {
    for py in y..y + h {
        for px in x..x + w {
            if px < 0 || py < 0 || px >= stride || py >= stride * 2 {
                continue;
            }
            // Simple corner rounding check
            let in_corner = (px < x + r || px >= x + w - r) && (py < y + r || py >= y + h - r);
            if in_corner {
                let cx = if px < x + r { x + r } else { x + w - r - 1 };
                let cy = if py < y + r { y + r } else { y + h - r - 1 };
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy > r * r {
                    continue;
                }
            }
            let idx = (py * stride + px) as usize;
            if idx < pixels.len() {
                pixels[idx] = color;
            }
        }
    }
}

fn draw_rect_border(pixels: &mut [u32], stride: i32, total_h: i32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    for px in x..x + w {
        set_pixel(pixels, stride, total_h, px, y, color);
        set_pixel(pixels, stride, total_h, px, y + h - 1, color);
    }
    for py in y..y + h {
        set_pixel(pixels, stride, total_h, x, py, color);
        set_pixel(pixels, stride, total_h, x + w - 1, py, color);
    }
}

fn set_pixel(pixels: &mut [u32], stride: i32, total_h: i32, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && x < stride && y < total_h {
        let idx = (y * stride + x) as usize;
        if idx < pixels.len() {
            pixels[idx] = color;
        }
    }
}

unsafe fn draw_text_centered(hdc: HDC, text: &str, x: i32, y: i32, w: i32, h: i32) {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    };
    windows::Win32::Graphics::Gdi::DrawTextW(
        hdc,
        &mut wide.clone(),
        &mut rect,
        windows::Win32::Graphics::Gdi::DT_CENTER
            | windows::Win32::Graphics::Gdi::DT_VCENTER
            | windows::Win32::Graphics::Gdi::DT_SINGLELINE
            | windows::Win32::Graphics::Gdi::DT_NOCLIP,
    );
}

fn find_solo_mapping<'a>(profile: &'a CompiledProfile, button: PadButton) -> Option<&'a CompiledMapping> {
    profile.mappings.iter().find(|m| {
        m.buttons.len() == 1 && m.buttons.contains(&button)
    })
}

fn find_combo_mapping<'a>(
    profile: &'a CompiledProfile,
    modifier: PadButton,
    button: PadButton,
) -> Option<&'a CompiledMapping> {
    profile.mappings.iter().find(|m| {
        m.buttons.len() == 2 && m.buttons.contains(&modifier) && m.buttons.contains(&button)
    })
}

fn button_short_name(button: PadButton) -> &'static str {
    match button {
        PadButton::A => "A",
        PadButton::B => "B",
        PadButton::X => "X",
        PadButton::Y => "Y",
        PadButton::Lb => "LB",
        PadButton::Rb => "RB",
        PadButton::Lt => "LT",
        PadButton::Rt => "RT",
        PadButton::Back => "Bk",
        PadButton::Start => "St",
        PadButton::L3 => "L3",
        PadButton::R3 => "R3",
        PadButton::DpadUp => "Up",
        PadButton::DpadDown => "Dn",
        PadButton::DpadLeft => "L",
        PadButton::DpadRight => "R",
    }
}
```

- [ ] **Step 5: Build to verify**

Run: `cargo build --release 2>&1`
Expected: Compiles. Some unused import warnings may appear.

- [ ] **Step 6: Commit**

```bash
git add src/overlay.rs
git commit -m "feat(overlay): add layout calculation and GDI rendering"
```

---

### Task 4: Main integration — wire overlay into AppState and tick_controller

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add overlay state to AppState**

Add to the `AppState` struct:
```rust
overlay: Option<overlay::OverlayWindow>,
overlay_visible: bool,
```

Add import at top:
```rust
use config::OverlayPosition;
```

- [ ] **Step 2: Initialize overlay in AppState constructor**

In the `run()` function, after creating AppState, add overlay fields:
```rust
overlay: None, // created in WM_CREATE
overlay_visible: false,
```

- [ ] **Step 3: Create overlay window in WM_CREATE**

In the `WM_CREATE` handler, after the foreground hook setup, add:
```rust
if state.config.overlay {
    match overlay::OverlayWindow::create(state.config.overlay_position) {
        Ok(ov) => {
            state.overlay = Some(ov);
            state.overlay_visible = true;
        }
        Err(err) => report_error(state, "Overlay creation failed", &err),
    }
}
```

- [ ] **Step 4: Destroy overlay in WM_DESTROY**

In the `WM_DESTROY` handler, before `PostQuitMessage`, add:
```rust
if let Some(ref mut ov) = state.overlay {
    ov.destroy();
}
```

- [ ] **Step 5: Update overlay in tick_controller**

At the end of `tick_controller`, after `state.active_mapping_ids` is updated, add:
```rust
// Update overlay
if let Some(ref mut ov) = state.overlay {
    if state.overlay_visible {
        let profile_data = profile_index
            .and_then(|i| state.config.profiles.get(i).map(|p| (i, p)));
        ov.update(&state.pressed_buttons, profile_data);
        ov.show();
    }
}
```

In the early returns of `tick_controller` (when disabled, no profile, disconnected), add overlay hide:
```rust
if let Some(ref mut ov) = state.overlay {
    ov.hide();
}
```

- [ ] **Step 6: Show/hide overlay on profile change**

In `refresh_window_context`, after profile change detection, sync overlay visibility:
```rust
if let Some(ref mut ov) = state.overlay {
    if state.active_profile_index.is_some() && state.overlay_visible {
        ov.show();
    } else {
        ov.hide();
    }
}
```

- [ ] **Step 7: Handle overlay on suspend/resume**

In the `PBT_APMSUSPEND` and `WTS_SESSION_LOCK` handlers, add:
```rust
if let Some(ref mut ov) = state.overlay {
    ov.hide();
}
```

In `PBT_APMRESUMEAUTOMATIC` and `WTS_SESSION_UNLOCK`, the existing `refresh_window_context` call will handle showing it.

- [ ] **Step 8: Handle overlay on config reload**

In the `ID_RELOAD_CONFIG` handler, after loading new config, recreate overlay if settings changed:
```rust
// Recreate overlay if config changed
if let Some(ref mut ov) = state.overlay {
    ov.destroy();
    state.overlay = None;
}
if state.config.overlay {
    match overlay::OverlayWindow::create(state.config.overlay_position) {
        Ok(ov) => {
            state.overlay = Some(ov);
            state.overlay_visible = true;
        }
        Err(err) => report_error(state, "Overlay creation failed", &err),
    }
}
```

- [ ] **Step 9: Build to verify**

Run: `cargo build --release 2>&1`
Expected: Compiles with no errors.

- [ ] **Step 10: Commit**

```bash
git add src/main.rs
git commit -m "feat(overlay): integrate overlay into main loop and lifecycle"
```

---

### Task 5: Tray menu — add overlay toggle

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add menu constant**

Add after `ID_EXIT`:
```rust
const ID_TOGGLE_OVERLAY: usize = 1006;
```

- [ ] **Step 2: Add menu item in show_context_menu**

After the "Enable/Disable mappings" item and before the Controller submenu, add:
```rust
if state.overlay.is_some() {
    let overlay_label = if state.overlay_visible {
        "Hide overlay"
    } else {
        "Show overlay"
    };
    append_text_item(menu, ID_TOGGLE_OVERLAY, overlay_label, MF_STRING)?;
}
```

- [ ] **Step 3: Handle toggle in handle_command**

Add case in `handle_command`:
```rust
ID_TOGGLE_OVERLAY => {
    state.overlay_visible = !state.overlay_visible;
    if let Some(ref mut ov) = state.overlay {
        if state.overlay_visible {
            ov.show();
        } else {
            ov.hide();
        }
    }
}
```

- [ ] **Step 4: Build to verify**

Run: `cargo build --release 2>&1`
Expected: Compiles.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(overlay): add tray menu toggle for overlay visibility"
```

---

### Task 6: Final build, manual test, and release commit

**Files:**
- Modify: `config.toml` (add overlay = true for testing)

- [ ] **Step 1: Enable overlay in config.toml for testing**

Add to the top of `config.toml` (and SAMPLE_CONFIG in main.rs):
```toml
overlay = true
overlay_position = "bottom-right"
```

- [ ] **Step 2: Full release build**

Run: `cargo build --release 2>&1`
Expected: Clean compile, no warnings.

- [ ] **Step 3: Fix any compilation issues**

Resolve any remaining type errors, missing imports, or API mismatches with the `windows` crate 0.58.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: overlay HUD for controller button visualization

Adds a semi-transparent overlay showing controller layout with
key mappings. Supports press visualization, dynamic combo layer
switching when a single button is held, and configurable position."
```
