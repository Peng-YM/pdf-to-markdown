use crate::error::Result;

/// Map a provider type string to the credential key used for storage.
pub fn provider_key(provider_type: &str) -> &str {
    if provider_type.starts_with("zhipu") {
        "zhipu"
    } else if provider_type.starts_with("mineru") {
        "mineru"
    } else {
        "paddleocr"
    }
}

// ---------------------------------------------------------------------------
// Encrypted file storage (all platforms)
// ---------------------------------------------------------------------------

mod platform {
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

    #[cfg(target_os = "linux")]
    fn machine_id() -> String {
        std::fs::read_to_string("/etc/machine-id")
            .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))
            .unwrap_or_else(|_| "pdf-to-markdown-fallback-id".to_string())
    }

    #[cfg(target_os = "macos")]
    fn machine_id() -> String {
        let output = std::process::Command::new("ioreg")
            .args(["-d2", "-c", "IOPlatformExpertDevice"])
            .output();
        if let Ok(out) = output {
            if let Ok(plist) = String::from_utf8(out.stdout) {
                if let Some(line) = plist.lines().find(|l| l.contains("IOPlatformUUID")) {
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start + 1..].find('"') {
                            return line[start + 1..start + 1 + end].to_string();
                        }
                    }
                }
            }
        }
        "pdf-to-markdown-fallback-id".to_string()
    }

    #[cfg(target_os = "windows")]
    fn machine_id() -> String {
        let output = std::process::Command::new("reg")
            .args([
                "query",
                r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography",
                "/v",
                "MachineGuid",
            ])
            .output();
        if let Ok(out) = output {
            if let Ok(text) = String::from_utf8(out.stdout) {
                for line in text.lines() {
                    if let Some(pos) = line.find("REG_SZ") {
                        return line[pos + 6..].trim().to_string();
                    }
                }
            }
        }
        "pdf-to-markdown-fallback-id".to_string()
    }

    fn derive_key() -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();
        hasher.update(b"pdf-to-markdown-credential-v1");
        hasher.update(machine_id().trim().as_bytes());
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

    pub fn get(credential_key: &str) -> Result<Option<String>> {
        let creds = read_credentials()?;
        Ok(creds.entries.get(credential_key).cloned())
    }

    pub fn set(credential_key: &str, api_key: &str) -> Result<()> {
        let mut creds = read_credentials()?;
        creds.entries.insert(credential_key.to_string(), api_key.to_string());
        write_credentials(&creds)
    }

    pub fn delete(credential_key: &str) -> Result<bool> {
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

    pub fn list() -> Result<Vec<String>> {
        let creds = read_credentials()?;
        Ok(creds.entries.keys().cloned().collect())
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Retrieve a stored API key for the given credential key.
/// Returns `Ok(None)` if no credential is stored.
pub fn get_credential(credential_key: &str) -> Result<Option<String>> {
    platform::get(credential_key)
}

/// Store an API key for the given credential key.
pub fn set_credential(credential_key: &str, api_key: &str) -> Result<()> {
    platform::set(credential_key, api_key)
}

/// Delete a stored credential.
pub fn delete_credential(credential_key: &str) -> Result<()> {
    if platform::delete(credential_key)? {
        Ok(())
    } else {
        Err(crate::error::anyhow!("No stored credential found for '{}'", credential_key))
    }
}

/// List all providers that have stored credentials.
pub fn list_credentials() -> Result<Vec<String>> {
    platform::list()
}
