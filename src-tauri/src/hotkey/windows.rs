//! Windows: low-level keyboard hook (WH_KEYBOARD_LL).
//!
//! The hook does not see key events on the secure desktop (lock screen,
//! Ctrl+Alt+Del), so key-ups of e.g. Win+L are lost. A message-only window
//! listens for session lock/unlock and resets the matcher's key state.

use std::sync::mpsc;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    SetWindowsHookExW, TranslateMessage, HWND_MESSAGE, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_WTSSESSION_CHANGE,
    WNDCLASSW, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};

use super::{dispatch, reset};
use crate::keys::Key;

pub fn run() -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("hotkey-windows".into())
        .spawn(move || unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
            if hook.is_null() {
                let _ = tx.send(Err("SetWindowsHookExW failed".to_string()));
                return;
            }
            if let Err(e) = watch_session(module) {
                // Not fatal: the shortcut still works, it may just need an
                // extra key press after unlocking.
                tracing::warn!("{e}");
            }
            let _ = tx.send(Ok(()));
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

/// Create a message-only window on the current thread that receives
/// session lock/unlock notifications.
unsafe fn watch_session(module: *mut core::ffi::c_void) -> Result<(), String> {
    let class: Vec<u16> = "JabberwriteSession\0".encode_utf16().collect();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(session_proc),
        hInstance: module,
        lpszClassName: class.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassW(&wc) == 0 {
        return Err("RegisterClassW failed".into());
    }
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        std::ptr::null(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        std::ptr::null_mut(),
        module,
        std::ptr::null(),
    );
    if hwnd.is_null() {
        return Err("CreateWindowExW failed".into());
    }
    if WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) == 0 {
        return Err("WTSRegisterSessionNotification failed".into());
    }
    Ok(())
}

unsafe extern "system" fn session_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_WTSSESSION_CHANGE && matches!(wparam as u32, WTS_SESSION_LOCK | WTS_SESSION_UNLOCK)
    {
        reset();
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam as *const KBDLLHOOKSTRUCT);
        // Ignore our own synthesized paste keystrokes.
        if info.flags & LLKHF_INJECTED == 0 {
            match wparam as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => dispatch(map(info.vkCode), true),
                WM_KEYUP | WM_SYSKEYUP => dispatch(map(info.vkCode), false),
                _ => {}
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

fn map(vk: u32) -> Key {
    match vk {
        0x11 | 0xA2 | 0xA3 => Key::Ctrl,
        0x10 | 0xA0 | 0xA1 => Key::Shift,
        0x12 | 0xA4 | 0xA5 => Key::Alt,
        0x5B | 0x5C => Key::Super,
        0x20 => Key::Space,
        0x0D => Key::Enter,
        0x09 => Key::Tab,
        0x1B => Key::Escape,
        0x08 => Key::Backspace,
        0x30..=0x39 | 0x41..=0x5A => Key::Char(char::from_u32(vk).unwrap()),
        0x70..=0x87 => Key::F((vk - 0x70 + 1) as u8),
        _ => Key::Other(vk),
    }
}
