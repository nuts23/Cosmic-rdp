use crate::profile::ProfileId;
use oo7::Keyring;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, info};

const APP_ID: &str = "dev.cosmic.Rdp";
const SCHEMA_NAME: &str = "dev.cosmic.Rdp.Password";

#[derive(Error, Debug)]
pub enum KeyringError {
    #[error("Secret service error: {0}")]
    SecretService(#[from] oo7::Error),
    #[error("Password not found in keyring for profile {0}")]
    NotFound(ProfileId),
}

/// Helper for storing and retrieving connection passwords via Freedesktop Secret Service
pub struct SecretStore;

impl SecretStore {
    /// Save password to the system keyring for a profile
    pub async fn save_password(profile_id: ProfileId, password: &str) -> Result<(), KeyringError> {
        debug!("Saving password to keyring for profile {}", profile_id);
        let keyring = Keyring::new().await?;
        let profile_str = profile_id.to_string();
        let mut attributes = HashMap::new();
        attributes.insert("app", APP_ID);
        attributes.insert("schema", SCHEMA_NAME);
        attributes.insert("profile_id", profile_str.as_str());

        keyring
            .create_item(
                &format!("Cosmic RDP: {}", profile_id),
                &attributes,
                password.as_bytes(),
                true, // replace if exists
            )
            .await?;

        info!("Password saved securely to keyring for profile {}", profile_id);
        Ok(())
    }

    /// Retrieve password from system keyring for a profile
    pub async fn get_password(profile_id: ProfileId) -> Result<String, KeyringError> {
        let keyring = Keyring::new().await?;
        let profile_str = profile_id.to_string();
        let mut attributes = HashMap::new();
        attributes.insert("app", APP_ID);
        attributes.insert("schema", SCHEMA_NAME);
        attributes.insert("profile_id", profile_str.as_str());

        let items = keyring.search_items(&attributes).await?;
        if let Some(item) = items.into_iter().next() {
            let secret = item.secret().await?;
            let password = String::from_utf8_lossy(&secret).to_string();
            Ok(password)
        } else {
            Err(KeyringError::NotFound(profile_id))
        }
    }

    /// Delete password from system keyring for a profile
    pub async fn delete_password(profile_id: ProfileId) -> Result<(), KeyringError> {
        let keyring = Keyring::new().await?;
        let profile_str = profile_id.to_string();
        let mut attributes = HashMap::new();
        attributes.insert("app", APP_ID);
        attributes.insert("schema", SCHEMA_NAME);
        attributes.insert("profile_id", profile_str.as_str());

        keyring.delete(&attributes).await?;
        Ok(())
    }
}
