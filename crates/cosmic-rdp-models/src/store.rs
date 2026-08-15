use crate::profile::{ConnectionProfile, ProfileId};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Disk store managing user's saved connection profiles
#[derive(Debug, Clone, Default)]
pub struct ProfileStore {
    profiles: HashMap<ProfileId, ConnectionProfile>,
    config_dir: PathBuf,
}

impl ProfileStore {
    /// Initialize profile store from default user config directory
    pub fn load_default() -> Result<Self, StoreError> {
        let config_dir = dirs_or_fallback();
        Self::load_from_dir(&config_dir)
    }

    /// Load profiles from a specific directory
    pub fn load_from_dir(dir: &Path) -> Result<Self, StoreError> {
        let file_path = dir.join("profiles.json");
        if !file_path.exists() {
            debug!("Profiles file does not exist at {:?}, initializing empty store", file_path);
            return Ok(Self {
                profiles: HashMap::new(),
                config_dir: dir.to_path_buf(),
            });
        }

        let content = fs::read_to_string(&file_path)?;
        let profile_list: Vec<ConnectionProfile> = serde_json::from_str(&content)?;
        let mut profiles = HashMap::new();
        for p in profile_list {
            profiles.insert(p.id, p);
        }

        info!("Loaded {} profiles from {:?}", profiles.len(), file_path);
        Ok(Self {
            profiles,
            config_dir: dir.to_path_buf(),
        })
    }

    /// Save all profiles to disk
    pub fn save(&self) -> Result<(), StoreError> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir)?;
        }
        let file_path = self.config_dir.join("profiles.json");
        let list: Vec<&ConnectionProfile> = self.profiles.values().collect();
        let json = serde_json::to_string_pretty(&list)?;
        fs::write(&file_path, json)?;
        debug!("Saved {} profiles to {:?}", self.profiles.len(), file_path);
        Ok(())
    }

    /// Get a profile by ID
    pub fn get(&self, id: &ProfileId) -> Option<&ConnectionProfile> {
        self.profiles.get(id)
    }

    /// List all profiles sorted by name
    pub fn list(&self) -> Vec<ConnectionProfile> {
        let mut list: Vec<ConnectionProfile> = self.profiles.values().cloned().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    /// Insert or update a profile
    pub fn upsert(&mut self, profile: ConnectionProfile) -> Result<(), StoreError> {
        self.profiles.insert(profile.id, profile);
        self.save()
    }

    /// Remove a profile
    pub fn remove(&mut self, id: &ProfileId) -> Result<Option<ConnectionProfile>, StoreError> {
        let removed = self.profiles.remove(id);
        if removed.is_some() {
            self.save()?;
        }
        Ok(removed)
    }
}

fn dirs_or_fallback() -> PathBuf {
    if let Some(config_dir) = dirs_next() {
        config_dir.join("cosmic-rdp")
    } else {
        PathBuf::from(".config/cosmic-rdp")
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
        })
}
