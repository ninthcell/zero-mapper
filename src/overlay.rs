#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeSet;

use anyhow::Result;
use windows::Win32::Foundation::{HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DIB_RGB_COLORS, DT_CENTER, DT_NOCLIP,
    DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, GetDC, HDC, HFONT,
    ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetWindowRect, HWND_TOPMOST, RegisterClassW,
    SPI_GETWORKAREA, SWP_NOACTIVATE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE,
    SetWindowPos, ShowWindow, SystemParametersInfoW, ULW_ALPHA, UpdateLayeredWindow,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::PCWSTR;

use crate::config::{CompiledMapping, CompiledProfile, OverlayPosition, PadButton};

const OVERLAY_CLASS: &str = "zero_mapper_overlay";
const OVERLAY_WIDTH: i32 = 320;
const OVERLAY_HEIGHT: i32 = 240;
const OVERLAY_MARGIN: i32 = 16;
const BTN_W: i32 = 38;
const BTN_H: i32 = 28;
const BTN_GAP: i32 = 3;
const LABEL_H: i32 = 14;
const BG_ALPHA: u8 = 200;

pub struct OverlayWindow {
    hwnd: HWND,
    visible: bool,
    position: OverlayPosition,
    font_button: HFONT,
    font_label: HFONT,
    font_profile: HFONT,
    prev_buttons: BTreeSet<PadButton>,
    prev_profile_index: Option<usize>,
}

struct ButtonSlot {
    button: PadButton,
    x: i32,
    y: i32,
}

impl OverlayWindow {
    pub unsafe fn create(position: OverlayPosition) -> Result<Self> {
        let hinstance = GetModuleHandleW(None)?;
        let class_name = crate::wide_null(OVERLAY_CLASS);

        let wc = WNDCLASSW {
            hInstance: hinstance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: Some(overlay_wnd_proc),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
        let hwnd = CreateWindowExW(
            ex_style,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            0,
            0,
            OVERLAY_WIDTH,
            OVERLAY_HEIGHT,
            None,
            None,
            hinstance,
            None,
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
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        )?;
        Ok(())
    }

    unsafe fn render(&self, pressed: &BTreeSet<PadButton>, profile: &CompiledProfile) {
        let width = OVERLAY_WIDTH;
        let height = OVERLAY_HEIGHT;

        let hdc_screen = GetDC(HWND::default());
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            .expect("CreateDIBSection");
        let old_bmp = SelectObject(hdc_mem, hbmp);

        let pixels =
            std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
        let bg_color = premultiply_argb(BG_ALPHA, 0x10, 0x10, 0x18);
        pixels.fill(bg_color);

        SetBkMode(hdc_mem, TRANSPARENT);

        let modifier: Option<PadButton> = if pressed.len() == 1 {
            pressed.iter().next().copied()
        } else {
            None
        };

        let layout = button_layout();
        for slot in &layout {
            let is_pressed = pressed.contains(&slot.button);
            let is_modifier = modifier == Some(slot.button);

            let key_label = if let Some(mod_btn) = modifier {
                if slot.button == mod_btn {
                    None
                } else {
                    find_combo_mapping(profile, mod_btn, slot.button)
                        .map(|m| format_key_label(&m.keys))
                }
            } else {
                find_solo_mapping(profile, slot.button).map(|m| format_key_label(&m.keys))
            };

            let is_dimmed = modifier.is_some() && !is_modifier && key_label.is_none();

            let (bg, text_color, label_color) = if is_modifier {
                (
                    premultiply_argb(BG_ALPHA, 0x2a, 0x4a, 0x8a),
                    0x00FFBB88u32,
                    0x00000000u32,
                )
            } else if is_pressed {
                (
                    premultiply_argb(BG_ALPHA, 0x3a, 0x6a, 0xba),
                    0x00FFFFFFu32,
                    0x00FFFFFFu32,
                )
            } else if is_dimmed {
                (
                    premultiply_argb(BG_ALPHA, 0x22, 0x22, 0x22),
                    0x00555555u32,
                    0x00000000u32,
                )
            } else {
                (
                    premultiply_argb(BG_ALPHA, 0x33, 0x33, 0x33),
                    0x00AAAAAAu32,
                    0x00FFAA66u32,
                )
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

            let btn_name = button_short_name(slot.button);
            SelectObject(hdc_mem, self.font_button);
            SetTextColor(hdc_mem, COLORREF(text_color));
            draw_text_centered(hdc_mem, btn_name, slot.x, slot.y, BTN_W, BTN_H);

            if let Some(label) = &key_label {
                SelectObject(hdc_mem, self.font_label);
                SetTextColor(hdc_mem, COLORREF(label_color));
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

        SelectObject(hdc_mem, self.font_profile);
        SetTextColor(hdc_mem, COLORREF(0x00555555));
        draw_text_centered(hdc_mem, &profile.name, 0, height - 20, width, 16);

        let pt_src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: width,
            cy: height,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let mut rect = RECT::default();
        let _ = GetWindowRect(self.hwnd, &mut rect);
        let pt_dst = POINT {
            x: rect.left,
            y: rect.top,
        };

        let _ = UpdateLayeredWindow(
            self.hwnd,
            hdc_screen,
            Some(&pt_dst),
            Some(&size),
            hdc_mem,
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(HWND::default(), hdc_screen);
    }
}

unsafe fn create_font(height: i32, bold: bool) -> HFONT {
    CreateFontW(
        height,
        0,
        0,
        0,
        if bold { 700 } else { 400 },
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        PCWSTR(crate::wide_null("Segoe UI").as_ptr()),
    )
}

unsafe fn delete_font(font: &mut HFONT) {
    if !font.is_invalid() {
        let _ = DeleteObject(*font);
        *font = HFONT::default();
    }
}

fn button_layout() -> Vec<ButtonSlot> {
    let dpad_cx = 80;
    let face_cx = 240;
    let row_top = 55;

    vec![
        ButtonSlot { button: PadButton::Lb, x: dpad_cx - BTN_W / 2, y: 6 },
        ButtonSlot { button: PadButton::Rb, x: face_cx - BTN_W / 2, y: 6 },
        ButtonSlot { button: PadButton::DpadUp, x: dpad_cx - BTN_W / 2, y: row_top },
        ButtonSlot { button: PadButton::DpadLeft, x: dpad_cx - BTN_W - BTN_W / 2 - BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::DpadRight, x: dpad_cx + BTN_W / 2 + BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::DpadDown, x: dpad_cx - BTN_W / 2, y: row_top + 2 * (BTN_H + BTN_GAP) },
        ButtonSlot { button: PadButton::X, x: face_cx - BTN_W / 2, y: row_top },
        ButtonSlot { button: PadButton::Y, x: face_cx - BTN_W - BTN_W / 2 - BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::A, x: face_cx + BTN_W / 2 + BTN_GAP, y: row_top + BTN_H + BTN_GAP },
        ButtonSlot { button: PadButton::B, x: face_cx - BTN_W / 2, y: row_top + 2 * (BTN_H + BTN_GAP) },
        ButtonSlot { button: PadButton::Start, x: (OVERLAY_WIDTH - BTN_W) / 2, y: row_top + 3 * (BTN_H + BTN_GAP) + LABEL_H + 4 },
    ]
}

fn format_key_label(keys: &[u16]) -> String {
    keys.iter()
        .map(|&vk| {
            match vk {
                0x08 => "Bksp".to_string(),
                0x09 => "Tab".to_string(),
                0x0D => "Ent".to_string(),
                0x10 => "Shft".to_string(),
                0x11 => "C".to_string(),
                0x12 => "Alt".to_string(),
                0x1B => "Esc".to_string(),
                0x20 => "Spc".to_string(),
                0x21 => "PgU".to_string(),
                0x22 => "PgD".to_string(),
                0x23 => "End".to_string(),
                0x24 => "Home".to_string(),
                0x2D => "Ins".to_string(),
                0x2E => "Del".to_string(),
                0x5B => "Win".to_string(),
                vk @ 0x30..=0x39 => ((vk as u8) as char).to_string(),
                vk @ 0x41..=0x5A => ((vk as u8) as char).to_string(),
                vk @ 0x70..=0x87 => format!("F{}", vk - 0x6F),
                0xBA => ";".to_string(),
                0xBB => "=".to_string(),
                0xBC => ",".to_string(),
                0xBD => "-".to_string(),
                0xBE => ".".to_string(),
                0xBF => "/".to_string(),
                0xDB => "[".to_string(),
                0xDC => "\\".to_string(),
                0xDD => "]".to_string(),
                0xDE => "'".to_string(),
                _ => "?".to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

fn premultiply_argb(a: u8, r: u8, g: u8, b: u8) -> u32 {
    let a32 = a as u32;
    let r32 = (r as u32 * a32 / 255) & 0xFF;
    let g32 = (g as u32 * a32 / 255) & 0xFF;
    let b32 = (b as u32 * a32 / 255) & 0xFF;
    (a32 << 24) | (r32 << 16) | (g32 << 8) | b32
}

fn fill_rounded_rect(
    pixels: &mut [u32],
    stride: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: i32,
    color: u32,
) {
    let total_h = pixels.len() as i32 / stride;
    for py in y..y + h {
        for px in x..x + w {
            if px < 0 || py < 0 || px >= stride || py >= total_h {
                continue;
            }
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

fn draw_rect_border(
    pixels: &mut [u32],
    stride: i32,
    total_h: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
) {
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
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    };
    DrawTextW(
        hdc,
        &mut wide,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOCLIP,
    );
}

fn find_solo_mapping<'a>(
    profile: &'a CompiledProfile,
    button: PadButton,
) -> Option<&'a CompiledMapping> {
    profile
        .mappings
        .iter()
        .find(|m| m.buttons.len() == 1 && m.buttons.contains(&button))
}

fn find_combo_mapping<'a>(
    profile: &'a CompiledProfile,
    modifier: PadButton,
    button: PadButton,
) -> Option<&'a CompiledMapping> {
    profile
        .mappings
        .iter()
        .find(|m| m.buttons.len() == 2 && m.buttons.contains(&modifier) && m.buttons.contains(&button))
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
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
