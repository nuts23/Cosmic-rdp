use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraDeviceInfo {
    pub path: String,
    pub name: String,
}

/// Enumerates local V4L2 / Linux webcam devices
pub fn list_local_cameras() -> Vec<CameraDeviceInfo> {
    let mut devices = Vec::new();

    // Check /dev/v4l/by-id first for stable names
    if let Ok(entries) = std::fs::read_dir("/dev/v4l/by-id") {
        for entry in entries.flatten() {
            let path = entry.path().to_string_lossy().to_string();
            let name = entry.file_name().to_string_lossy().to_string();
            devices.push(CameraDeviceInfo { path, name });
        }
    }

    // Fallback to /dev/video* if by-id is empty
    if devices.is_empty() {
        for i in 0..16 {
            let dev_path = format!("/dev/video{}", i);
            if Path::new(&dev_path).exists() {
                devices.push(CameraDeviceInfo {
                    path: dev_path.clone(),
                    name: format!("Camera ({})", dev_path),
                });
            }
        }
    }

    devices
}
