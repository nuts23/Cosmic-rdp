use crate::events::{SessionCommand, SessionEvent};
use cosmic_rdp_models::ConnectionProfile;
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Failed to connect to target: {0}")]
    ConnectionFailed(String),
    #[error("Authentication rejected: {0}")]
    AuthenticationFailed(String),
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Engine error: {0}")]
    Engine(String),
}

/// Handle held by the UI to interact with an active RDP session
#[derive(Clone)]
pub struct RdpSessionHandle {
    command_tx: mpsc::Sender<SessionCommand>,
}

impl RdpSessionHandle {
    pub fn new(command_tx: mpsc::Sender<SessionCommand>) -> Self {
        Self { command_tx }
    }

    /// Send a command to the session backend
    pub async fn send_command(&self, cmd: SessionCommand) -> Result<(), SessionError> {
        self.command_tx
            .send(cmd)
            .await
            .map_err(|_| SessionError::ChannelClosed)
    }

    /// Disconnect active session
    pub async fn disconnect(&self) -> Result<(), SessionError> {
        self.send_command(SessionCommand::Disconnect).await
    }

    /// Request server-side dynamic display resize
    pub async fn request_resize(&self, width: u32, height: u32, scale_factor: u32) -> Result<(), SessionError> {
        self.send_command(SessionCommand::Resize { width, height, scale_factor }).await
    }
}

/// Trait implemented by RDP backends (e.g. FreeRDP 3 backend or mock simulator)
#[async_trait::async_trait]
pub trait RdpBackend: Send + Sync {
    async fn start(
        &mut self,
        profile: ConnectionProfile,
        password: Option<String>,
        event_tx: mpsc::Sender<SessionEvent>,
        command_rx: mpsc::Receiver<SessionCommand>,
    ) -> Result<(), SessionError>;
}
