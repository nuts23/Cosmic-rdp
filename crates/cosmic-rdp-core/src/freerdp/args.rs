use cosmic_rdp_models::profile::{
    AudioOutputMode, CertificatePolicy, ConnectionProfile,
};

/// Builds the FreeRDP command line arguments for a connection profile
pub fn build_freerdp_arguments(
    profile: &ConnectionProfile,
    password: Option<&str>,
) -> Vec<String> {
    let mut args = Vec::new();

    // Target Host and Port
    args.push(format!("/v:{}:{}", profile.host, profile.port));

    // Authentication
    if !profile.username.is_empty() {
        args.push(format!("/u:{}", profile.username));
    }
    if let Some(ref domain) = profile.domain {
        args.push(format!("/d:{}", domain));
    }
    if let Some(pwd) = password {
        if !pwd.is_empty() {
            args.push(format!("/p:{}", pwd));
        }
    }

    // Audio Output (rdpsnd)
    match profile.audio.output_mode {
        AudioOutputMode::PlayLocally => {
            args.push("/sound".to_string());
        }
        AudioOutputMode::PlayOnRemote => {
            args.push("/sound:sys:remote".to_string());
        }
        AudioOutputMode::DoNotPlay => {
            args.push("-sound".to_string());
        }
    }

    // Microphone Input (audin) - critical for Teams
    if profile.audio.microphone_enabled {
        if let Some(ref mic) = profile.audio.preferred_mic_device {
            args.push(format!("/microphone:sys:pulse,dev:{}", mic));
        } else {
            args.push("/microphone".to_string());
        }
    } else {
        args.push("-microphone".to_string());
    }

    // Camera Passthrough (rdpecam DVC channel) - critical for Teams
    if profile.camera.enabled {
        if let Some(ref dev) = profile.camera.device_name {
            args.push(format!("/dvc:rdpecam,device:{}", dev));
        } else {
            args.push("/dvc:rdpecam".to_string());
        }
    }

    // Display & Dynamic Resolution Scaling
    if profile.display.dynamic_resolution {
        args.push("+dynamic-resolution".to_string());
        args.push("+auto-reconnect".to_string());
    } else {
        args.push(format!("/size:{}x{}", profile.display.custom_width, profile.display.custom_height));
    }

    if profile.display.fullscreen {
        args.push("/f".to_string());
    }

    // Certificate Policy
    match profile.cert_policy {
        CertificatePolicy::Strict => {
            args.push("/cert:deny".to_string());
        }
        CertificatePolicy::TrustOnFirstUse | CertificatePolicy::Ignore => {
            args.push("/cert:ignore".to_string());
        }
    }

    // High performance graphics & desktop experience
    args.push("/gfx".to_string());
    args.push("+rfx".to_string());
    args.push("+video".to_string());
    args.push("+clipboard".to_string());
    args.push("+fonts".to_string());
    args.push("+aero".to_string());
    args.push("+window-drag".to_string());
    args.push("+menu-anims".to_string());

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_arguments_teams_profile() {
        let mut profile = ConnectionProfile::new("Office PC", "10.0.1.25");
        profile.username = "bob".to_string();
        profile.domain = Some("CONTOSO".to_string());
        profile.audio.microphone_enabled = true;
        profile.camera.enabled = true;
        profile.display.dynamic_resolution = true;

        let args = build_freerdp_arguments(&profile, Some("SecretPass123!"));
        assert!(args.contains(&"/v:10.0.1.25:3389".to_string()));
        assert!(args.contains(&"/u:bob".to_string()));
        assert!(args.contains(&"/d:CONTOSO".to_string()));
        assert!(args.contains(&"/p:SecretPass123!".to_string()));
        assert!(args.contains(&"/microphone".to_string()));
        assert!(args.contains(&"/sound".to_string()));
        assert!(args.contains(&"/dvc:rdpecam".to_string()));
        assert!(args.contains(&"+dynamic-resolution".to_string()));
        assert!(args.contains(&"+video".to_string()));
    }
}
