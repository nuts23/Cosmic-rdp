use cosmic_rdp_core::freerdp::{build_freerdp_arguments, xkb_to_rdp_scancode};
use cosmic_rdp_models::profile::{AudioOutputMode, ConnectionProfile};

#[test]
fn test_freerdp_argument_generation() {
    let mut profile = ConnectionProfile::new("Corp Laptop", "vpn.corp.com");
    profile.port = 3390;
    profile.username = "jdoe".to_string();
    profile.domain = Some("CORP".to_string());
    profile.audio.microphone_enabled = true;
    profile.audio.output_mode = AudioOutputMode::PlayLocally;
    profile.camera.enabled = true;
    profile.display.dynamic_resolution = true;

    let args = build_freerdp_arguments(&profile, Some("hunter2"));

    assert!(args.contains(&"/v:vpn.corp.com:3390".to_string()));
    assert!(args.contains(&"/u:jdoe".to_string()));
    assert!(args.contains(&"/d:CORP".to_string()));
    assert!(args.contains(&"/p:hunter2".to_string()));
    assert!(args.contains(&"/microphone".to_string()));
    assert!(args.contains(&"/sound".to_string()));
    assert!(args.contains(&"/dvc:rdpecam".to_string()));
    assert!(args.contains(&"+dynamic-resolution".to_string()));
    assert!(args.contains(&"/gfx".to_string()));
    assert!(args.contains(&"+video".to_string()));
}

#[test]
fn test_scancode_mapping() {
    // Alphanumeric keys
    let (code_a, ext_a) = xkb_to_rdp_scancode(38); // 'a'
    assert_eq!(code_a, 0x1E);
    assert_eq!(ext_a, false);

    // Escape key
    let (code_esc, ext_esc) = xkb_to_rdp_scancode(9);
    assert_eq!(code_esc, 0x01);
    assert_eq!(ext_esc, false);

    // Super / Windows key
    let (code_win, ext_win) = xkb_to_rdp_scancode(133);
    assert_eq!(code_win, 0x5B);
    assert_eq!(ext_win, true);
}
