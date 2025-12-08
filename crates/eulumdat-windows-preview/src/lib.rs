//! Windows Preview Handler for EULUMDAT (LDT) and IES photometric files
//!
//! This crate implements a Windows Shell Preview Handler that displays
//! polar diagrams for LDT/IES files in the File Explorer preview pane.
//!
//! ## Supported Architectures
//!
//! - x64 (x86_64-pc-windows-msvc)
//! - ARM64 (aarch64-pc-windows-msvc)
//!
//! ## Installation
//!
//! ```powershell
//! # Build for your architecture
//! cargo build --release -p eulumdat-preview --target x86_64-pc-windows-msvc   # x64
//! cargo build --release -p eulumdat-preview --target aarch64-pc-windows-msvc  # ARM64
//!
//! # Register (run as Administrator)
//! regsvr32 target\x86_64-pc-windows-msvc\release\eulumdat_preview.dll
//! # or
//! regsvr32 target\aarch64-pc-windows-msvc\release\eulumdat_preview.dll
//!
//! # Unregister
//! regsvr32 /u target\<arch>\release\eulumdat_preview.dll
//! ```

#![cfg(windows)]

mod handler;
mod registry;
mod render;

use std::ffi::c_void;
use windows::core::{implement, Error as WinError, IUnknown, Interface, Result as WinResult, GUID};
use windows::Win32::Foundation::{
    BOOL, CLASS_E_CLASSNOTAVAILABLE, E_POINTER, E_UNEXPECTED, HINSTANCE, S_OK,
};

const DLL_PROCESS_ATTACH: u32 = 1;
const DLL_PROCESS_DETACH: u32 = 0;

/// Debug logging to file
fn debug_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("C:\\eulumdat_preview_debug.log")
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

/// DLL entry point - logs when DLL is loaded/unloaded
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    _hinst: HINSTANCE,
    reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            debug_log("DllMain: DLL_PROCESS_ATTACH");
        }
        DLL_PROCESS_DETACH => {
            debug_log("DllMain: DLL_PROCESS_DETACH");
        }
        _ => {}
    }
    BOOL::from(true)
}

use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};

use handler::EulumdatPreviewHandler;

/// CLSID for the Eulumdat Preview Handler
/// {A1B2C3D4-E5F6-7890-ABCD-EF1234567890}
pub const CLSID_EULUMDAT_PREVIEW: GUID = GUID::from_values(
    0xA1B2C3D4,
    0xE5F6,
    0x7890,
    [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90],
);

/// Class factory for creating preview handler instances
#[implement(IClassFactory)]
pub struct EulumdatClassFactory;

impl IClassFactory_Impl for EulumdatClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> WinResult<()> {
        debug_log("CreateInstance called");

        // COM aggregation not supported
        if punkouter.is_some() {
            debug_log("CreateInstance: Aggregation not supported");
            return Err(WinError::from(
                windows::Win32::Foundation::CLASS_E_NOAGGREGATION,
            ));
        }

        unsafe {
            if ppvobject.is_null() {
                debug_log("CreateInstance: Null ppvobject");
                return Err(WinError::from(E_POINTER));
            }
            *ppvobject = std::ptr::null_mut();

            debug_log("CreateInstance: Creating handler...");
            let handler: IUnknown = EulumdatPreviewHandler::new().into();
            let result = handler.query(&*riid, ppvobject);
            debug_log(&format!("CreateInstance: query result = {:?}", result));
            result.ok()
        }
    }

    fn LockServer(&self, _flock: windows::Win32::Foundation::BOOL) -> WinResult<()> {
        Ok(())
    }
}

/// DLL entry point for COM class factory
///
/// # Safety
/// This function is called by COM to get class factories
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> windows::core::HRESULT {
    debug_log("DllGetClassObject called");

    if ppv.is_null() {
        debug_log("DllGetClassObject: Null ppv");
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    if *rclsid != CLSID_EULUMDAT_PREVIEW {
        debug_log("DllGetClassObject: Wrong CLSID");
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    debug_log("DllGetClassObject: Creating factory...");
    let factory: IClassFactory = EulumdatClassFactory.into();
    let result = factory.query(&*riid, ppv);
    debug_log(&format!("DllGetClassObject: result = {:?}", result));
    result
}

/// Check if DLL can be unloaded
///
/// # Safety
/// Called by COM to check if DLL can be freed
#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> windows::core::HRESULT {
    // For simplicity, always say we can be unloaded
    S_OK
}

/// Register the preview handler
///
/// # Safety
/// Called by regsvr32 to register the DLL
#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> windows::core::HRESULT {
    match registry::register_preview_handler() {
        Ok(()) => S_OK,
        Err(e) => {
            // Log error to a file for debugging
            let _ = std::fs::write(
                "C:\\eulumdat_preview_register_error.txt",
                format!("Registration failed: {:?}", e),
            );
            E_UNEXPECTED
        }
    }
}

/// Unregister the preview handler
///
/// # Safety
/// Called by regsvr32 /u to unregister the DLL
#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> windows::core::HRESULT {
    match registry::unregister_preview_handler() {
        Ok(()) => S_OK,
        Err(_) => E_UNEXPECTED,
    }
}
