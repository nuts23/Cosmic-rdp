use cosmic_rdp_models::profile::AudioOutputMode;
use cosmic_rdp_models::rdp_file::{export_rdp_file, parse_rdp_file};

#[test]
fn test_parse_rdp_file() {
    let rdp_content = r#"
full address:s:rdp.company.com:3390
username:s:john_doe
domain:s:CORPNET
audiomode:i:0
audiocapturemode:i:1
camerastoredirect:s:*
smart sizing:i:1
desktopwidth:i:2560
desktopheight:i:1440
screen mode id:i:2
"#;

    let profile = parse_rdp_file(rdp_content, Some("Work_Session.rdp")).unwrap();
    assert_eq!(profile.name, "Work_Session");
    assert_eq!(profile.host, "rdp.company.com");
    assert_eq!(profile.port, 3390);
    assert_eq!(profile.username, "john_doe");
    assert_eq!(profile.domain, Some("CORPNET".to_string()));
    assert_eq!(profile.audio.output_mode, AudioOutputMode::PlayLocally);
    assert!(profile.audio.microphone_enabled);
    assert!(profile.camera.enabled);
    assert!(profile.display.dynamic_resolution);
    assert!(profile.display.fullscreen);
    assert_eq!(profile.display.custom_width, 2560);
    assert_eq!(profile.display.custom_height, 1440);
}

#[test]
fn test_roundtrip_rdp_file() {
    let original = cosmic_rdp_models::profile::ConnectionProfile::new("Prod Server", "10.10.1.50");
    let exported = export_rdp_file(&original);

    let parsed = parse_rdp_file(&exported, Some("Prod Server.rdp")).unwrap();
    assert_eq!(parsed.host, "10.10.1.50");
    assert_eq!(parsed.port, 3389);
    assert_eq!(parsed.audio.microphone_enabled, original.audio.microphone_enabled);
    assert_eq!(parsed.display.dynamic_resolution, original.display.dynamic_resolution);
}
