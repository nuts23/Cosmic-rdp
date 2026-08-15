use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraDeviceInfo {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
}

/// Enumerates local V4L2 / Linux webcam devices
pub fn list_local_cameras() -> Vec<CameraDeviceInfo> {
    let mut devices = Vec::new();

    // Check /dev/v4l/by-id first for stable names
    if let Ok(entries) = std::fs::read_dir("/dev/v4l/by-id") {
        for entry in entries.flatten() {
            let path = entry.path().to_string_lossy().to_string();
            let raw_name = entry.file_name().to_string_lossy().to_string();
            // Clean up long device id string for readable UI display
            let clean_name = raw_name
                .replace("usb-", "")
                .replace("_0001", "")
                .replace("-video-index0", " (Primary)")
                .replace("-video-index1", " (Secondary)")
                .replace('_', " ");
            devices.push(CameraDeviceInfo {
                path,
                name: clean_name,
            });
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

/// Enumerates local microphone and audio input devices (PipeWire / PulseAudio / ALSA)
pub fn list_local_audio_inputs() -> Vec<AudioDeviceInfo> {
    let mut devices = Vec::new();

    // 1. Try querying pactl for PipeWire / PulseAudio sources
    if let Ok(output) = Command::new("pactl").args(["list", "sources", "short"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    let id = parts[1].trim().to_string();
                    // Ignore monitor sinks
                    if !id.contains(".monitor") {
                        let clean_name = id
                            .replace("alsa_input.pci-0000_", "PCI ")
                            .replace(".HiFi__Mic1__source", " (Digital Mic 1)")
                            .replace(".HiFi__Mic2__source", " (Stereo Mic 2)")
                            .replace('_', " ");
                        devices.push(AudioDeviceInfo {
                            id: id.clone(),
                            name: clean_name,
                        });
                    }
                }
            }
        }
    }

    // 2. Fallback to /proc/asound/cards if no pactl devices found
    if devices.is_empty() {
        if let Ok(cards_content) = std::fs::read_to_string("/proc/asound/cards") {
            for line in cards_content.lines() {
                if line.contains('[') && line.contains(']') {
                    let parts: Vec<&str> = line.split('[').collect();
                    if parts.len() >= 2 {
                        let name_part = parts[1].split(']').next().unwrap_or("Audio Card");
                        devices.push(AudioDeviceInfo {
                            id: format!("hw:{}", name_part.trim()),
                            name: format!("ALSA: {}", name_part.trim()),
                        });
                    }
                }
            }
        }
    }

    devices
}
