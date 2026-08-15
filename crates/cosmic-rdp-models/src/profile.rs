use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a connection profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(pub Uuid);

impl ProfileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProfileId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sound output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AudioOutputMode {
    #[default]
    PlayLocally,
    PlayOnRemote,
    DoNotPlay,
}

/// Audio redirection settings (optimized for Teams usage)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioSettings {
    /// Enable local microphone redirection (`audin`) to remote Windows
    pub microphone_enabled: bool,
    /// Remote sound output redirection (`rdpsnd`)
    pub output_mode: AudioOutputMode,
    /// Low-latency buffer tuning for Teams voice calls
    pub low_latency: bool,
    /// Preferred input device (microphone) name
    pub preferred_mic_device: Option<String>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            microphone_enabled: true,
            output_mode: AudioOutputMode::PlayLocally,
            low_latency: true,
            preferred_mic_device: None,
        }
    }
}

/// Camera passthrough settings (`rdpecam` / MS-RDPECAM)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CameraSettings {
    /// Enable camera passthrough
    pub enabled: bool,
    /// Preferred camera device path or name (optional)
    pub device_name: Option<String>,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            device_name: None,
        }
    }
}

/// Scaling mode behavior for active session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScalingMode {
    /// Dynamic display update channel (auto-resize remote desktop resolution to fit window)
    #[default]
    DynamicResize,
    /// Scale remote framebuffer to fit the current window size
    FitWindow,
    /// Display remote framebuffer in 1:1 original pixel size
    OriginalSize,
}

/// Display and resolution settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplaySettings {
    /// Dynamic display update channel (auto-resize remote desktop to window size)
    pub dynamic_resolution: bool,
    /// Scaling mode
    pub scaling_mode: ScalingMode,
    /// Launch in fullscreen mode
    pub fullscreen: bool,
    /// Fixed custom resolution width (if dynamic resolution is disabled)
    pub custom_width: u32,
    /// Fixed custom resolution height (if dynamic resolution is disabled)
    pub custom_height: u32,
    /// High DPI / display scale factor percentage (e.g. 100, 125, 150, 200)
    pub scale_factor: u32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            dynamic_resolution: true,
            scaling_mode: ScalingMode::DynamicResize,
            fullscreen: false,
            custom_width: 1920,
            custom_height: 1080,
            scale_factor: 100,
        }
    }
}

/// Connection security and certificate validation policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CertificatePolicy {
    /// Prompt on first connect and verify thumbprint
    #[default]
    TrustOnFirstUse,
    /// Strict validation with CA bundle
    Strict,
    /// Ignore certificate errors (insecure / dev mode)
    Ignore,
}

/// RDP Target Connection Profile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionProfile {
    /// Unique profile ID
    pub id: ProfileId,
    /// User-friendly label (e.g. "Work Desktop", "Windows Dev Box")
    pub name: String,
    /// Target hostname or IP address
    pub host: String,
    /// Target RDP port (default 3389)
    pub port: u16,
    /// RDP Username
    pub username: String,
    /// Windows Domain (optional)
    pub domain: Option<String>,
    /// Stored certificate thumbprint for TOFU validation
    pub cert_fingerprint: Option<String>,
    /// Certificate validation policy
    pub cert_policy: CertificatePolicy,
    /// Audio redirection configuration
    pub audio: AudioSettings,
    /// Camera redirection configuration
    pub camera: CameraSettings,
    /// Display resolution configuration
    pub display: DisplaySettings,
    /// Optional color accent or tag for the connection card
    pub color_tag: Option<String>,
    /// Last connected timestamp
    pub last_connected: Option<DateTime<Utc>>,
    /// Freeform user notes
    pub notes: String,
}

impl ConnectionProfile {
    pub fn new(name: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            id: ProfileId::new(),
            name: name.into(),
            host: host.into(),
            port: 3389,
            username: String::new(),
            domain: None,
            cert_fingerprint: None,
            cert_policy: CertificatePolicy::TrustOnFirstUse,
            audio: AudioSettings::default(),
            camera: CameraSettings::default(),
            display: DisplaySettings::default(),
            color_tag: None,
            last_connected: None,
            notes: String::new(),
        }
    }
}
