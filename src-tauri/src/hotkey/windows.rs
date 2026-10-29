//! Windows: low-level keyboard hook (WH_KEYBOARD_LL).

use std::sync::mpsc;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP,
};

use super::dispatch;
use crate::keys::Key;

pub fn run() -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("hotkey-windows".into())
        .spawn(move || unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(hook_proc),
                GetModuleHandleW(std::ptr::null()),
                0,
            );
            if hook.is_null() {
                let _ = tx.send(Err("SetWindowsHookExW failed".to_string()));
                return;
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
