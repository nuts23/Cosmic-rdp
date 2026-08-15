use crate::profile::{
    AudioOutputMode, AudioSettings, CameraSettings, CertificatePolicy, ConnectionProfile,
    DisplaySettings, ProfileId, ScalingMode,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RdpFileError {
    #[error("Missing full address or host in RDP file")]
    MissingHost,
    #[error("Invalid line format: {0}")]
    InvalidFormat(String),
}

/// Parse standard Microsoft .rdp file format into a ConnectionProfile
pub fn parse_rdp_file(content: &str, file_name_hint: Option<&str>) -> Result<ConnectionProfile, RdpFileError> {
    let mut entries = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Format is typically `key:type:value` (e.g. `full address:s:192.168.1.100:3389`)
        let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
        if parts.len() == 3 {
            let key = parts[0].trim().to_lowercase();
            let _val_type = parts[1].trim();
            let val = parts[2].trim();
            entries.insert(key, val.to_string());
        } else if parts.len() == 2 {
            let key = parts[0].trim().to_lowercase();
            let val = parts[1].trim();
            entries.insert(key, val.to_string());
        }
    }

    let full_address = entries
        .get("full address")
        .or_else(|| entries.get("alternate full address"))
        .or_else(|| entries.get("server name"))
        .ok_or(RdpFileError::MissingHost)?;

    let (host, port) = if let Some(idx) = full_address.rfind(':') {
        let h = &full_address[..idx];
        let p = full_address[idx + 1..].parse::<u16>().unwrap_or(3389);
        (h.to_string(), p)
    } else {
        (full_address.to_string(), 3389)
    };

    let username = entries.get("username").cloned().unwrap_or_default();
    let domain = entries.get("domain").cloned().filter(|d| !d.is_empty());

    // Audio settings
    // audiomode:i:0 = bring to this computer, 1 = leave at remote computer, 2 = do not play
    let audiomode = entries.get("audiomode").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let output_mode = match audiomode {
        0 => AudioOutputMode::PlayLocally,
        1 => AudioOutputMode::PlayOnRemote,
        _ => AudioOutputMode::DoNotPlay,
    };

    // audiocapturemode:i:1 = record from this computer (mic redirect)
    let audiocapturemode = entries.get("audiocapturemode").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let microphone_enabled = audiocapturemode == 1;

    // Camera settings
    let camerastoredirect = entries.get("camerastoredirect").cloned().unwrap_or_default();
    let camera_enabled = camerastoredirect == "*" || !camerastoredirect.is_empty();

    // Display settings
    let dynamic_resolution = entries
        .get("smart sizing")
        .or_else(|| entries.get("dynamic resolution"))
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v == 1)
        .unwrap_or(true);

    let screen_mode = entries.get("screen mode id").and_then(|v| v.parse::<u32>().ok()).unwrap_or(1);
    let fullscreen = screen_mode == 2;

    let custom_width = entries.get("desktopwidth").and_then(|v| v.parse::<u32>().ok()).unwrap_or(1920);
    let custom_height = entries.get("desktopheight").and_then(|v| v.parse::<u32>().ok()).unwrap_or(1080);

    let name = file_name_hint
        .unwrap_or_else(|| host.as_str())
        .trim_end_matches(".rdp")
        .to_string();

    Ok(ConnectionProfile {
        id: ProfileId::new(),
        name,
        host,
        port,
        username,
        domain,
        cert_fingerprint: None,
        cert_policy: CertificatePolicy::TrustOnFirstUse,
        audio: AudioSettings {
            microphone_enabled,
            output_mode,
            low_latency: true,
            preferred_mic_device: None,
        },
        camera: CameraSettings {
            enabled: camera_enabled,
            device_name: if camerastoredirect == "*" || camerastoredirect.is_empty() {
                None
            } else {
                Some(camerastoredirect)
            },
        },
        display: DisplaySettings {
            dynamic_resolution,
            scaling_mode: ScalingMode::DynamicResize,
            fullscreen,
            custom_width,
            custom_height,
            scale_factor: 100,
        },
        color_tag: None,
        last_connected: None,
        notes: String::new(),
    })
}

/// Export a ConnectionProfile to standard Microsoft .rdp format string
pub fn export_rdp_file(profile: &ConnectionProfile) -> String {
    let mut out = String::new();
    out.push_str(&format!("full address:s:{}:{}\r\n", profile.host, profile.port));

    if !profile.username.is_empty() {
        out.push_str(&format!("username:s:{}\r\n", profile.username));
    }
    if let Some(ref domain) = profile.domain {
        out.push_str(&format!("domain:s:{}\r\n", domain));
    }

    // Audio
    let audio_val = match profile.audio.output_mode {
        AudioOutputMode::PlayLocally => 0,
        AudioOutputMode::PlayOnRemote => 1,
        AudioOutputMode::DoNotPlay => 2,
    };
    out.push_str(&format!("audiomode:i:{}\r\n", audio_val));
    out.push_str(&format!(
        "audiocapturemode:i:{}\r\n",
        if profile.audio.microphone_enabled { 1 } else { 0 }
    ));

    // Camera
    if profile.camera.enabled {
        let cam_str = profile.camera.device_name.as_deref().unwrap_or("*");
        out.push_str(&format!("camerastoredirect:s:{}\r\n", cam_str));
    }

    // Display
    out.push_str(&format!(
        "smart sizing:i:{}\r\n",
        if profile.display.dynamic_resolution { 1 } else { 0 }
    ));
    out.push_str(&format!(
        "screen mode id:i:{}\r\n",
        if profile.display.fullscreen { 2 } else { 1 }
    ));
    out.push_str(&format!("desktopwidth:i:{}\r\n", profile.display.custom_width));
    out.push_str(&format!("desktopheight:i:{}\r\n", profile.display.custom_height));

    out
}
