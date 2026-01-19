#[cfg(target_os = "windows")]
use rdev::{simulate, EventType, Key, SimulateError};
#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use sysinfo::RefreshKind;
#[cfg(target_os = "windows")]
use sysinfo::System;
use sysinfo::{ProcessRefreshKind, UpdateKind};

#[cfg(target_os = "windows")]
fn send(event_type: &EventType) {
    match simulate(event_type) {
        Ok(()) => (),
        Err(SimulateError) => {
            println!("We could not send {:?}", event_type);
        }
    }

    thread::sleep(Duration::from_millis(20));
}

#[cfg(target_os = "windows")]
fn is_natspeak_running() -> bool {
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Never)),
    );
    for process in sys.processes().values() {
        if process.name().to_string_lossy().to_lowercase() == "natspeak.exe" {
            return true;
        }
    }
    false
}

#[cfg(target_os = "windows")]
pub fn disable_dragon_dictation() {
    if !is_natspeak_running() {
        return;
    }

    send(&EventType::KeyPress(Key::ControlLeft));
    send(&EventType::KeyPress(Key::F7));
    send(&EventType::KeyRelease(Key::F7));
    send(&EventType::KeyRelease(Key::ControlLeft));

    send(&EventType::KeyPress(Key::F7));
    send(&EventType::KeyRelease(Key::F7));

    send(&EventType::KeyPress(Key::F7));
    send(&EventType::KeyRelease(Key::F7));

    send(&EventType::KeyPress(Key::ControlLeft));
    send(&EventType::KeyPress(Key::F7));
    send(&EventType::KeyRelease(Key::F7));
    send(&EventType::KeyRelease(Key::ControlLeft));
}

#[cfg(target_os = "windows")]
pub fn enable_dragon_dictation() {
    if !is_natspeak_running() {
        return;
    }

    thread::sleep(Duration::from_millis(50));

    send(&EventType::KeyPress(Key::Alt));
    send(&EventType::KeyPress(Key::ControlLeft));
    send(&EventType::KeyPress(Key::F7));
    send(&EventType::KeyRelease(Key::F7));
    send(&EventType::KeyRelease(Key::ControlLeft));
    send(&EventType::KeyRelease(Key::Alt));
}

#[cfg(not(target_os = "windows"))]
pub fn disable_dragon_dictation() {
    // No-op for non-windows platforms
}

#[cfg(not(target_os = "windows"))]
pub fn enable_dragon_dictation() {
    // No-op for non-windows platforms
}
