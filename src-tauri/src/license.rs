//! Local trial and a pluggable license check.
//!
//! Enforcement defaults to off, so the beta keeps working. The development
//! checker uses a public test secret on purpose: anyone can mint a development
//! key. Set `license_endpoint` to your own server before charging money. That
//! server is the only checker once the address is set. No payment-provider
//! secret is compiled into the app.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const TRIAL_MS: u64 = 14 * 24 * 60 * 60 * 1000;
const DEV_LICENSE_SECRET: &[u8] = b"saywhen-dev-license-not-for-production-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseTier {
    Free,
    Trial,
    Pro,
}

impl LicenseTier {
    pub fn as_str(self) -> &'static str {
        match self {
            LicenseTier::Free => "free",
            LicenseTier::Trial => "trial",
            LicenseTier::Pro => "pro",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseEnforcement {
    Off,
    Soft,
    Required,
}

impl LicenseEnforcement {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "required" => LicenseEnforcement::Required,
            "soft" => LicenseEnforcement::Soft,
            _ => LicenseEnforcement::Off,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LicenseEnforcement::Off => "off",
            LicenseEnforcement::Soft => "soft",
            LicenseEnforcement::Required => "required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseFeature {
    Core,
    Snippets,
    Profiles,
    HistoryExport,
    CloudCleanup,
    Multilingual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseDecision {
    pub tier: LicenseTier,
    pub enforcement: LicenseEnforcement,
    pub trial_days_left: i64,
    pub clock_tampered: bool,
    pub key_present: bool,
    pub checker: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LicenseStateFile {
    trial_started_at_ms: u64,
    last_seen_at_ms: u64,
    #[serde(default)]
    clock_tampered: bool,
    #[serde(default)]
    cached_pro: bool,
    #[serde(default)]
    cached_expires_at_ms: Option<u64>,
}

pub fn license_allows(feature: LicenseFeature, decision: &LicenseDecision) -> bool {
    if decision.enforcement != LicenseEnforcement::Required {
        return true;
    }
    match feature {
        LicenseFeature::Core => true,
        LicenseFeature::Snippets
        | LicenseFeature::Profiles
        | LicenseFeature::HistoryExport
        | LicenseFeature::CloudCleanup
        | LicenseFeature::Multilingual => decision.tier == LicenseTier::Pro,
    }
}

pub fn evaluate_license(
    app_data_dir: &Path,
    now_ms: u64,
    enforcement: LicenseEnforcement,
    endpoint: &str,
    license_key: Option<&str>,
) -> LicenseDecision {
    let mut state = load_or_init_state(app_data_dir, now_ms);
    if now_ms + 120_000 < state.last_seen_at_ms {
        state.clock_tampered = true;
    }
    if now_ms > state.last_seen_at_ms {
        state.last_seen_at_ms = now_ms;
    }
    let _ = save_state(app_data_dir, &state);

    let trial_left_ms = state
        .trial_started_at_ms
        .saturating_add(TRIAL_MS)
        .saturating_sub(now_ms);
    let trial_alive = !state.clock_tampered && trial_left_ms > 0;
    let trial_days_left = if trial_alive {
        ((trial_left_ms + 86_400_000 - 1) / 86_400_000) as i64
    } else {
        0
    };

    let key = license_key.unwrap_or("").trim();
    let key_present = !key.is_empty();
    let endpoint = endpoint.trim();
    let (tier, checker, message) = if !endpoint.is_empty() {
        let server_pro = state.cached_pro
            && state
                .cached_expires_at_ms
                .map(|expires| now_ms < expires)
                .unwrap_or(true);
        if server_pro {
            (
                LicenseTier::Pro,
                "server",
                "Pro, confirmed by your license server.".to_string(),
            )
        } else if trial_alive {
            (
                LicenseTier::Trial,
                "server",
                "Trial is active. Check the license with your server to confirm Pro.".to_string(),
            )
        } else {
            (
                LicenseTier::Free,
                "server",
                "No confirmed Pro license from your server.".to_string(),
            )
        }
    } else if let Some(expires) = validate_dev_license_key(key, now_ms) {
        (
            LicenseTier::Pro,
            "development",
            format!(
                "Development Pro key, valid through {}. This checker is public and must not be how you charge money.",
                expires
            ),
        )
    } else if key_present {
        (
            if trial_alive {
                LicenseTier::Trial
            } else {
                LicenseTier::Free
            },
            "development",
            "That key is not a valid development key.".to_string(),
        )
    } else if trial_alive {
        (
            LicenseTier::Trial,
            "none",
            format!(
                "Trial, {trial_days_left} days left. Enforcement is {}.",
                enforcement.as_str()
            ),
        )
    } else if state.clock_tampered {
        (
            LicenseTier::Free,
            "none",
            "The clock moved backwards, so the trial is treated as finished.".to_string(),
        )
    } else {
        (
            LicenseTier::Free,
            "none",
            "The 14-day trial has ended.".to_string(),
        )
    };

    LicenseDecision {
        tier,
        enforcement,
        trial_days_left,
        clock_tampered: state.clock_tampered,
        key_present,
        checker,
        message,
    }
}

/// Accepts a development key of the form `SAYWHEN-PRO-YYYYMMDD-<64 hex hmac>`.
/// Returns the expiry date when the signature matches and the date is still ahead.
pub fn validate_dev_license_key(key: &str, now_ms: u64) -> Option<String> {
    let rest = key.trim().strip_prefix("SAYWHEN-PRO-")?;
    let (date, mac) = rest.split_once('-')?;
    if date.len() != 8 || !date.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    if mac.len() != 64 || !mac.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let expected = hex_encode(&hmac_sha256(
        DEV_LICENSE_SECRET,
        format!("pro|{date}").as_bytes(),
    ));
    if !expected.eq_ignore_ascii_case(mac) {
        return None;
    }
    let expiry_ms = date_to_unix_ms(date)?;
    if now_ms > expiry_ms.saturating_add(86_400_000) {
        return None;
    }
    Some(date.to_string())
}

pub fn issue_dev_license_key(date: &str) -> String {
    let mac = hex_encode(&hmac_sha256(
        DEV_LICENSE_SECRET,
        format!("pro|{date}").as_bytes(),
    ));
    format!("SAYWHEN-PRO-{date}-{mac}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerLicense {
    pub valid: bool,
    pub tier: String,
    pub expires_at_ms: Option<u64>,
}

pub fn interpret_license_server_body(status: u16, body: &str) -> Result<ServerLicense, String> {
    if !(200..300).contains(&status) {
        return Err(format!("License server returned HTTP {status}"));
    }
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|err| format!("License server sent invalid JSON: {err}"))?;
    let valid = value
        .get("valid")
        .and_then(|item| item.as_bool())
        .unwrap_or(false);
    let tier = value
        .get("tier")
        .and_then(|item| item.as_str())
        .unwrap_or("free")
        .to_string();
    let expires_at_ms = value.get("expiresAtMs").and_then(|item| item.as_u64());
    Ok(ServerLicense {
        valid,
        tier,
        expires_at_ms,
    })
}

pub fn cache_server_license(app_data_dir: &Path, now_ms: u64, license: &ServerLicense) {
    let mut state = load_or_init_state(app_data_dir, now_ms);
    state.cached_pro = license.valid && license.tier.eq_ignore_ascii_case("pro");
    state.cached_expires_at_ms = license.expires_at_ms;
    state.last_seen_at_ms = now_ms.max(state.last_seen_at_ms);
    let _ = save_state(app_data_dir, &state);
}

fn state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("license-state.json")
}

fn load_or_init_state(app_data_dir: &Path, now_ms: u64) -> LicenseStateFile {
    if let Ok(raw) = fs::read_to_string(state_path(app_data_dir)) {
        if let Ok(parsed) = serde_json::from_str::<LicenseStateFile>(&raw) {
            return parsed;
        }
    }
    LicenseStateFile {
        trial_started_at_ms: now_ms,
        last_seen_at_ms: now_ms,
        clock_tampered: false,
        cached_pro: false,
        cached_expires_at_ms: None,
    }
}

fn save_state(app_data_dir: &Path, state: &LicenseStateFile) -> Result<(), String> {
    fs::create_dir_all(app_data_dir)
        .map_err(|err| format!("Could not create the app data folder: {err}"))?;
    let text = serde_json::to_string_pretty(state)
        .map_err(|err| format!("Could not save the license state: {err}"))?;
    fs::write(state_path(app_data_dir), text)
        .map_err(|err| format!("Could not save the license state: {err}"))
}

fn date_to_unix_ms(date: &str) -> Option<u64> {
    let year: i64 = date[0..4].parse().ok()?;
    let month: u32 = date[4..6].parse().ok()?;
    let day: u32 = date[6..8].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    Some((days as u64).saturating_mul(86_400_000))
}

fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146097 + doe as i64) - 719468)
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let hashed = Sha256::digest(key);
        key_block[..32].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for index in 0..64 {
        ipad[index] ^= key_block[index];
        opad[index] ^= key_block[index];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    let digest = outer.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "saywhen-license-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn dev_key_roundtrip_and_forgery_rejected() {
        let key = issue_dev_license_key("20301231");
        assert!(validate_dev_license_key(&key, 1_700_000_000_000).is_some());
        let mut forged: Vec<char> = key.chars().collect();
        let last = forged.len() - 1;
        forged[last] = if forged[last] == 'a' { 'b' } else { 'a' };
        let forged: String = forged.into_iter().collect();
        assert!(validate_dev_license_key(&forged, 1_700_000_000_000).is_none());
        assert!(validate_dev_license_key("SAYWHEN-PRO-FOREVER", 0).is_none());
    }

    #[test]
    fn enforcement_off_allows_features_and_required_gates_them() {
        let dir = scratch();
        let now = 1_800_000_000_000;
        let open = evaluate_license(&dir, now, LicenseEnforcement::Off, "", None);
        assert_eq!(open.tier, LicenseTier::Trial);
        assert!(license_allows(LicenseFeature::Snippets, &open));
        assert!(license_allows(LicenseFeature::Core, &open));

        let expired_now = now + TRIAL_MS + 86_400_000;
        let closed = evaluate_license(&dir, expired_now, LicenseEnforcement::Required, "", None);
        assert_eq!(closed.tier, LicenseTier::Free);
        assert!(!license_allows(LicenseFeature::HistoryExport, &closed));
        assert!(license_allows(LicenseFeature::Core, &closed));

        let key = issue_dev_license_key("20301231");
        let pro = evaluate_license(
            &dir,
            expired_now,
            LicenseEnforcement::Required,
            "",
            Some(&key),
        );
        assert_eq!(pro.tier, LicenseTier::Pro);
        assert_eq!(pro.checker, "development");
        assert!(license_allows(LicenseFeature::CloudCleanup, &pro));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clock_rollback_ends_the_trial() {
        let dir = scratch();
        let now = 1_800_000_000_000;
        let _ = evaluate_license(&dir, now, LicenseEnforcement::Soft, "", None);
        let rolled = evaluate_license(
            &dir,
            now - 10 * 86_400_000,
            LicenseEnforcement::Soft,
            "",
            None,
        );
        assert!(rolled.clock_tampered);
        assert_eq!(rolled.tier, LicenseTier::Free);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn server_body_sets_the_cached_pro_flag() {
        let dir = scratch();
        let parsed = interpret_license_server_body(
            200,
            r#"{"valid":true,"tier":"pro","expiresAtMs":2000000000000}"#,
        )
        .unwrap();
        cache_server_license(&dir, 1_800_000_000_000, &parsed);
        let decision = evaluate_license(
            &dir,
            1_800_000_000_000,
            LicenseEnforcement::Required,
            "https://licenses.example.com/check",
            Some("anything"),
        );
        assert_eq!(decision.tier, LicenseTier::Pro);
        assert_eq!(decision.checker, "server");
        assert!(interpret_license_server_body(401, "{}").is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
