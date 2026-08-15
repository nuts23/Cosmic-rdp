use crate::events::{SessionCommand, SessionEvent, SessionState};
use crate::frame::FrameBuffer;
use crate::session::{RdpBackend, SessionError};
use async_trait::async_trait;
use cosmic_rdp_models::ConnectionProfile;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::info;

/// A mock RDP backend simulator for UI testing and prototyping
pub struct MockRdpBackend;

impl MockRdpBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockRdpBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RdpBackend for MockRdpBackend {
    async fn start(
        &mut self,
        profile: ConnectionProfile,
        _password: Option<String>,
        event_tx: mpsc::Sender<SessionEvent>,
        mut command_rx: mpsc::Receiver<SessionCommand>,
    ) -> Result<(), SessionError> {
        info!("Starting mock RDP session for profile: {}", profile.name);

        let _ = event_tx
            .send(SessionEvent::StateChanged(SessionState::Connecting {
                message: format!("Connecting to {}...", profile.host),
            }))
            .await;

        sleep(Duration::from_millis(600)).await;

        let _ = event_tx
            .send(SessionEvent::StateChanged(SessionState::Connected))
            .await;

        let mut width = if profile.display.dynamic_resolution { 1280 } else { profile.display.custom_width };
        let mut height = if profile.display.dynamic_resolution { 720 } else { profile.display.custom_height };

        let _ = event_tx
            .send(SessionEvent::ServerResolutionChanged { width, height })
            .await;

        // Render an initial Windows-style mock desktop
        let mut seq = 1u64;
        let initial_frame = generate_mock_desktop_frame(width, height, seq, &profile.name, &profile.username);
        let _ = event_tx.send(SessionEvent::FrameUpdate(initial_frame)).await;

        let mut tick_interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    seq += 1;
                    // Only update occasionally to simulate activity
                    if seq % 10 == 0 {
                        let frame = generate_mock_desktop_frame(width, height, seq, &profile.name, &profile.username);
                        if event_tx.send(SessionEvent::FrameUpdate(frame)).await.is_err() {
                            break;
                        }
                    }
                }
                cmd = command_rx.recv() => {
                    match cmd {
                        Some(SessionCommand::Disconnect) | None => {
                            info!("Session disconnected via command or channel close");
                            let _ = event_tx.send(SessionEvent::StateChanged(SessionState::Disconnected)).await;
                            break;
                        }
                        Some(SessionCommand::Resize { width: new_w, height: new_h, .. }) => {
                            info!("Received resize request: {}x{}", new_w, new_h);
                            width = new_w.max(320);
                            height = new_h.max(240);
                            seq += 1;
                            let frame = generate_mock_desktop_frame(width, height, seq, &profile.name, &profile.username);
                            let _ = event_tx.send(SessionEvent::FrameUpdate(frame)).await;
                        }
                        Some(SessionCommand::Mouse(_input)) => {
                            // Forward mouse in real implementation
                        }
                        Some(SessionCommand::Key(_input)) => {
                            // Forward key in real implementation
                        }
                        Some(SessionCommand::SendCtrlAltDel) => {
                            info!("Sent Ctrl+Alt+Del");
                        }
                        Some(SessionCommand::MuteMicrophone(muted)) => {
                            info!("Set microphone muted: {}", muted);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn generate_mock_desktop_frame(width: u32, height: u32, seq: u64, _name: &str, _user: &str) -> FrameBuffer {
    let total_bytes = (width * height * 4) as usize;
    let mut data = vec![0u8; total_bytes];

    // Background gradient: Windows 11-style Bloom deep blue
    for y in 0..height {
        let y_ratio = y as f32 / height as f32;
        for x in 0..width {
            let x_ratio = x as f32 / width as f32;
            let offset = ((y * width + x) * 4) as usize;
            if offset + 3 < data.len() {
                let r = (20.0 + 30.0 * x_ratio) as u8;
                let g = (40.0 + 60.0 * (1.0 - y_ratio)) as u8;
                let b = (90.0 + 120.0 * y_ratio) as u8;
                data[offset] = r;
                data[offset + 1] = g;
                data[offset + 2] = b;
                data[offset + 3] = 255;
            }
        }
    }

    // Draw a Windows-style Taskbar at the bottom (48px high)
    let taskbar_height = 48u32;
    if height > taskbar_height {
        let taskbar_start = height - taskbar_height;
        for y in taskbar_start..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                if offset + 3 < data.len() {
                    data[offset] = 16;
                    data[offset + 1] = 20;
                    data[offset + 2] = 28;
                    data[offset + 3] = 240;
                }
            }
        }
    }

    FrameBuffer::new(width, height, data, seq)
}
