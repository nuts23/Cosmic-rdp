use cosmic_rdp_models::profile::{AudioOutputMode, ConnectionProfile};
use cosmic_rdp_models::store::ProfileStore;
use tempfile::tempdir;

#[test]
fn test_profile_creation_and_defaults() {
    let profile = ConnectionProfile::new("Test Machine", "192.168.1.100");
    assert_eq!(profile.name, "Test Machine");
    assert_eq!(profile.host, "192.168.1.100");
    assert_eq!(profile.port, 3389);
    assert!(profile.audio.microphone_enabled);
    assert_eq!(profile.audio.output_mode, AudioOutputMode::PlayLocally);
    assert!(profile.camera.enabled);
    assert!(profile.display.dynamic_resolution);
}

#[test]
fn test_store_save_and_reload() {
    let tmp = tempdir().unwrap();
    let mut store = ProfileStore::load_from_dir(tmp.path()).unwrap();
    assert_eq!(store.list().len(), 0);

    let mut p1 = ConnectionProfile::new("Work PC", "work.company.local");
    p1.username = "alice".to_string();
    p1.domain = Some("CORP".to_string());
    let id1 = p1.id;
    store.upsert(p1).unwrap();

    let p2 = ConnectionProfile::new("Dev Box", "10.0.0.5");
    let id2 = p2.id;
    store.upsert(p2).unwrap();

    assert_eq!(store.list().len(), 2);

    // Reload from disk
    let reloaded = ProfileStore::load_from_dir(tmp.path()).unwrap();
    let item1 = reloaded.get(&id1).expect("profile 1 exists");
    assert_eq!(item1.name, "Work PC");
    assert_eq!(item1.username, "alice");
    assert_eq!(item1.domain, Some("CORP".to_string()));

    let item2 = reloaded.get(&id2).expect("profile 2 exists");
    assert_eq!(item2.name, "Dev Box");
}

#[test]
fn test_store_remove() {
    let tmp = tempdir().unwrap();
    let mut store = ProfileStore::load_from_dir(tmp.path()).unwrap();

    let p = ConnectionProfile::new("To Remove", "1.2.3.4");
    let id = p.id;
    store.upsert(p).unwrap();
    assert_eq!(store.list().len(), 1);

    let removed = store.remove(&id).unwrap();
    assert!(removed.is_some());
    assert_eq!(store.list().len(), 0);

    let reloaded = ProfileStore::load_from_dir(tmp.path()).unwrap();
    assert_eq!(reloaded.list().len(), 0);
}
