#![windows_subsystem = "windows"]
#![allow(unsafe_op_in_unsafe_fn)]

mod config;
mod input;
mod mapper;
mod overlay;
mod startup;

use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use config::{
    CompiledConfig, CompiledMapping, CompiledProfile, OutputMode, PadButton, load_config,
};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::WTSRegisterSessionNotification;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Input::XboxController::{
    XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_DPAD_DOWN,
    XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP,
    XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_SHOULDER,
    XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y,
    XINPUT_STATE, XInputGetState,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CREATESTRUCTW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, EVENT_SYSTEM_FOREGROUND, GWLP_USERDATA, GetCursorPos,
    GetForegroundWindow, GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
    HMENU, IDC_ARROW, KillTimer, LoadCursorW, LoadIconW, MENU_ITEM_FLAGS, MF_CHECKED, MF_GRAYED,
    MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
    SW_SHOWNORMAL, SetForegroundWindow, SetTimer, SetWindowLongPtrW, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_APP, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_DISPLAYCHANGE,
    WM_LBUTTONUP, WM_NCCREATE, WM_NULL, WM_POWERBROADCAST, WM_RBUTTONUP, WM_TIMER,
    WM_WTSSESSION_CHANGE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, w};

const WM_TRAYICON: u32 = WM_APP + 1;
const WM_FOREGROUND_CHANGED: u32 = WM_APP + 2;
const WM_CONFIG_CHANGED: u32 = WM_APP + 3;
const TIMER_CONTROLLER: usize = 1;

const ID_TOGGLE_ENABLE: usize = 1001;
const ID_TOGGLE_STARTUP: usize = 1002;
const ID_RELOAD_CONFIG: usize = 1003;
const ID_OPEN_CONFIG: usize = 1004;
const ID_EXIT: usize = 1005;
const ID_TOGGLE_OVERLAY: usize = 1006;
const ID_PLAYER_BASE: usize = 1100;
const POLL_INTERVAL_ACTIVE_MS: u32 = 16;
const POLL_INTERVAL_IDLE_MS: u32 = 150;
const POLL_INTERVAL_DISCONNECTED_MS: u32 = 1000;
const POLL_INTERVAL_BACKGROUND_MS: u32 = 500;
const IDLE_THRESHOLD: u128 = 1000;

const PBT_APMSUSPEND: u32 = 0x0004;
const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;
const WTS_SESSION_LOCK: u32 = 0x7;
const WTS_SESSION_UNLOCK: u32 = 0x8;

const APP_CLASS: &str = "zero_mapper_hidden_window";
static FOREGROUND_TARGET_HWND: AtomicIsize = AtomicIsize::new(0);
const SAMPLE_CONFIG: &str = r#"schema_version = 1
controller_player = 1
nintendo_layout = true
overlay = true
overlay_position = "bottom-right"
overlay_opacity = 80

[profiles.clip_studio]
name = "Clip Studio Paint"
title_regex = "CLIP STUDIO PAINT|Clip Studio Paint"

[profiles.clip_studio.map]
A = "P"
Y = "B"
B = "E"
X = { mode = "hold", send = "Space" }
"LB+Y" = "I"
DpadLeft = "Ctrl+Z"
DpadRight = "Ctrl+Y"
"LB+A" = "M"
"LB+B" = "K"
"RB+A" = "Ctrl+T"
Start = "Ctrl+S"

[profiles.photoshop]
name = "Photoshop"
title_regex = "Photoshop"

[profiles.photoshop.map]
A = "B"
Y = "I"
B = "E"
X = { mode = "hold", send = "Space" }
DpadLeft = "Ctrl+Z"
DpadRight = "Ctrl+Shift+Z"
"LB+A" = "M"
"LB+B" = "V"
"LB+DpadLeft" = "["
"LB+DpadRight" = "]"
"RB+DpadLeft" = "Ctrl+-"
"RB+DpadRight" = "Ctrl+="
"RB+A" = "Ctrl+T"
"RB+B" = "X"
Start = "Ctrl+S"

[profiles.aseprite]
name = "Aseprite"
title_regex = "Aseprite"

[profiles.aseprite.map]
A = "B"
B = "E"
X = { mode = "hold", send = "Space" }
Y = { mode = "hold", send = "Alt" }
"LB+Y" = { mode = "hold", send = "Ctrl" }
DpadLeft = "Ctrl+Z"
DpadRight = "Ctrl+Y"
"LB+A" = "M"
"RB+X" = "Tab"
Start = "Ctrl+S"

[profiles.krita]
name = "Krita"
title_regex = "Krita"

[profiles.krita.map]
A = "B"
B = "E"
X = { mode = "hold", send = "Space" }
Y = { mode = "hold", send = "Ctrl" }
DpadLeft = "Ctrl+Z"
DpadRight = "Ctrl+Shift+Z"
"LB+A" = "Ctrl+R"
"LB+B" = "V"
"LB+DpadLeft" = "["
"LB+DpadRight" = "]"
"RB+B" = "X"
"RB+X" = "D"
"RB+Y" = "F5"
Start = "Ctrl+S"
"#;

struct AppState {
    hwnd: HWND,
    app_icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    config_path: PathBuf,
    exe_path: String,
    config: CompiledConfig,
    current_title: String,
    active_profile_index: Option<usize>,
    last_matched_title: String,
    last_matched_profile_index: Option<usize>,
    enabled: bool,
    startup_enabled: bool,
    last_error: Option<String>,
    active_mapping_ids: Vec<usize>,
    held_mapping_ids: Vec<usize>,
    held_key_refs: HashMap<u16, u32>,
    pressed_buttons: BTreeSet<PadButton>,
    scratch_mapping_ids: Vec<usize>,
    controller_connected: bool,
    controller_timer_interval_ms: Option<u32>,
    last_input_tick: Option<Instant>,
    suspended: bool,
    in_menu: bool,
    foreground_hook: HWINEVENTHOOK,
    overlay: Option<overlay::OverlayWindow>,
    overlay_visible: bool,
}

fn main() {
    if let Err(err) = unsafe { run() } {
        let text = wide_null(&format!("{err:#}"));
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                w!("ZeroMapper"),
                windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
}

unsafe fn run() -> Result<()> {
    let exe_path = std::env::current_exe().context("failed to determine executable path")?;
    let config_path = exe_path
        .parent()
        .context("failed to determine executable directory")?
        .join("config.toml");
    if !config_path.exists() {
        std::fs::write(&config_path, SAMPLE_CONFIG)
            .context("failed to create sample config.toml")?;
    }

    let config = load_config(&config_path)?;
    let hinstance = GetModuleHandleW(None)?;
    let class_name = wide_null(APP_CLASS);

    let app_icon = LoadIconW(hinstance, PCWSTR(1 as *const u16))?;

    let wc = WNDCLASSW {
        hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW)?,
        hIcon: app_icon,
        hInstance: hinstance.into(),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        lpfnWndProc: Some(window_proc),
        hbrBackground: HBRUSH::default(),
        ..Default::default()
    };
    RegisterClassW(&wc);

    let state = Box::new(AppState {
        hwnd: HWND::default(),
        app_icon,
        config_path,
        exe_path: exe_path.display().to_string(),
        config,
        current_title: String::new(),
        active_profile_index: None,
        last_matched_title: String::new(),
        last_matched_profile_index: None,
        enabled: true,
        startup_enabled: startup::is_startup_enabled().unwrap_or(false),
        last_error: None,
        active_mapping_ids: Vec::new(),
        held_mapping_ids: Vec::new(),
        held_key_refs: HashMap::new(),
        pressed_buttons: BTreeSet::new(),
        scratch_mapping_ids: Vec::new(),
        controller_connected: false,
        controller_timer_interval_ms: None,
        last_input_tick: None,
        suspended: false,
        in_menu: false,
        foreground_hook: HWINEVENTHOOK::default(),
        overlay: None,
        overlay_visible: false,
    });

    let _hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        PCWSTR(class_name.as_ptr()),
        w!("ZeroMapper"),
        WS_OVERLAPPEDWINDOW,
        0,
        0,
        0,
        0,
        None,
        None,
        hinstance,
        Some(Box::into_raw(state) as *const c_void),
    )?;

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let createstruct = &*(lparam.0 as *const CREATESTRUCTW);
            let state_ptr = createstruct.lpCreateParams as *mut AppState;
            (*state_ptr).hwnd = hwnd;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            LRESULT(1)
        }
        WM_CREATE => {
            let state = get_state(hwnd);
            FOREGROUND_TARGET_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
            if let Err(err) = add_tray_icon(state) {
                state.last_error = Some(format!("{err:#}"));
            }
            if let Err(err) = install_foreground_hook(state) {
                report_error(state, "Foreground hook failed", &err);
            }
            let _ = WTSRegisterSessionNotification(hwnd, 0);
            if state.config.overlay {
                match overlay::OverlayWindow::create(
                    state.config.overlay_position,
                    state.config.overlay_opacity,
                    state.config.nintendo_layout,
                ) {
                    Ok(ov) => {
                        state.overlay = Some(ov);
                        state.overlay_visible = true;
                    }
                    Err(err) => report_error(state, "Overlay creation failed", &err),
                }
            }
            let _ = refresh_window_context(state);
            let _ = sync_controller_timer(state);
            let _ = tick_controller(state);
            spawn_config_watcher(hwnd, &state.config_path);
            LRESULT(0)
        }
        WM_CONFIG_CHANGED => {
            let state = get_state(hwnd);
            handle_command(state, ID_RELOAD_CONFIG);
            LRESULT(0)
        }
        WM_COMMAND => {
            let state = get_state(hwnd);
            handle_command(state, (wparam.0 & 0xFFFF) as usize);
            LRESULT(0)
        }
        WM_TIMER => {
            let state = get_state(hwnd);
            let result = match wparam.0 {
                TIMER_CONTROLLER => tick_controller(state),
                _ => Ok(()),
            };
            if let Err(err) = result {
                state.last_error = Some(format!("{err:#}"));
            }
            LRESULT(0)
        }
        WM_FOREGROUND_CHANGED => {
            let state = get_state(hwnd);
            if let Err(err) = refresh_window_context(state) {
                state.last_error = Some(format!("{err:#}"));
            }
            let _ = sync_controller_timer(state);
            let _ = tick_controller(state);
            LRESULT(0)
        }
        WM_POWERBROADCAST => {
            let state = get_state(hwnd);
            match wparam.0 as u32 {
                PBT_APMSUSPEND => {
                    state.suspended = true;
                    clear_active_outputs(state);
                    if let Some(ref mut ov) = state.overlay {
                        ov.hide();
                    }
                    let _ = sync_controller_timer(state);
                }
                PBT_APMRESUMEAUTOMATIC => {
                    state.suspended = false;
                    state.controller_connected = false;
                    let _ = refresh_window_context(state);
                    let _ = sync_controller_timer(state);
                }
                _ => {}
            }
            LRESULT(1)
        }
        WM_WTSSESSION_CHANGE => {
            let state = get_state(hwnd);
            match wparam.0 as u32 {
                WTS_SESSION_LOCK => {
                    state.suspended = true;
                    clear_active_outputs(state);
                    if let Some(ref mut ov) = state.overlay {
                        ov.hide();
                    }
                    let _ = sync_controller_timer(state);
                }
                WTS_SESSION_UNLOCK => {
                    state.suspended = false;
                    let _ = refresh_window_context(state);
                    let _ = sync_controller_timer(state);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            let state = get_state(hwnd);
            if let Some(ref mut ov) = state.overlay {
                let _ = ov.reposition();
            }
            let _ = refresh_window_context(state);
            let _ = sync_controller_timer(state);
            let _ = tick_controller(state);
            LRESULT(0)
        }
        WM_TRAYICON => {
            if matches!(lparam.0 as u32, WM_RBUTTONUP | WM_LBUTTONUP) {
                let state = get_state(hwnd);
                let _ = show_context_menu(state);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
            if !state_ptr.is_null() {
                let mut state = Box::from_raw(state_ptr);
                FOREGROUND_TARGET_HWND.store(0, Ordering::Relaxed);
                if state.controller_timer_interval_ms.is_some() {
                    let _ = KillTimer(state.hwnd, TIMER_CONTROLLER);
                }
                uninstall_foreground_hook(&mut state);
                if let Some(ref mut ov) = state.overlay {
                    ov.destroy();
                }
                let _ = input::release_all(&mut state.held_key_refs);
                let _ = delete_tray_icon(state.hwnd);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn handle_command(state: &mut AppState, command_id: usize) {
    match command_id {
        ID_TOGGLE_ENABLE => {
            state.enabled = !state.enabled;
            if !state.enabled {
                clear_active_outputs(state);
            }
            let _ = sync_controller_timer(state);
        }
        ID_TOGGLE_STARTUP => {
            match startup::set_startup_enabled(&state.exe_path, !state.startup_enabled) {
                Ok(_) => state.startup_enabled = !state.startup_enabled,
                Err(err) => report_error(state, "Startup toggle failed", &err),
            }
        }
        ID_RELOAD_CONFIG => match load_config(&state.config_path) {
            Ok(config) => {
                state.config = config;
                state.last_error = None;
                clear_active_outputs(state);
                state.pressed_buttons.clear();
                if let Some(ref mut ov) = state.overlay {
                    ov.destroy();
                    state.overlay = None;
                }
                if state.config.overlay {
                    match overlay::OverlayWindow::create(
                        state.config.overlay_position,
                        state.config.overlay_opacity,
                        state.config.nintendo_layout,
                    ) {
                        Ok(ov) => {
                            state.overlay = Some(ov);
                            state.overlay_visible = true;
                        }
                        Err(err) => report_error(state, "Overlay creation failed", &err),
                    }
                }
                // Re-match the foreground window. If it doesn't match (e.g. editor
                // is still foreground during auto-reload), fall back to the last
                // known match so the timer keeps running.
                let _ = refresh_window_context(state);
                if state.active_profile_index.is_none() {
                    if let Some(prev) = state.last_matched_profile_index {
                        if prev < state.config.profiles.len() {
                            state.active_profile_index = Some(prev);
                        }
                    }
                }
                let _ = sync_controller_timer(state);
            }
            Err(err) => report_error(state, "Reload config failed", &err),
        },
        ID_OPEN_CONFIG => {
            let file = wide_null(&state.config_path.display().to_string());
            let _ = ShellExecuteW(
                state.hwnd,
                w!("open"),
                PCWSTR(file.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
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
        ID_EXIT => {
            let _ = DestroyWindow(state.hwnd);
        }
        player if (ID_PLAYER_BASE + 1..=ID_PLAYER_BASE + 4).contains(&player) => {
            state.config.controller_player = (player - ID_PLAYER_BASE) as u32;
            clear_active_outputs(state);
            state.controller_connected = false;
            let _ = sync_controller_timer(state);
        }
        _ => {}
    }
}

unsafe fn tick_controller(state: &mut AppState) -> Result<()> {
    // Safety net: re-check foreground window in case hook missed a change
    let fg_title = window_title(GetForegroundWindow());
    if fg_title != state.current_title {
        refresh_window_context(state)?;
        sync_controller_timer(state)?;
    }

    if !state.enabled || state.in_menu {
        clear_active_outputs(state);
        if let Some(ref mut ov) = state.overlay {
            ov.hide();
        }
        return Ok(());
    }

    let Some(profile_index) = state.active_profile_index else {
        clear_active_outputs(state);
        if let Some(ref mut ov) = state.overlay {
            ov.hide();
        }
        return Ok(());
    };

    let connected = poll_controller_buttons(
        state.config.controller_player - 1,
        &mut state.pressed_buttons,
        state.config.nintendo_layout,
    );
    if !connected {
        if state.controller_connected {
            state.controller_connected = false;
            sync_controller_timer(state)?;
        }
        clear_active_outputs(state);
        if let Some(ref mut ov) = state.overlay {
            if state.overlay_visible {
                let profile_data = state
                    .config
                    .profiles
                    .get(profile_index)
                    .map(|p| (profile_index, p));
                ov.update(&state.pressed_buttons, profile_data, false);
            }
        }
        return Ok(());
    }
    if !state.controller_connected {
        state.controller_connected = true;
        state.last_input_tick = None;
        sync_controller_timer(state)?;
    }

    let had_input = !state.pressed_buttons.is_empty();
    let was_idle = match state.last_input_tick {
        Some(tick) => tick.elapsed().as_millis() >= IDLE_THRESHOLD,
        None => true,
    };
    if had_input {
        state.last_input_tick = Some(Instant::now());
    }
    if had_input && was_idle {
        sync_controller_timer(state)?;
    }

    let profile = &state.config.profiles[profile_index];
    let resolved = mapper::resolve_active_mappings(profile, &state.pressed_buttons);
    state.scratch_mapping_ids.clear();
    state
        .scratch_mapping_ids
        .extend(resolved.iter().map(|mapping| mapping.id));

    for mapping in &resolved {
        if mapping.mode == OutputMode::Tap && !state.active_mapping_ids.contains(&mapping.id) {
            input::tap_keys(&mapping.keys)?;
        }
    }

    let mut held_index = 0usize;
    while held_index < state.held_mapping_ids.len() {
        let held_id = state.held_mapping_ids[held_index];
        if !state.scratch_mapping_ids.contains(&held_id) {
            if let Some(mapping) = find_mapping(profile, held_id) {
                input::release_keys(&mapping.keys, &mut state.held_key_refs)?;
            }
            state.held_mapping_ids.swap_remove(held_index);
        } else {
            held_index += 1;
        }
    }

    for mapping in &resolved {
        if mapping.mode == OutputMode::Hold && !state.held_mapping_ids.contains(&mapping.id) {
            input::press_keys(&mapping.keys, &mut state.held_key_refs)?;
            state.held_mapping_ids.push(mapping.id);
        }
    }

    state.active_mapping_ids.clear();
    state
        .active_mapping_ids
        .extend(state.scratch_mapping_ids.iter().copied());

    if let Some(ref mut ov) = state.overlay {
        if state.overlay_visible {
            let profile_data = state
                .config
                .profiles
                .get(profile_index)
                .map(|p| (profile_index, p));
            ov.update(&state.pressed_buttons, profile_data, true);
            ov.show();
        }
    }

    if !had_input && !was_idle {
        if let Some(tick) = state.last_input_tick {
            if tick.elapsed().as_millis() >= IDLE_THRESHOLD {
                sync_controller_timer(state)?;
            }
        }
    }

    Ok(())
}

unsafe fn refresh_window_context(state: &mut AppState) -> Result<()> {
    let title = window_title(GetForegroundWindow());
    let previous_profile = state.active_profile_index;
    state.current_title = title;
    state.active_profile_index = state
        .config
        .profiles
        .iter()
        .position(|profile| profile.regex.is_match(&state.current_title));

    if state.active_profile_index.is_some() {
        state.last_matched_title.clone_from(&state.current_title);
        state.last_matched_profile_index = state.active_profile_index;
    }

    if state.active_profile_index != previous_profile {
        clear_active_outputs(state);
        if state.active_profile_index.is_some() {
            // Boost polling to active rate (16ms) for immediate responsiveness
            state.last_input_tick = Some(Instant::now());
        }
    }

    if let Some(ref mut ov) = state.overlay {
        if let (Some(idx), true) = (state.active_profile_index, state.overlay_visible) {
            let profile_data = state.config.profiles.get(idx).map(|p| (idx, p));
            ov.update(
                &state.pressed_buttons,
                profile_data,
                state.controller_connected,
            );
            ov.show();
        } else {
            ov.hide();
        }
    }

    Ok(())
}

unsafe fn clear_active_outputs(state: &mut AppState) {
    let _ = input::release_all(&mut state.held_key_refs);
    state.active_mapping_ids.clear();
    state.held_mapping_ids.clear();
}

fn find_mapping(profile: &CompiledProfile, id: usize) -> Option<&CompiledMapping> {
    profile.mappings.iter().find(|mapping| mapping.id == id)
}

unsafe fn show_context_menu(state: &mut AppState) -> Result<()> {
    state.in_menu = true;
    let menu = CreatePopupMenu()?;
    let player_menu = CreatePopupMenu()?;
    let controller_label = wide_null("Controller");
    let enabled_label = if state.enabled {
        "Disable mappings"
    } else {
        "Enable mappings"
    };
    let startup_label = if state.startup_enabled {
        "Startup: On"
    } else {
        "Startup: Off"
    };

    let status_parts = [
        if state.enabled { "on" } else { "off" },
        if state.controller_connected {
            "connected"
        } else {
            "disconnected"
        },
    ];
    let timer_info = match state.controller_timer_interval_ms {
        Some(ms) => format!("{ms}ms"),
        None => "stopped".to_string(),
    };
    let status_label = format!(
        "Status: {} | P{} {} | timer {}",
        status_parts[0], state.config.controller_player, status_parts[1], timer_info,
    );
    append_text_item(menu, 0, &status_label, MF_STRING | MF_GRAYED)?;

    let rule_label = match state.last_matched_profile_index {
        Some(index) => {
            let profile = &state.config.profiles[index];
            format!("Rule: {} [{}]", profile.name, profile.title_regex)
        }
        None => "Rule: none".to_string(),
    };
    append_text_item(menu, 0, &rule_label, MF_STRING | MF_GRAYED)?;

    if !state.last_matched_title.is_empty() {
        append_text_item(
            menu,
            0,
            &format!("Window: {}", state.last_matched_title),
            MF_STRING | MF_GRAYED,
        )?;
    }

    if let Some(error) = &state.last_error {
        let single_line = summarize_error(error);
        append_text_item(
            menu,
            0,
            &format!("Last error: {single_line}"),
            MF_STRING | MF_GRAYED,
        )?;
    }

    AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null())?;
    append_text_item(menu, ID_TOGGLE_ENABLE, enabled_label, MF_STRING)?;
    if state.overlay.is_some() {
        let overlay_label = if state.overlay_visible {
            "Hide overlay"
        } else {
            "Show overlay"
        };
        append_text_item(menu, ID_TOGGLE_OVERLAY, overlay_label, MF_STRING)?;
    }

    for player in 1..=4usize {
        let flags = if state.config.controller_player as usize == player {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        append_text_item(
            player_menu,
            ID_PLAYER_BASE + player,
            &format!("Player {player}"),
            flags,
        )?;
    }

    AppendMenuW(
        menu,
        MF_POPUP,
        player_menu.0 as usize,
        PCWSTR(controller_label.as_ptr()),
    )?;
    append_text_item(menu, ID_TOGGLE_STARTUP, startup_label, MF_STRING)?;
    append_text_item(menu, ID_RELOAD_CONFIG, "Reload config", MF_STRING)?;
    append_text_item(menu, ID_OPEN_CONFIG, "Open config.toml", MF_STRING)?;
    AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null())?;
    append_text_item(menu, ID_EXIT, "Exit", MF_STRING)?;

    let mut point = POINT::default();
    GetCursorPos(&mut point)?;
    let _ = SetForegroundWindow(state.hwnd);
    let _ = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN,
        point.x,
        point.y,
        0,
        state.hwnd,
        None,
    );
    let _ = PostMessageW(state.hwnd, WM_NULL, WPARAM(0), LPARAM(0));
    DestroyMenu(player_menu)?;
    DestroyMenu(menu)?;
    state.in_menu = false;
    Ok(())
}

unsafe fn add_tray_icon(state: &AppState) -> Result<()> {
    let mut notify = NOTIFYICONDATAW::default();
    notify.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    notify.hWnd = state.hwnd;
    notify.uID = 1;
    notify.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    notify.uCallbackMessage = WM_TRAYICON;
    notify.hIcon = state.app_icon;
    copy_wide_into_buf("ZeroMapper", &mut notify.szTip);
    Shell_NotifyIconW(NIM_ADD, &notify).ok()?;
    Ok(())
}

unsafe fn delete_tray_icon(hwnd: HWND) -> Result<()> {
    let mut notify = NOTIFYICONDATAW::default();
    notify.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    notify.hWnd = hwnd;
    notify.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &notify).ok()?;
    Ok(())
}

unsafe fn append_text_item(
    menu: HMENU,
    id: usize,
    text: &str,
    flags: MENU_ITEM_FLAGS,
) -> Result<()> {
    let wide = wide_null(text);
    AppendMenuW(menu, flags, id, PCWSTR(wide.as_ptr()))?;
    Ok(())
}

unsafe fn poll_controller_buttons(
    user_index: u32,
    buttons: &mut BTreeSet<PadButton>,
    nintendo_layout: bool,
) -> bool {
    let mut state = XINPUT_STATE::default();
    if XInputGetState(user_index, &mut state) != 0 {
        buttons.clear();
        return false;
    }

    let gamepad = state.Gamepad;
    let raw = gamepad.wButtons;
    let trigger_threshold = 30;
    buttons.clear();

    // Nintendo layout: physical A(right)=XInput B, physical B(bottom)=XInput A
    // So when nintendo_layout is true, we swap A↔B and X↔Y to match printed labels.
    let (btn_a, btn_b, btn_x, btn_y) = if nintendo_layout {
        (PadButton::B, PadButton::A, PadButton::Y, PadButton::X)
    } else {
        (PadButton::A, PadButton::B, PadButton::X, PadButton::Y)
    };

    if (raw & XINPUT_GAMEPAD_A).0 != 0 {
        buttons.insert(btn_a);
    }
    if (raw & XINPUT_GAMEPAD_B).0 != 0 {
        buttons.insert(btn_b);
    }
    if (raw & XINPUT_GAMEPAD_X).0 != 0 {
        buttons.insert(btn_x);
    }
    if (raw & XINPUT_GAMEPAD_Y).0 != 0 {
        buttons.insert(btn_y);
    }
    if (raw & XINPUT_GAMEPAD_LEFT_SHOULDER).0 != 0 {
        buttons.insert(PadButton::Lb);
    }
    if (raw & XINPUT_GAMEPAD_RIGHT_SHOULDER).0 != 0 {
        buttons.insert(PadButton::Rb);
    }
    if (raw & XINPUT_GAMEPAD_BACK).0 != 0 {
        buttons.insert(PadButton::Back);
    }
    if (raw & XINPUT_GAMEPAD_START).0 != 0 {
        buttons.insert(PadButton::Start);
    }
    if (raw & XINPUT_GAMEPAD_LEFT_THUMB).0 != 0 {
        buttons.insert(PadButton::L3);
    }
    if (raw & XINPUT_GAMEPAD_RIGHT_THUMB).0 != 0 {
        buttons.insert(PadButton::R3);
    }
    if (raw & XINPUT_GAMEPAD_DPAD_UP).0 != 0 {
        buttons.insert(PadButton::DpadUp);
    }
    if (raw & XINPUT_GAMEPAD_DPAD_DOWN).0 != 0 {
        buttons.insert(PadButton::DpadDown);
    }
    if (raw & XINPUT_GAMEPAD_DPAD_LEFT).0 != 0 {
        buttons.insert(PadButton::DpadLeft);
    }
    if (raw & XINPUT_GAMEPAD_DPAD_RIGHT).0 != 0 {
        buttons.insert(PadButton::DpadRight);
    }
    // Some compact controllers (8BitDo Zero 2, etc.) report Dpad as left stick
    const STICK_DPAD_THRESHOLD: i16 = 16384;
    if gamepad.sThumbLY > STICK_DPAD_THRESHOLD {
        buttons.insert(PadButton::DpadUp);
    }
    if gamepad.sThumbLY < -STICK_DPAD_THRESHOLD {
        buttons.insert(PadButton::DpadDown);
    }
    if gamepad.sThumbLX < -STICK_DPAD_THRESHOLD {
        buttons.insert(PadButton::DpadLeft);
    }
    if gamepad.sThumbLX > STICK_DPAD_THRESHOLD {
        buttons.insert(PadButton::DpadRight);
    }
    if gamepad.bLeftTrigger >= trigger_threshold {
        buttons.insert(PadButton::Lt);
    }
    if gamepad.bRightTrigger >= trigger_threshold {
        buttons.insert(PadButton::Rt);
    }

    true
}

fn window_title(hwnd: HWND) -> String {
    if hwnd.0.is_null() {
        return String::new();
    }
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..copied as usize])
}

fn get_state(hwnd: HWND) -> &'static mut AppState {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };
    unsafe { &mut *ptr }
}

fn report_error(state: &mut AppState, title: &str, err: &anyhow::Error) {
    let message = format!("{title}\n\n{err:#}");
    state.last_error = Some(message.clone());
    show_error_popup(state.hwnd, title, &message);
}

fn summarize_error(message: &str) -> String {
    let line = message
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(message);
    truncate_for_menu(line, 72)
}

fn truncate_for_menu(text: &str, max_chars: usize) -> String {
    let mut truncated = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= max_chars {
            truncated.push_str("...");
            return truncated;
        }
        truncated.push(ch);
    }
    truncated
}

fn show_error_popup(hwnd: HWND, title: &str, message: &str) {
    let title_w = wide_null(title);
    let message_w = wide_null(message);
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            hwnd,
            PCWSTR(message_w.as_ptr()),
            PCWSTR(title_w.as_ptr()),
            windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}

unsafe fn install_foreground_hook(state: &mut AppState) -> Result<()> {
    let hook = SetWinEventHook(
        EVENT_SYSTEM_FOREGROUND,
        EVENT_SYSTEM_FOREGROUND,
        None,
        Some(foreground_event_proc),
        0,
        0,
        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
    );
    if hook.is_invalid() {
        anyhow::bail!("SetWinEventHook failed");
    }
    state.foreground_hook = hook;
    Ok(())
}

unsafe fn uninstall_foreground_hook(state: &mut AppState) {
    if !state.foreground_hook.is_invalid() {
        let _ = UnhookWinEvent(state.foreground_hook);
        state.foreground_hook = HWINEVENTHOOK::default();
    }
}

unsafe fn sync_controller_timer(state: &mut AppState) -> Result<()> {
    let desired = desired_controller_interval(state);
    if desired == state.controller_timer_interval_ms {
        return Ok(());
    }

    if state.controller_timer_interval_ms.is_some() {
        let _ = KillTimer(state.hwnd, TIMER_CONTROLLER);
        state.controller_timer_interval_ms = None;
    }

    if let Some(interval_ms) = desired {
        let timer = SetTimer(state.hwnd, TIMER_CONTROLLER, interval_ms, None);
        if timer == 0 {
            anyhow::bail!("SetTimer failed for controller polling");
        }
        state.controller_timer_interval_ms = Some(interval_ms);
    }

    Ok(())
}

fn desired_controller_interval(state: &AppState) -> Option<u32> {
    if state.suspended {
        return None;
    }

    if !state.enabled || state.active_profile_index.is_none() {
        return Some(POLL_INTERVAL_BACKGROUND_MS);
    }

    if !state.controller_connected {
        return Some(POLL_INTERVAL_DISCONNECTED_MS);
    }

    let idle = match state.last_input_tick {
        Some(tick) => tick.elapsed().as_millis() >= IDLE_THRESHOLD,
        None => true,
    };

    Some(if idle {
        POLL_INTERVAL_IDLE_MS
    } else {
        POLL_INTERVAL_ACTIVE_MS
    })
}

unsafe extern "system" fn foreground_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _event_time: u32,
) {
    let target = FOREGROUND_TARGET_HWND.load(Ordering::Relaxed);
    if target != 0 {
        let _ = PostMessageW(
            HWND(target as *mut c_void),
            WM_FOREGROUND_CHANGED,
            WPARAM(0),
            LPARAM(0),
        );
    }
}

fn copy_wide_into_buf(text: &str, buffer: &mut [u16]) {
    let wide = wide_null(text);
    let len = wide.len().min(buffer.len());
    buffer[..len].copy_from_slice(&wide[..len]);
    if len == buffer.len() {
        buffer[buffer.len() - 1] = 0;
    }
}

fn spawn_config_watcher(hwnd: HWND, config_path: &PathBuf) {
    use windows::Win32::Storage::FileSystem::{
        FILE_NOTIFY_CHANGE_LAST_WRITE, FindFirstChangeNotificationW, FindNextChangeNotification,
    };
    use windows::Win32::System::Threading::WaitForSingleObject;

    let dir = config_path.parent().unwrap_or(config_path).to_path_buf();
    let file_name = config_path.file_name().unwrap_or_default().to_os_string();
    let hwnd_raw = hwnd.0 as isize;

    std::thread::spawn(move || unsafe {
        let dir_wide = wide_null(&dir.display().to_string());
        let handle = FindFirstChangeNotificationW(
            PCWSTR(dir_wide.as_ptr()),
            false,
            FILE_NOTIFY_CHANGE_LAST_WRITE,
        );
        let Ok(handle) = handle else { return };

        // Track mtime to avoid spurious notifications
        let mut last_mtime = std::fs::metadata(dir.join(&file_name))
            .and_then(|m| m.modified())
            .ok();

        loop {
            let _ = WaitForSingleObject(handle, u32::MAX);
            // Delay for editors that write in multiple steps (truncate+write)
            std::thread::sleep(std::time::Duration::from_millis(300));

            let current_mtime = std::fs::metadata(dir.join(&file_name))
                .and_then(|m| m.modified())
                .ok();
            if current_mtime != last_mtime {
                last_mtime = current_mtime;
                let hwnd = HWND(hwnd_raw as *mut _);
                let _ = PostMessageW(hwnd, WM_CONFIG_CHANGED, WPARAM(0), LPARAM(0));
            }

            let _ = FindNextChangeNotification(handle);
        }
    });
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
