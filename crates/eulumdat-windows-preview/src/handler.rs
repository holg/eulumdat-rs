//! Preview Handler implementation
//!
//! Implements IPreviewHandler, IInitializeWithStream, IObjectWithSite, and IOleWindow interfaces

#![cfg(windows)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

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

use windows::core::{
    implement, Error as WinError, IUnknown, Interface, Result as WinResult, PCWSTR,
};
use windows::Win32::Foundation::{COLORREF, E_FAIL, HWND, RECT, S_FALSE};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect,
    SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, PAINTSTRUCT,
};
use windows::Win32::System::Com::IStream;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{
    IObjectWithSite, IObjectWithSite_Impl, IOleWindow, IOleWindow_Impl,
};
use windows::Win32::UI::Shell::PropertiesSystem::{
    IInitializeWithStream, IInitializeWithStream_Impl,
};
use windows::Win32::UI::Shell::{IPreviewHandler, IPreviewHandler_Impl};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW,
    RegisterClassW, SetWindowLongPtrW, ShowWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, SW_SHOW,
    WINDOW_EX_STYLE, WM_DESTROY, WM_PAINT, WNDCLASSW, WS_CHILD, WS_VISIBLE,
};

use crate::render::render_ldt_to_bgra;

// Unique class name counter to avoid conflicts
static CLASS_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Preview handler state
struct PreviewState {
    /// The SVG content
    svg_content: Option<String>,
    /// Rendered BGRA pixels
    bgra_pixels: Option<Vec<u8>>,
    /// Image dimensions
    image_width: u32,
    image_height: u32,
    /// Parent window
    parent_hwnd: Option<HWND>,
    /// Our preview window
    preview_hwnd: Option<HWND>,
    /// Preview rectangle
    rect: RECT,
    /// Site (for IObjectWithSite)
    site: Option<IUnknown>,
    /// Window class name
    class_name: Vec<u16>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            svg_content: None,
            bgra_pixels: None,
            image_width: 0,
            image_height: 0,
            parent_hwnd: None,
            preview_hwnd: None,
            rect: RECT::default(),
            site: None,
            class_name: Vec::new(),
        }
    }
}

/// Box to store pixel data for WM_PAINT
struct PixelData {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// The Eulumdat Preview Handler COM object
#[implement(IPreviewHandler, IInitializeWithStream, IOleWindow, IObjectWithSite)]
pub struct EulumdatPreviewHandler {
    state: RefCell<PreviewState>,
}

impl EulumdatPreviewHandler {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(PreviewState::default()),
        }
    }

    /// Read file content from IStream
    fn read_stream_content(stream: &IStream) -> Result<String, WinError> {
        let mut content = Vec::new();
        let mut buffer = [0u8; 4096];

        loop {
            let mut bytes_read = 0u32;
            unsafe {
                let hr = stream.Read(
                    buffer.as_mut_ptr() as *mut c_void,
                    buffer.len() as u32,
                    Some(&mut bytes_read),
                );
                if hr.is_err() {
                    break; // End of stream or error
                }
            }

            if bytes_read == 0 {
                break;
            }

            content.extend_from_slice(&buffer[..bytes_read as usize]);
        }

        if content.is_empty() {
            return Err(WinError::from(E_FAIL));
        }

        // Try to decode as Windows-1252 (common for LDT files)
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&content);
        Ok(decoded.into_owned())
    }

    /// Generate SVG from file content
    fn generate_svg(content: &str, width: f64, height: f64) -> Result<String, String> {
        // Try LDT first, then IES
        let ldt = eulumdat::Eulumdat::parse(content)
            .or_else(|_| eulumdat::IesParser::parse(content))
            .map_err(|e| e.to_string())?;

        let polar = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
        let theme = eulumdat::diagram::SvgTheme::light();
        Ok(polar.to_svg(width, height, &theme))
    }
}

impl IInitializeWithStream_Impl for EulumdatPreviewHandler_Impl {
    fn Initialize(&self, pstream: Option<&IStream>, _grfmode: u32) -> WinResult<()> {
        debug_log("Initialize called");

        let stream = pstream.ok_or_else(|| {
            debug_log("Initialize: No stream provided");
            WinError::from(E_FAIL)
        })?;

        // Read stream content
        debug_log("Initialize: Reading stream content...");
        let content = match EulumdatPreviewHandler::read_stream_content(stream) {
            Ok(c) => {
                debug_log(&format!("Initialize: Read {} bytes", c.len()));
                c
            }
            Err(e) => {
                debug_log(&format!("Initialize: Failed to read stream: {:?}", e));
                return Err(e);
            }
        };

        // Store content for later rendering
        let mut state = self.state.borrow_mut();

        // Generate SVG at a reasonable preview size
        debug_log("Initialize: Generating SVG...");
        match EulumdatPreviewHandler::generate_svg(&content, 500.0, 500.0) {
            Ok(svg) => {
                debug_log(&format!("Initialize: SVG generated, {} chars", svg.len()));
                state.svg_content = Some(svg);
                Ok(())
            }
            Err(e) => {
                debug_log(&format!("Initialize: Failed to generate SVG: {}", e));
                Err(WinError::from(E_FAIL))
            }
        }
    }
}

impl IObjectWithSite_Impl for EulumdatPreviewHandler_Impl {
    fn SetSite(&self, punksite: Option<&IUnknown>) -> WinResult<()> {
        let mut state = self.state.borrow_mut();
        state.site = punksite.map(|s| s.clone());
        Ok(())
    }

    fn GetSite(
        &self,
        riid: *const windows::core::GUID,
        ppvsite: *mut *mut c_void,
    ) -> WinResult<()> {
        let state = self.state.borrow();
        if let Some(ref site) = state.site {
            unsafe { site.query(riid, ppvsite).ok() }
        } else {
            Err(WinError::from(E_FAIL))
        }
    }
}

impl IPreviewHandler_Impl for EulumdatPreviewHandler_Impl {
    fn SetWindow(&self, hwnd: HWND, prc: *const RECT) -> WinResult<()> {
        debug_log(&format!("SetWindow called: hwnd={:?}", hwnd));
        let mut state = self.state.borrow_mut();
        state.parent_hwnd = Some(hwnd);
        if !prc.is_null() {
            unsafe {
                state.rect = *prc;
                debug_log(&format!(
                    "SetWindow: rect=({},{},{},{})",
                    state.rect.left, state.rect.top, state.rect.right, state.rect.bottom
                ));
            }
        }
        Ok(())
    }

    fn SetRect(&self, prc: *const RECT) -> WinResult<()> {
        if prc.is_null() {
            return Err(WinError::from(E_FAIL));
        }

        let mut state = self.state.borrow_mut();
        unsafe {
            state.rect = *prc;
        }

        // Resize preview window if it exists
        if let Some(hwnd) = state.preview_hwnd {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    hwnd,
                    HWND::default(),
                    state.rect.left,
                    state.rect.top,
                    state.rect.right - state.rect.left,
                    state.rect.bottom - state.rect.top,
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                        | windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
                );
                let _ = InvalidateRect(hwnd, None, true);
            }
        }

        Ok(())
    }

    fn DoPreview(&self) -> WinResult<()> {
        debug_log("DoPreview called");
        let mut state = self.state.borrow_mut();

        let svg = state.svg_content.clone().ok_or_else(|| {
            debug_log("DoPreview: No SVG content");
            WinError::from(E_FAIL)
        })?;
        let parent = state.parent_hwnd.ok_or_else(|| {
            debug_log("DoPreview: No parent window");
            WinError::from(E_FAIL)
        })?;

        // Calculate preview size
        let width = (state.rect.right - state.rect.left).max(50) as u32;
        let height = (state.rect.bottom - state.rect.top).max(50) as u32;
        debug_log(&format!("DoPreview: size={}x{}", width, height));

        // Render SVG to BGRA pixels
        debug_log("DoPreview: Rendering SVG to BGRA...");
        let (pixels, img_w, img_h) = render_ldt_to_bgra(&svg, width, height).map_err(|e| {
            debug_log(&format!("DoPreview: Render failed: {}", e));
            WinError::from(E_FAIL)
        })?;
        debug_log(&format!(
            "DoPreview: Rendered {}x{}, {} bytes",
            img_w,
            img_h,
            pixels.len()
        ));

        state.bgra_pixels = Some(pixels.clone());
        state.image_width = img_w;
        state.image_height = img_h;

        // Create preview window
        unsafe {
            // Generate unique class name
            let counter = CLASS_COUNTER.fetch_add(1, Ordering::SeqCst);
            let class_name_str = format!("EulumdatPreview_{}\0", counter);
            let class_name: Vec<u16> = class_name_str.encode_utf16().collect();
            state.class_name = class_name.clone();

            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(preview_window_proc),
                hInstance: GetModuleHandleW(None).unwrap_or_default().into(),
                lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
                hbrBackground: CreateSolidBrush(COLORREF(0x00FFFFFF)), // White
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            // Allocate pixel data on heap and store pointer
            let pixel_data = Box::new(PixelData {
                pixels,
                width: img_w,
                height: img_h,
            });
            let pixel_data_ptr = Box::into_raw(pixel_data);

            // Create child window
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR::from_raw(class_name.as_ptr()),
                PCWSTR::null(),
                WS_CHILD | WS_VISIBLE,
                state.rect.left,
                state.rect.top,
                state.rect.right - state.rect.left,
                state.rect.bottom - state.rect.top,
                parent,
                None,
                None,
                Some(pixel_data_ptr as *const c_void),
            )?;

            // Store pixel data pointer in window
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, pixel_data_ptr as isize);

            state.preview_hwnd = Some(hwnd);
            debug_log(&format!("DoPreview: Created window {:?}", hwnd));
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = InvalidateRect(hwnd, None, true);
        }

        debug_log("DoPreview: Complete");
        Ok(())
    }

    fn Unload(&self) -> WinResult<()> {
        debug_log("Unload called");
        let mut state = self.state.borrow_mut();

        if let Some(hwnd) = state.preview_hwnd.take() {
            unsafe {
                // Get and free pixel data
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PixelData;
                if !ptr.is_null() {
                    let _ = Box::from_raw(ptr);
                }
                let _ = DestroyWindow(hwnd);
            }
        }

        state.svg_content = None;
        state.bgra_pixels = None;

        Ok(())
    }

    fn SetFocus(&self) -> WinResult<()> {
        let state = self.state.borrow();
        if let Some(hwnd) = state.preview_hwnd {
            unsafe {
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(hwnd);
            }
        }
        Ok(())
    }

    fn QueryFocus(&self) -> WinResult<HWND> {
        unsafe { Ok(windows::Win32::UI::Input::KeyboardAndMouse::GetFocus()) }
    }

    fn TranslateAccelerator(
        &self,
        _pmsg: *const windows::Win32::UI::WindowsAndMessaging::MSG,
    ) -> WinResult<()> {
        Err(WinError::from(S_FALSE))
    }
}

impl IOleWindow_Impl for EulumdatPreviewHandler_Impl {
    fn GetWindow(&self) -> WinResult<HWND> {
        let state = self.state.borrow();
        Ok(state
            .preview_hwnd
            .unwrap_or(state.parent_hwnd.unwrap_or_default()))
    }

    fn ContextSensitiveHelp(&self, _fentermode: windows::Win32::Foundation::BOOL) -> WinResult<()> {
        Ok(())
    }
}

/// Window procedure for the preview window
unsafe extern "system" fn preview_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            // Get pixel data from window
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const PixelData;

            if !ptr.is_null() {
                let data = &*ptr;

                if !data.pixels.is_empty() && data.width > 0 && data.height > 0 {
                    // Get client rect
                    let mut client_rect = RECT::default();
                    let _ = GetClientRect(hwnd, &mut client_rect);
                    let client_w = client_rect.right - client_rect.left;
                    let client_h = client_rect.bottom - client_rect.top;

                    // Fill background with white
                    let brush = CreateSolidBrush(COLORREF(0x00FFFFFF));
                    let _ = FillRect(hdc, &client_rect, brush);
                    let _ = DeleteObject(brush);

                    // Center the image
                    let x = (client_w - data.width as i32) / 2;
                    let y = (client_h - data.height as i32) / 2;

                    // Create bitmap info
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: data.width as i32,
                            biHeight: -(data.height as i32), // Top-down
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    // Draw the bitmap
                    SetDIBitsToDevice(
                        hdc,
                        x.max(0),
                        y.max(0),
                        data.width,
                        data.height,
                        0,
                        0,
                        0,
                        data.height,
                        data.pixels.as_ptr() as *const c_void,
                        &bmi,
                        DIB_RGB_COLORS,
                    );
                }
            } else {
                // No data - fill with light gray
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let brush = CreateSolidBrush(COLORREF(0x00F0F0F0));
                let _ = FillRect(hdc, &rect, brush);
                let _ = DeleteObject(brush);
            }

            let _ = EndPaint(hwnd, &ps);
            windows::Win32::Foundation::LRESULT(0)
        }
        WM_DESTROY => {
            // Clean up pixel data
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PixelData;
            if !ptr.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(ptr);
            }
            windows::Win32::Foundation::LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
