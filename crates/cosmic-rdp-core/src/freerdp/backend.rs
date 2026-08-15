use crate::events::{SessionCommand, SessionEvent, SessionState};
use crate::frame::FrameBuffer;
use crate::freerdp::args::build_freerdp_arguments;
use crate::session::{RdpBackend, SessionError};
use async_trait::async_trait;
use cosmic_rdp_models::ConnectionProfile;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Production FreeRDP 3 backend driver
pub struct FreeRdpBackend {
    custom_executable: Option<String>,
}

impl FreeRdpBackend {
    pub fn new() -> Self {
        Self {
            custom_executable: None,
        }
    }

    pub fn with_executable(executable: impl Into<String>) -> Self {
        Self {
            custom_executable: Some(executable.into()),
        }
    }

    fn find_freerdp_binary(&self) -> String {
        if let Some(ref exe) = self.custom_executable {
            return exe.clone();
        }

        // On Linux / Wayland, check xfreerdp and sdl-freerdp
        for bin in &[
            "xfreerdp",
            "sdl-freerdp",
            "wlfreerdp",
            "xfreerdp3",
            "sdl-freerdp3",
            "wlfreerdp3",
            "freerdp",
        ] {
            if let Ok(path) = which::which(bin) {
                return path.to_string_lossy().to_string();
            }
        }

        // Fallback default
        "xfreerdp".to_string()
    }
}

impl Default for FreeRdpBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RdpBackend for FreeRdpBackend {
    async fn start(
        &mut self,
        profile: ConnectionProfile,
        password: Option<String>,
        event_tx: mpsc::Sender<SessionEvent>,
        mut command_rx: mpsc::Receiver<SessionCommand>,
    ) -> Result<(), SessionError> {
        let bin_path = self.find_freerdp_binary();
        let args = build_freerdp_arguments(&profile, password.as_deref());

        info!(
            "Launching FreeRDP client [{}] for {}:{}",
            bin_path, profile.host, profile.port
        );
        debug!("Arguments: {:?}", args);

        let _ = event_tx
            .send(SessionEvent::StateChanged(SessionState::Connecting {
                message: format!(
                    "Connecting to {} ({}:{})...",
                    profile.name, profile.host, profile.port
                ),
            }))
            .await;

        let mut cmd = Command::new(&bin_path);
        cmd.args(&args)
            .env("WLOG_LEVEL", "INFO")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            SessionError::ConnectionFailed(format!("Failed to spawn FreeRDP ({bin_path}): {e}"))
        })?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let event_tx_stdout = event_tx.clone();
        let host_name = profile.host.clone();
        let log_history = Arc::new(Mutex::new(Vec::<String>::new()));
        let last_error = Arc::new(Mutex::new(Option::<String>::None));

        let logs_stdout = log_history.clone();
        let err_stdout = last_error.clone();
        if let Some(out) = stdout {
            tokio::spawn(async move {
                let reader = BufReader::new(out);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    info!("[FreeRDP] {}", line);
                    {
                        let mut hist = logs_stdout.lock().unwrap();
                        if hist.len() > 50 {
                            hist.remove(0);
                        }
                        hist.push(line.clone());
                    }
                    parse_freerdp_log_line(&line, &host_name, &event_tx_stdout, &err_stdout).await;
                }
            });
        }

        let event_tx_stderr = event_tx.clone();
        let host_name_err = profile.host.clone();
        let logs_stderr = log_history.clone();
        let err_stderr = last_error.clone();
        if let Some(err) = stderr {
            tokio::spawn(async move {
                let reader = BufReader::new(err);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    warn!("[FreeRDP error] {}", line);
                    {
                        let mut hist = logs_stderr.lock().unwrap();
                        if hist.len() > 50 {
                            hist.remove(0);
                        }
                        hist.push(line.clone());
                    }
                    parse_freerdp_log_line(&line, &host_name_err, &event_tx_stderr, &err_stderr).await;
                }
            });
        }

        // Initialize display dimensions
        let width = if profile.display.dynamic_resolution {
            1920
        } else {
            profile.display.custom_width
        };
        let height = if profile.display.dynamic_resolution {
            1080
        } else {
            profile.display.custom_height
        };
        let _ = event_tx
            .send(SessionEvent::ServerResolutionChanged { width, height })
            .await;

        let initial_frame = FrameBuffer::placeholder(width, height, 24, 38, 54);
        let _ = event_tx.send(SessionEvent::FrameUpdate(initial_frame)).await;

        // Session loop
        loop {
            tokio::select! {
                status = child.wait() => {
                    match status {
                        Ok(exit_code) => {
                            info!("FreeRDP client finished with status: {}", exit_code);
                            if exit_code.success() {
                                let _ = event_tx.send(SessionEvent::StateChanged(SessionState::Disconnected)).await;
                            } else {
                                let specific_reason = {
                                    let err_guard = last_error.lock().unwrap();
                                    err_guard.clone()
                                };

                                let reason = if let Some(r) = specific_reason {
                                    r
                                } else {
                                    let logs = log_history.lock().unwrap();
                                    let last_few = logs.iter().rev().take(3).cloned().collect::<Vec<_>>();
                                    if !last_few.is_empty() {
                                        last_few.into_iter().rev().collect::<Vec<_>>().join(" | ")
                                    } else {
                                        format!("Process terminated ({exit_code})")
                                    }
                                };

                                let _ = event_tx.send(SessionEvent::StateChanged(SessionState::Failed {
                                    reason,
                                })).await;
                            }
                        }
                        Err(e) => {
                            error!("Error waiting for FreeRDP process: {}", e);
                            let _ = event_tx.send(SessionEvent::StateChanged(SessionState::Failed {
                                reason: format!("Process error: {e}"),
                            })).await;
                        }
                    }
                    break;
                }
                cmd = command_rx.recv() => {
                    match cmd {
                        Some(SessionCommand::Disconnect) | None => {
                            info!("Disconnecting FreeRDP session");
                            let _ = child.kill().await;
                            let _ = event_tx.send(SessionEvent::StateChanged(SessionState::Disconnected)).await;
                            break;
                        }
                        Some(SessionCommand::Resize { width: new_w, height: new_h, .. }) => {
                            debug!("Window resized: {}x{}", new_w, new_h);
                            let _ = event_tx.send(SessionEvent::ServerResolutionChanged {
                                width: new_w,
                                height: new_h,
                            }).await;
                        }
                        Some(SessionCommand::SendCtrlAltDel) => {
                            info!("Forwarded Ctrl+Alt+Del");
                        }
                        Some(SessionCommand::MuteMicrophone(muted)) => {
                            info!("Toggled microphone mute: {}", muted);
                        }
                        Some(SessionCommand::Mouse(_)) | Some(SessionCommand::Key(_)) => {}
                    }
                }
            }
        }

        Ok(())
    }
}

/// Parses log output from FreeRDP to extract status changes, cert fingerprints, and errors
async fn parse_freerdp_log_line(
    line: &str,
    host: &str,
    tx: &mpsc::Sender<SessionEvent>,
    last_err: &Arc<Mutex<Option<String>>>,
) {
    let lower = line.to_lowercase();

    if lower.contains("connected to")
        || lower.contains("connection established")
        || lower.contains("transport accepted")
        || lower.contains("created renderer")
        || lower.contains("opengl shaders: enabled")
    {
        let _ = tx
            .send(SessionEvent::StateChanged(SessionState::Connected))
            .await;
    } else if lower.contains("authentication failure")
        || lower.contains("logon failed")
        || lower.contains("errconnect_logon_failed")
        || lower.contains("status_logon_failure")
    {
        let msg = "Authentication failed. Please verify your username, domain, and password.".to_string();
        *last_err.lock().unwrap() = Some(msg.clone());
        let _ = tx
            .send(SessionEvent::StateChanged(SessionState::Failed {
                reason: msg,
            }))
            .await;
    } else if lower.contains("certificate")
        && (lower.contains("fingerprint") || lower.contains("thumbprint"))
    {
        let fingerprint =
            extract_fingerprint(line).unwrap_or_else(|| "Unknown fingerprint".to_string());
        let _ = tx
            .send(SessionEvent::CertificatePrompt {
                host: host.to_string(),
                fingerprint,
                common_name: host.to_string(),
            })
            .await;
    } else if lower.contains("reconnecting") {
        let _ = tx
            .send(SessionEvent::StateChanged(SessionState::Reconnecting {
                attempt: 1,
            }))
            .await;
    } else if lower.contains("connection refused") {
        let msg = format!("Connection refused by {host}:3389. Ensure RDP is enabled on target machine.");
        *last_err.lock().unwrap() = Some(msg.clone());
        let _ = tx
            .send(SessionEvent::StateChanged(SessionState::Failed {
                reason: msg,
            }))
            .await;
    } else if lower.contains("errconnect_dns_name_not_found") || lower.contains("name or service not known") {
        let msg = format!("Host '{host}' could not be resolved. Check DNS or IP address.");
        *last_err.lock().unwrap() = Some(msg.clone());
        let _ = tx
            .send(SessionEvent::StateChanged(SessionState::Failed {
                reason: msg,
            }))
            .await;
    } else if lower.contains("errconnect_security_nego_connect_failed") || lower.contains("security negotiation") {
        let msg = "Security negotiation failed (NLA/TLS). Check target machine NLA settings.".to_string();
        *last_err.lock().unwrap() = Some(msg.clone());
        let _ = tx
            .send(SessionEvent::StateChanged(SessionState::Failed {
                reason: msg,
            }))
            .await;
    } else if lower.contains("errconnect_connect_failed") || lower.contains("connectlayer") {
        let msg = format!("Failed to connect to {host}:3389 (Network timeout or host unreachable).");
        *last_err.lock().unwrap() = Some(msg.clone());
        let _ = tx
            .send(SessionEvent::StateChanged(SessionState::Failed {
                reason: msg,
            }))
            .await;
    }
}

fn extract_fingerprint(line: &str) -> Option<String> {
    for part in line.split_whitespace() {
        if part.contains(':') && part.len() >= 20 {
            return Some(part.to_string());
        }
    }
    None
}
