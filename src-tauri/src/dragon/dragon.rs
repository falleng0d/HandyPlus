use rdev::{simulate, EventType, Key, SimulateError};
use std::thread;
use std::time::Duration;

fn send(event_type: &EventType) {
    match simulate(event_type) {
        Ok(()) => (),
        Err(SimulateError) => {
            println!("We could not send {:?}", event_type);
        }
    }

    thread::sleep(Duration::from_millis(20));
}

pub fn disable_dragon_dictation() {
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

pub fn enable_dragon_dictation() {
    send(&EventType::KeyPress(Key::ControlLeft));
    send(&EventType::KeyPress(Key::F7));
    send(&EventType::KeyRelease(Key::F7));
    send(&EventType::KeyRelease(Key::ControlLeft));
}
