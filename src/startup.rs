use anyhow::{Context, Result, anyhow};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyW, RegDeleteValueW,
    RegGetValueW, RegSetValueExW,
};
use windows::core::PCWSTR;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_NAME: &str = "ZeroMapper";

pub fn is_startup_enabled() -> Result<bool> {
    let subkey = wide_null(RUN_KEY);
    let value_name = wide_null(APP_NAME);
    let mut bytes = [0u8; 2048];
    let mut size = bytes.len() as u32;

    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(bytes.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };

    match result {
        code if code == WIN32_ERROR(0) => Ok(true),
        code if code == ERROR_FILE_NOT_FOUND => Ok(false),
        code => Err(anyhow!("RegGetValueW failed with code {}", code.0))
            .context("failed to query startup registry value"),
    }
}

pub fn set_startup_enabled(exe_path: &str, enabled: bool) -> Result<()> {
    let subkey = wide_null(RUN_KEY);
    let value_name = wide_null(APP_NAME);
    let mut key = HKEY::default();
    let command = format!("\"{exe_path}\"");

    unsafe {
        let create_result = RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), &mut key);
        if create_result != WIN32_ERROR(0) {
            return Err(anyhow!(
                "RegCreateKeyW failed with code {}",
                create_result.0
            ))
            .context("failed to open startup registry key");
        }

        let update_result = if enabled {
            let data = wide_null(&command);
            let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2);
            RegSetValueExW(key, PCWSTR(value_name.as_ptr()), 0, REG_SZ, Some(bytes))
        } else {
            RegDeleteValueW(key, PCWSTR(value_name.as_ptr()))
        };

        let close_result = RegCloseKey(key);
        if close_result != WIN32_ERROR(0) {
            return Err(anyhow!("RegCloseKey failed with code {}", close_result.0));
        }

        match update_result {
            code if code == WIN32_ERROR(0) => Ok(()),
            code if !enabled && code == ERROR_FILE_NOT_FOUND => Ok(()),
            code => Err(anyhow!("registry update failed with code {}", code.0))
                .context("failed to update startup registry value"),
        }
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
