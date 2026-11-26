//! Local secret storage for opt-in credentials.
//!
//! API keys are never written into `settings.local.json` and never placed on a
//! process command line. macOS and Windows try the OS keychain first. Every
//! platform can fall back to a user-only file under the app data directory.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const LLM_CLEANUP_API_KEY: &str = "llm-cleanup-api-key";
pub const LICENSE_KEY_ACCOUNT: &str = "license-key";

pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, value: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
    /// Where a present secret lives: `"keyring"`, `"file"`, or `"none"`.
    fn backend(&self, account: &str) -> String;
}

pub struct FileSecretStore {
    dir: PathBuf,
}

impl FileSecretStore {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            dir: app_data_dir.join("secrets"),
        }
    }

    fn account_path(&self, account: &str) -> Result<PathBuf, String> {
        if account.is_empty()
            || !account
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("Invalid secret name".to_string());
        }
        Ok(self.dir.join(format!("{account}.secret")))
    }
}

impl SecretStore for FileSecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        let path = self.account_path(account)?;
        match fs::read_to_string(&path) {
            Ok(raw) => {
                let trimmed = raw.trim().to_string();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(trimmed))
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(format!("Could not read the saved secret: {err}")),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        let path = self.account_path(account)?;
        fs::create_dir_all(&self.dir)
            .map_err(|err| format!("Could not create the secrets directory: {err}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.dir, fs::Permissions::from_mode(0o700));
        }
        write_private_file(&path, value.trim())?;
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        let path = self.account_path(account)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!("Could not remove the saved secret: {err}")),
        }
    }

    fn backend(&self, account: &str) -> String {
        match self.get(account) {
            Ok(Some(_)) => "file".to_string(),
            _ => "none".to_string(),
        }
    }
}

/// Keychain when the OS has one, otherwise the private file store.
pub struct DefaultSecretStore {
    file: FileSecretStore,
    prefer_keyring: bool,
}

impl DefaultSecretStore {
    pub fn open(app_data_dir: &Path) -> Self {
        let prefer_keyring = cfg!(all(
            feature = "os-keychain",
            any(target_os = "macos", target_os = "windows")
        ));
        Self {
            file: FileSecretStore::new(app_data_dir),
            prefer_keyring,
        }
    }
}

impl SecretStore for DefaultSecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        if self.prefer_keyring {
            match keyring_get(account) {
                Ok(Some(value)) => return Ok(Some(value)),
                Ok(None) => {}
                Err(err) => {
                    eprintln!("saywhen: keychain read failed ({err}); trying the private file");
                }
            }
        }
        self.file.get(account)
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        if self.prefer_keyring {
            match keyring_set(account, value) {
                Ok(()) => {
                    let _ = self.file.delete(account);
                    return Ok(());
                }
                Err(err) => {
                    eprintln!("saywhen: keychain save failed ({err}); using a private file");
                }
            }
        }
        self.file.set(account, value)
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        let mut first_err = None;
        if self.prefer_keyring {
            if let Err(err) = keyring_delete(account) {
                first_err = Some(err);
            }
        }
        self.file.delete(account)?;
        if let Some(err) = first_err {
            if self.file.get(account)?.is_some() {
                return Err(err);
            }
        }
        Ok(())
    }

    fn backend(&self, account: &str) -> String {
        if self.prefer_keyring {
            match keyring_get(account) {
                Ok(Some(_)) => return "keyring".to_string(),
                Ok(None) => {}
                Err(_) => {}
            }
        }
        self.file.backend(account)
    }
}

pub fn validate_secret_value(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("The value is empty".to_string());
    }
    if value.chars().count() > 512 {
        return Err("That value is too long".to_string());
    }
    if value.chars().any(|c| c.is_control()) {
        return Err("That value contains characters that are not allowed".to_string());
    }
    Ok(())
}

/// If an older `settings.local.json` stored `llmApiKey` in plain text, lift it
/// out and rewrite the file without that field. Returns the key when one was present.
pub fn take_legacy_llm_api_key(settings_path: &Path) -> Option<String> {
    let raw = fs::read_to_string(settings_path).ok()?;
    let mut value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let key = value
        .get("llmApiKey")
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)?;
    let obj = value.as_object_mut()?;
    obj.remove("llmApiKey");
    if let Ok(text) = serde_json::to_string_pretty(&value) {
        let _ = write_private_file(settings_path, &text);
    }
    Some(key)
}

fn write_private_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Could not create {}: {err}", parent.display()))?;
        }
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| format!("Could not write {}: {err}", path.display()))?;
    file.write_all(contents.as_bytes())
        .map_err(|err| format!("Could not write {}: {err}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(all(
    feature = "os-keychain",
    any(target_os = "macos", target_os = "windows")
))]
fn keyring_entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("dev.saywhen.app", account).map_err(|err| err.to_string())
}

#[cfg(all(
    feature = "os-keychain",
    any(target_os = "macos", target_os = "windows")
))]
fn keyring_get(account: &str) -> Result<Option<String>, String> {
    let entry = keyring_entry(account)?;
    match entry.get_password() {
        Ok(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed))
            }
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(all(
    feature = "os-keychain",
    any(target_os = "macos", target_os = "windows")
))]
fn keyring_set(account: &str, value: &str) -> Result<(), String> {
    keyring_entry(account)?
        .set_password(value.trim())
        .map_err(|err| err.to_string())
}

#[cfg(all(
    feature = "os-keychain",
    any(target_os = "macos", target_os = "windows")
))]
fn keyring_delete(account: &str) -> Result<(), String> {
    match keyring_entry(account)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(not(all(
    feature = "os-keychain",
    any(target_os = "macos", target_os = "windows")
)))]
fn keyring_get(_account: &str) -> Result<Option<String>, String> {
    Err("The system keychain is not used on this platform".to_string())
}

#[cfg(not(all(
    feature = "os-keychain",
    any(target_os = "macos", target_os = "windows")
)))]
fn keyring_set(_account: &str, _value: &str) -> Result<(), String> {
    Err("The system keychain is not used on this platform".to_string())
}

#[cfg(not(all(
    feature = "os-keychain",
    any(target_os = "macos", target_os = "windows")
)))]
fn keyring_delete(_account: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("saywhen-secrets-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn file_store_roundtrip_is_private_and_absent_after_delete() {
        let dir = scratch_dir();
        let store = FileSecretStore::new(&dir);
        assert_eq!(store.get(LLM_CLEANUP_API_KEY).unwrap(), None);
        store.set(LLM_CLEANUP_API_KEY, "sk-test-value").unwrap();
        assert_eq!(
            store.get(LLM_CLEANUP_API_KEY).unwrap().as_deref(),
            Some("sk-test-value")
        );
        assert_eq!(store.backend(LLM_CLEANUP_API_KEY), "file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.join("secrets").join("llm-cleanup-api-key.secret");
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        store.delete(LLM_CLEANUP_API_KEY).unwrap();
        assert_eq!(store.get(LLM_CLEANUP_API_KEY).unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_control_characters_in_secrets() {
        assert!(validate_secret_value("good-key").is_ok());
        assert!(validate_secret_value("bad\nkey").is_err());
        assert!(validate_secret_value("").is_err());
    }

    #[test]
    fn legacy_settings_key_is_removed_from_json() {
        let dir = scratch_dir();
        let path = dir.join("settings.local.json");
        fs::write(
            &path,
            r#"{"llmApiKey":"sk-legacy","handsFreeEnabled":true}"#,
        )
        .unwrap();
        let lifted = take_legacy_llm_api_key(&path).unwrap();
        assert_eq!(lifted, "sk-legacy");
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("sk-legacy"));
        assert!(!raw.contains("llmApiKey"));
        assert!(raw.contains("handsFreeEnabled"));
        let _ = fs::remove_dir_all(&dir);
    }
}
