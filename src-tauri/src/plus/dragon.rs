//! Pauses Dragon NaturallySpeaking dictation while Handy+ records, so both
//! don't transcribe the same speech, and resumes it afterwards. Windows only.

#[cfg(windows)]
mod platform {
    use crate::input::EnigoState;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use log::warn;
    use std::time::Duration;
    use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};
    use tauri::{AppHandle, Manager};

    const KEY_PAUSE: Duration = Duration::from_millis(20);

    fn dragon_running() -> bool {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Never)),
        );
        system
            .processes()
            .values()
            .any(|process| process.name().eq_ignore_ascii_case("natspeak.exe"))
    }

    fn press(enigo: &mut Enigo, key: Key, direction: Direction) {
        if let Err(e) = enigo.key(key, direction) {
            warn!("Dragon key {key:?} {direction:?} failed: {e}");
        }
        std::thread::sleep(KEY_PAUSE);
    }

    fn tap(enigo: &mut Enigo, key: Key) {
        press(enigo, key, Direction::Press);
        press(enigo, key, Direction::Release);
    }

    fn with_keys(app: &AppHandle, keys: impl FnOnce(&mut Enigo)) {
        if let Some(state) = app.try_state::<EnigoState>() {
            if let Ok(mut enigo) = state.0.lock() {
                keys(&mut enigo);
                return;
            }
        }
        match Enigo::new(&Settings::default()) {
            Ok(mut enigo) => keys(&mut enigo),
            Err(e) => warn!("Could not create Enigo for Dragon keys: {e}"),
        }
    }

    /// Dragon's Ctrl+F7 sequence that turns its microphone off.
    pub fn pause(app: AppHandle) {
        std::thread::spawn(move || {
            if !dragon_running() {
                return;
            }
            with_keys(&app, |enigo| {
                press(enigo, Key::Control, Direction::Press);
                tap(enigo, Key::F7);
                press(enigo, Key::Control, Direction::Release);
                tap(enigo, Key::F7);
                tap(enigo, Key::F7);
                press(enigo, Key::Control, Direction::Press);
                tap(enigo, Key::F7);
                press(enigo, Key::Control, Direction::Release);
            });
        });
    }

    /// Dragon's Ctrl+Alt+F7 shortcut that turns its microphone back on.
    pub fn resume(app: AppHandle) {
        std::thread::spawn(move || {
            if !dragon_running() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
            with_keys(&app, |enigo| {
                press(enigo, Key::Alt, Direction::Press);
                press(enigo, Key::Control, Direction::Press);
                tap(enigo, Key::F7);
                press(enigo, Key::Control, Direction::Release);
                press(enigo, Key::Alt, Direction::Release);
            });
        });
    }
}

#[cfg(not(windows))]
mod platform {
    use tauri::AppHandle;

    pub fn pause(_app: AppHandle) {}

    pub fn resume(_app: AppHandle) {}
}

pub use platform::{pause, resume};
