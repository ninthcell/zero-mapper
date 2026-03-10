use std::collections::HashMap;

use anyhow::{Result, anyhow};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
};

pub fn tap_keys(keys: &[u16]) -> Result<()> {
    let count = keys.len() * 2;
    let mut inputs = Vec::with_capacity(count);
    for &key in keys {
        inputs.push(make_input(key, false));
    }
    for &key in keys.iter().rev() {
        inputs.push(make_input(key, true));
    }
    send_inputs(&inputs)
}

pub fn press_keys(keys: &[u16], held_refs: &mut HashMap<u16, u32>) -> Result<()> {
    let mut inputs = Vec::new();
    for &key in keys {
        let counter = held_refs.entry(key).or_insert(0);
        if *counter == 0 {
            inputs.push(make_input(key, false));
        }
        *counter += 1;
    }
    if !inputs.is_empty() {
        send_inputs(&inputs)?;
    }
    Ok(())
}

pub fn release_keys(keys: &[u16], held_refs: &mut HashMap<u16, u32>) -> Result<()> {
    let mut inputs = Vec::new();
    for &key in keys.iter().rev() {
        if let Some(counter) = held_refs.get_mut(&key) {
            *counter = counter.saturating_sub(1);
            if *counter == 0 {
                held_refs.remove(&key);
                inputs.push(make_input(key, true));
            }
        }
    }
    if !inputs.is_empty() {
        send_inputs(&inputs)?;
    }
    Ok(())
}

pub fn release_all(held_refs: &mut HashMap<u16, u32>) -> Result<()> {
    let mut keys: Vec<u16> = held_refs.keys().copied().collect();
    keys.sort_unstable();
    let inputs: Vec<INPUT> = keys.iter().rev().map(|&key| make_input(key, true)).collect();
    held_refs.clear();
    if !inputs.is_empty() {
        send_inputs(&inputs)?;
    }
    Ok(())
}

fn make_input(vk: u16, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_inputs(inputs: &[INPUT]) -> Result<()> {
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(anyhow!("SendInput failed: sent {sent}/{}", inputs.len()));
    }
    Ok(())
}
