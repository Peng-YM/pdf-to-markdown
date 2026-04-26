use crate::error::Result;

const SERVICE_NAME: &str = "pdf-to-markdown";

/// Map a provider type string to the credential key used for storage.
pub fn provider_key(provider_type: &str) -> &str {
    if provider_type.starts_with("zhipu") {
        "zhipu"
    } else {
        "paddleocr"
    }
}

// ---------------------------------------------------------------------------
// macOS: use security-framework directly (avoids repeated keychain prompts
// by using SecItemAdd/SecItemCopyMatching instead of the legacy
// SecKeychainAddGenericPassword API that keyring uses)
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod platform {
    use crate::error::{anyhow, Result};
    use security_framework::passwords;

    pub fn get_credential(credential_key: &str) -> Result<Option<String>> {
        match passwords::get_generic_password(super::SERVICE_NAME, credential_key) {
            Ok(bytes) => Ok(Some(String::from_utf8(bytes)?)),
            Err(e) => {
                if e.code() == -25300 {
                    // errSecItemNotFound — no credential stored
                    return Ok(None);
                }
                crate::debug_print!("DEBUG: security-framework get failed: {}", e);
                Err(anyhow!("Failed to read from Keychain: {}", e))
            }
        }
    }

    pub fn set_credential(credential_key: &str, api_key: &str) -> Result<()> {
        passwords::set_generic_password(super::SERVICE_NAME, credential_key, api_key.as_bytes())?;
        Ok(())
    }

    pub fn delete_credential(credential_key: &str) -> Result<()> {
        match passwords::delete_generic_password(super::SERVICE_NAME, credential_key) {
            Ok(()) => Ok(()),
            Err(e) => {
                if e.code() == -25300 {
                    // errSecItemNotFound
                    return Err(anyhow!("No stored credential found for '{}'", credential_key));
                }
                Err(anyhow!("Failed to delete from Keychain: {}", e))
            }
        }
    }

    pub fn list_credentials() -> Result<Vec<String>> {
        let mut providers = Vec::new();
        for key in &["paddleocr", "zhipu"] {
            if passwords::get_generic_password(super::SERVICE_NAME, key).is_ok() {
                providers.push(key.to_string());
            }
        }
        Ok(providers)
    }
}

// ---------------------------------------------------------------------------
// Windows & Linux: use keyring crate
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "macos"))]
mod platform {
    use crate::error::{anyhow, Result};

    pub fn get_credential(credential_key: &str) -> Result<Option<String>> {
        // 1. Try system keychain
        match keyring::Entry::new(super::SERVICE_NAME, credential_key) {
            Ok(entry) => match entry.get_password() {
                Ok(password) => return Ok(Some(password)),
                Err(e) => {
                    crate::debug_print!("DEBUG: keyring get_password failed: {}", e);
                }
            },
            Err(e) => {
                crate::debug_print!("DEBUG: keyring Entry::new failed: {}", e);
            }
        }

        // 2. Fallback: encrypted file (Linux only)
        #[cfg(target_os = "linux")]
        {
            if let Some(password) = super::linux_fallback::get_from_file(credential_key)? {
                return Ok(Some(password));
            }
        }

        Ok(None)
    }

    pub fn set_credential(credential_key: &str, api_key: &str) -> Result<()> {
        // 1. Try system keychain
        match keyring::Entry::new(super::SERVICE_NAME, credential_key) {
            Ok(entry) => match entry.set_password(api_key) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    crate::debug_print!("DEBUG: keyring set_password failed: {}", e);
                }
            },
            Err(e) => {
                crate::debug_print!("DEBUG: keyring Entry::new failed: {}", e);
            }
        }

        // 2. Fallback: encrypted file (Linux only)
        #[cfg(target_os = "linux")]
        {
            super::linux_fallback::set_to_file(credential_key, api_key)
        }

        #[cfg(not(target_os = "linux"))]
        Err(anyhow!(
            "Failed to access system keychain. On Linux, ensure libdbus and libsecret are installed."
        ))
    }

    pub fn delete_credential(credential_key: &str) -> Result<()> {
        let mut deleted = false;

        // 1. Try system keychain
        match keyring::Entry::new(super::SERVICE_NAME, credential_key) {
            Ok(entry) => {
                if entry.delete_credential().is_ok() {
                    deleted = true;
                }
            }
            Err(e) => {
                crate::debug_print!("DEBUG: keyring Entry::new failed during delete: {}", e);
            }
        }

        // 2. Also try encrypted file (Linux only)
        #[cfg(target_os = "linux")]
        {
            if super::linux_fallback::delete_from_file(credential_key)? {
                deleted = true;
            }
        }

        if !deleted {
            return Err(anyhow!("No stored credential found for '{}'", credential_key));
        }

        Ok(())
    }

    pub fn list_credentials() -> Result<Vec<String>> {
        let mut providers = Vec::new();

        // Check known keys in keychain
        for key in &["paddleocr", "zhipu"] {
            if let Ok(entry) = keyring::Entry::new(super::SERVICE_NAME, key) {
                if entry.get_password().is_ok() {
                    providers.push(key.to_string());
                }
            }
        }

        // Also check encrypted file (Linux only)
        #[cfg(target_os = "linux")]
        {
            if let Ok(file_providers) = super::linux_fallback::list_from_file() {
                for p in file_providers {
                    if !providers.contains(&p) {
                        providers.push(p);
                    }
                }
            }
        }

        Ok(providers)
    }
}

// ---------------------------------------------------------------------------
// Linux encrypted file fallback
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux_fallback {
    use crate::error::{anyhow, Result};
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    use rand::rngs::OsRng;
    use rand::RngCore;
    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    const NONCE_SIZE: usize = 12;

    #[derive(Serialize, Deserialize, Default)]
    struct CredentialsFile {
        entries: HashMap<String, String>,
    }

    fn credentials_path() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("", "", "pdf-to-markdown")
            .ok_or_else(|| anyhow!("Could not determine config directory"))?;
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("credentials.enc"))
    }

    fn derive_key() -> Result<[u8; 32]> {
        let machine_id = std::fs::read_to_string("/etc/machine-id")
            .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))
            .unwrap_or_else(|_| "pdf-to-markdown-fallback-id".to_string());

        let mut hasher = Sha256::new();
        hasher.update(b"pdf-to-markdown-credential-v1");
        hasher.update(machine_id.trim().as_bytes());
        Ok(hasher.finalize().into())
    }

    fn read_credentials() -> Result<CredentialsFile> {
        let path = credentials_path()?;
        if !path.exists() {
            return Ok(CredentialsFile::default());
        }

        let data = fs::read(&path)?;
        if data.len() < NONCE_SIZE {
            return Err(anyhow!("Corrupted credentials file"));
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);
        let key = derive_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| anyhow!("Failed to initialize encryption"))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("Failed to decrypt credentials file"))?;

        let creds: CredentialsFile = serde_json::from_slice(&plaintext)?;
        Ok(creds)
    }

    fn write_credentials(creds: &CredentialsFile) -> Result<()> {
        let path = credentials_path()?;
        let plaintext = serde_json::to_vec(creds)?;

        let key = derive_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| anyhow!("Failed to initialize encryption"))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|_| anyhow!("Failed to encrypt credentials"))?;

        let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);

        fs::write(&path, output)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }
        Ok(())
    }

    pub fn get_from_file(credential_key: &str) -> Result<Option<String>> {
        let creds = read_credentials()?;
        Ok(creds.entries.get(credential_key).cloned())
    }

    pub fn set_to_file(credential_key: &str, api_key: &str) -> Result<()> {
        let mut creds = read_credentials()?;
        creds.entries.insert(credential_key.to_string(), api_key.to_string());
        write_credentials(&creds)
    }

    pub fn delete_from_file(credential_key: &str) -> Result<bool> {
        let mut creds = read_credentials()?;
        let existed = creds.entries.remove(credential_key).is_some();
        if existed {
            write_credentials(&creds)?;
            if creds.entries.is_empty() {
                let path = credentials_path()?;
                let _ = fs::remove_file(&path);
            }
        }
        Ok(existed)
    }

    pub fn list_from_file() -> Result<Vec<String>> {
        let creds = read_credentials()?;
        Ok(creds.entries.keys().cloned().collect())
    }
}

// ---------------------------------------------------------------------------
// Public API — delegates to platform-specific implementation
// ---------------------------------------------------------------------------

/// Retrieve a stored API key for the given credential key.
/// Returns `Ok(None)` if no credential is stored.
pub fn get_credential(credential_key: &str) -> Result<Option<String>> {
    platform::get_credential(credential_key)
}

/// Store an API key for the given credential key.
pub fn set_credential(credential_key: &str, api_key: &str) -> Result<()> {
    platform::set_credential(credential_key, api_key)
}

/// Delete a stored credential.
pub fn delete_credential(credential_key: &str) -> Result<()> {
    platform::delete_credential(credential_key)
}

/// List all providers that have stored credentials.
pub fn list_credentials() -> Result<Vec<String>> {
    platform::list_credentials()
}
