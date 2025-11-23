use log::error;
use rustfft::num_traits::ToPrimitive;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;

enum VolumeCommand {
    Get,
    Set(u8),
}

enum VolumeResponse {
    GetResult(u8),
    SetResult,
}

/// A self-contained volume controller that manages system volume operations
/// through a dedicated thread.
#[derive(Clone)]
pub struct VolumeController {
    tx: Sender<(VolumeCommand, Sender<VolumeResponse>)>,
}

impl VolumeController {
    /// Creates a new volume controller with a dedicated thread for volume operations.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<(VolumeCommand, Sender<VolumeResponse>)>();

        thread::spawn(move || {
            while let Ok((cmd, response_tx)) = rx.recv() {
                match cmd {
                    VolumeCommand::Get => {
                        let volume = cpvc::get_system_volume();
                        let _ = response_tx.send(VolumeResponse::GetResult(volume));
                    }
                    VolumeCommand::Set(volume) => {
                        cpvc::set_system_volume(volume);
                        let _ = response_tx.send(VolumeResponse::SetResult);
                    }
                }
            }
        });

        Self { tx }
    }

    /// Gets the current system volume.
    pub fn get_system_volume(&self) -> u8 {
        let (response_tx, response_rx) = mpsc::channel();

        if self.tx.send((VolumeCommand::Get, response_tx)).is_err() {
            error!("Failed to send volume get command");
            return 0.5.to_u8().unwrap();
        }

        match response_rx.recv() {
            Ok(VolumeResponse::GetResult(vol)) => vol,
            _ => {
                error!("Failed to get system volume");
                0.5.to_u8().unwrap()
            }
        }
    }

    /// Sets the system volume to the specified level.
    pub fn set_system_volume(&self, volume: u8) {
        let (response_tx, response_rx) = mpsc::channel();

        if self
            .tx
            .send((VolumeCommand::Set(volume), response_tx))
            .is_err()
        {
            error!("Failed to send volume set command");
            return;
        }

        if response_rx.recv().is_err() {
            error!("Failed to set system volume to {}", volume);
        }
    }
}
