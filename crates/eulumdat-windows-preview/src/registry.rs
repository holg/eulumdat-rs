//! Registry operations for Preview Handler registration
//!
//! This module handles registering and unregistering the preview handler
//! with the Windows Shell.

#![cfg(windows)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT,
    HKEY_LOCAL_MACHINE, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

use crate::CLSID_EULUMDAT_PREVIEW;

/// Get the path to this DLL
fn get_dll_path() -> Result<String, anyhow::Error> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };

    let mut buffer = vec![0u16; 512];

    unsafe {
        // Get the module handle for THIS DLL by using a function address within it
        let mut hmodule = HMODULE::default();
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            windows::core::PCWSTR::from_raw(get_dll_path as *const u16),
            &mut hmodule,
        )
        .map_err(|e| anyhow::anyhow!("Failed to get DLL module handle: {}", e))?;

        let len = GetModuleFileNameW(hmodule, &mut buffer);
        if len == 0 {
            return Err(anyhow::anyhow!("Failed to get module file name"));
        }

        let path = OsString::from_wide(&buffer[..len as usize]);
        Ok(path.to_string_lossy().into_owned())
    }
}

/// Convert a Rust string to a null-terminated wide string
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// CLSID as a registry string
fn clsid_string() -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        CLSID_EULUMDAT_PREVIEW.data1,
        CLSID_EULUMDAT_PREVIEW.data2,
        CLSID_EULUMDAT_PREVIEW.data3,
        CLSID_EULUMDAT_PREVIEW.data4[0],
        CLSID_EULUMDAT_PREVIEW.data4[1],
        CLSID_EULUMDAT_PREVIEW.data4[2],
        CLSID_EULUMDAT_PREVIEW.data4[3],
        CLSID_EULUMDAT_PREVIEW.data4[4],
        CLSID_EULUMDAT_PREVIEW.data4[5],
        CLSID_EULUMDAT_PREVIEW.data4[6],
        CLSID_EULUMDAT_PREVIEW.data4[7],
    )
}

/// Set a registry string value
unsafe fn set_reg_string(key: HKEY, name: Option<&str>, value: &str) -> Result<(), anyhow::Error> {
    let name_wide = name.map(to_wide);
    let name_ptr = match &name_wide {
        Some(n) => PCWSTR::from_raw(n.as_ptr()),
        None => PCWSTR::null(),
    };

    let value_wide = to_wide(value);
    let value_bytes: Vec<u8> = value_wide.iter().flat_map(|w| w.to_le_bytes()).collect();

    let result = RegSetValueExW(key, name_ptr, 0, REG_SZ, Some(&value_bytes));

    if result != ERROR_SUCCESS {
        return Err(anyhow::anyhow!("RegSetValueExW failed: {:?}", result));
    }

    Ok(())
}

/// Create a registry key
unsafe fn create_key(parent: HKEY, path: &str) -> Result<HKEY, anyhow::Error> {
    use windows::Win32::System::Registry::REG_CREATE_KEY_DISPOSITION;

    let path_wide = to_wide(path);
    let mut key = HKEY::default();
    let mut disposition = REG_CREATE_KEY_DISPOSITION::default();

    let result = RegCreateKeyExW(
        parent,
        PCWSTR::from_raw(path_wide.as_ptr()),
        0,
        PCWSTR::null(),
        REG_OPTION_NON_VOLATILE,
        KEY_WRITE,
        None,
        &mut key,
        Some(&mut disposition),
    );

    if result != ERROR_SUCCESS {
        return Err(anyhow::anyhow!("RegCreateKeyExW failed: {:?}", result));
    }

    Ok(key)
}

/// Set a DWORD registry value
unsafe fn set_reg_dword(key: HKEY, name: &str, value: u32) -> Result<(), anyhow::Error> {
    use windows::Win32::System::Registry::REG_DWORD;

    let name_wide = to_wide(name);
    let value_bytes = value.to_le_bytes();

    let result = RegSetValueExW(
        key,
        PCWSTR::from_raw(name_wide.as_ptr()),
        0,
        REG_DWORD,
        Some(&value_bytes),
    );

    if result != ERROR_SUCCESS {
        return Err(anyhow::anyhow!("RegSetValueExW DWORD failed: {:?}", result));
    }

    Ok(())
}

/// Register the preview handler with Windows
pub fn register_preview_handler() -> Result<(), anyhow::Error> {
    let clsid = clsid_string();
    let dll_path = get_dll_path()?;

    unsafe {
        // Register CLSID
        // HKEY_CLASSES_ROOT\CLSID\{clsid}
        let clsid_path = format!(r"CLSID\{}", clsid);
        let clsid_key = create_key(HKEY_CLASSES_ROOT, &clsid_path)?;
        set_reg_string(clsid_key, None, "Eulumdat Preview Handler")?;

        // Set AppID to same as CLSID (common pattern for preview handlers)
        set_reg_string(clsid_key, Some("AppID"), &clsid)?;

        // Disable low integrity process isolation - needed for preview handlers
        // Without this, the handler runs in a sandbox that can't write to C:\
        set_reg_dword(clsid_key, "DisableLowILProcessIsolation", 1)?;

        // InprocServer32
        let inproc_key = create_key(clsid_key, "InprocServer32")?;
        set_reg_string(inproc_key, None, &dll_path)?;
        set_reg_string(inproc_key, Some("ThreadingModel"), "Apartment")?;
        let _ = RegCloseKey(inproc_key);

        let _ = RegCloseKey(clsid_key);

        // Register for .ldt files
        // HKEY_CLASSES_ROOT\.ldt\ShellEx\{8895b1c6-b41f-4c1c-a562-0d564250836f}
        let ldt_key = create_key(HKEY_CLASSES_ROOT, r".ldt")?;
        set_reg_string(ldt_key, None, "LDT.EulumdatFile")?;
        set_reg_string(ldt_key, Some("Content Type"), "application/x-eulumdat")?;

        let ldt_shellex = create_key(ldt_key, r"ShellEx\{8895b1c6-b41f-4c1c-a562-0d564250836f}")?;
        set_reg_string(ldt_shellex, None, &clsid)?;
        let _ = RegCloseKey(ldt_shellex);
        let _ = RegCloseKey(ldt_key);

        // Register for .ies files
        let ies_key = create_key(HKEY_CLASSES_ROOT, r".ies")?;
        set_reg_string(ies_key, None, "IES.PhotometricFile")?;
        set_reg_string(ies_key, Some("Content Type"), "application/x-ies")?;

        let ies_shellex = create_key(ies_key, r"ShellEx\{8895b1c6-b41f-4c1c-a562-0d564250836f}")?;
        set_reg_string(ies_shellex, None, &clsid)?;
        let _ = RegCloseKey(ies_shellex);
        let _ = RegCloseKey(ies_key);

        // Register the preview handler in the approved list
        // HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\PreviewHandlers
        let preview_handlers = create_key(
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\PreviewHandlers",
        )?;
        set_reg_string(preview_handlers, Some(&clsid), "Eulumdat Preview Handler")?;
        let _ = RegCloseKey(preview_handlers);
    }

    Ok(())
}

/// Unregister the preview handler from Windows
pub fn unregister_preview_handler() -> Result<(), anyhow::Error> {
    let clsid = clsid_string();

    unsafe {
        // Remove .ldt ShellEx
        let ldt_shellex = to_wide(r".ldt\ShellEx\{8895b1c6-b41f-4c1c-a562-0d564250836f}");
        let _ = RegDeleteKeyW(HKEY_CLASSES_ROOT, PCWSTR::from_raw(ldt_shellex.as_ptr()));

        // Remove .ies ShellEx
        let ies_shellex = to_wide(r".ies\ShellEx\{8895b1c6-b41f-4c1c-a562-0d564250836f}");
        let _ = RegDeleteKeyW(HKEY_CLASSES_ROOT, PCWSTR::from_raw(ies_shellex.as_ptr()));

        // Remove CLSID entries
        let inproc_path = to_wide(&format!(r"CLSID\{}\InprocServer32", clsid));
        let _ = RegDeleteKeyW(HKEY_CLASSES_ROOT, PCWSTR::from_raw(inproc_path.as_ptr()));

        let clsid_path = to_wide(&format!(r"CLSID\{}", clsid));
        let _ = RegDeleteKeyW(HKEY_CLASSES_ROOT, PCWSTR::from_raw(clsid_path.as_ptr()));
    }

    Ok(())
}
