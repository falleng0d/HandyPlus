//! Live check: does a blocked modifier-only hotkey leak any of its events?
//!
//! An observer low-level hook is installed first, then handy-keys' blocking
//! hook (Windows calls the newest hook first). Injected events the observer
//! sees are events the foreground app would also receive.
//!
//!   cargo run --example modifier_leak_check -- [ctrl_right|alt_right]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use std::thread::sleep;
#[cfg(windows)]
use std::time::Duration;
#[cfg(windows)]
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_RCONTROL, VK_RMENU, VK_RSHIFT,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
};

#[cfg(windows)]
const MARK: usize = 0x4C45_414B;
#[cfg(windows)]
static SEEN: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

#[cfg(windows)]
unsafe extern "system" fn observer(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if kb.dwExtraInfo == MARK {
            SEEN.lock().unwrap().push((wparam.0 as u32, kb.vkCode));
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
fn inject(vk: VIRTUAL_KEY, up: bool) {
    let mut flags = KEYEVENTF_EXTENDEDKEY;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags | KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: MARK,
            },
        },
    };
    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    assert_eq!(sent, 1, "SendInput failed");
    sleep(Duration::from_millis(40));
}

#[cfg(windows)]
fn take_seen() -> Vec<(u32, u32)> {
    sleep(Duration::from_millis(200));
    std::mem::take(&mut *SEEN.lock().unwrap())
}

#[cfg(windows)]
fn main() -> handy_keys::Result<()> {
    use handy_keys::{Hotkey, HotkeyManager};

    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ctrl_right".into());
    let vk = match target.as_str() {
        "alt_right" => VK_RMENU,
        _ => VK_RCONTROL,
    };

    std::thread::spawn(|| unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(observer), None, 0).expect("observer hook");
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    });
    sleep(Duration::from_millis(300));

    let manager = HotkeyManager::new_with_blocking()?;
    manager.register(target.parse::<Hotkey>()?)?;
    sleep(Duration::from_millis(300));

    // Unbound key: must pass through untouched.
    inject(VK_RSHIFT, false);
    inject(VK_RSHIFT, true);
    let control = take_seen();

    // Bound key held: first press, five auto-repeats, release.
    for _ in 0..6 {
        inject(vk, false);
    }
    inject(vk, true);
    let leaked = take_seen();

    let mut states = Vec::new();
    while let Some(event) = manager.try_recv() {
        states.push(format!("{:?}", event.state));
    }

    println!("control (unbound RShift) seen by app: {control:x?}");
    println!("bound {target} seen by app: {leaked:x?}");
    println!("hotkey events: {states:?}");

    let pass = control.len() == 2
        && leaked.is_empty()
        && states.first().map(String::as_str) == Some("Pressed")
        && states.last().map(String::as_str) == Some("Released");
    println!("{}", if pass { "PASS" } else { "FAIL" });
    std::process::exit(if pass { 0 } else { 1 });
}
