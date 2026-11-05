use crate::audio::UtteranceCompleteEvent;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// 7 days in milliseconds (`7 * 24 * 60 * 60 * 1000`).
pub const HISTORY_RETENTION_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Maximum number of recent utterances retained in local history.
pub const MAX_HISTORY_ENTRIES: usize = 200;

pub fn history_file_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("utterance-history.jsonl")
}

/// Loads persisted utterances from `<app_data_dir>/utterance-history.jsonl`, automatically
/// purging any entries older than 7 days (`HISTORY_RETENTION_MS`) and capping at
/// `MAX_HISTORY_ENTRIES`. If any entries were pruned, rewrites the file on disk so expired
/// transcripts are physically deleted.
///
/// Returns entries in chronological order (oldest first, newest last).
pub fn load_and_prune_history(app_data_dir: &Path, now_ms: u64) -> Vec<UtteranceCompleteEvent> {
    let path = history_file_path(app_data_dir);
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut total_lines = 0usize;
    let mut kept: Vec<UtteranceCompleteEvent> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_lines += 1;
        if let Ok(entry) = serde_json::from_str::<UtteranceCompleteEvent>(trimmed) {
            let age_ms = now_ms.saturating_sub(entry.timestamp_ms);
            if age_ms <= HISTORY_RETENTION_MS {
                kept.push(entry);
            }
        }
    }

    let pruned_by_age = kept.len() != total_lines;
    let pruned_by_cap = if kept.len() > MAX_HISTORY_ENTRIES {
        let drop_count = kept.len() - MAX_HISTORY_ENTRIES;
        kept.drain(0..drop_count);
        true
    } else {
        false
    };

    if pruned_by_age || pruned_by_cap {
        let _ = write_history_entries(app_data_dir, &kept);
    }

    kept
}

/// Appends a completed utterance to `<app_data_dir>/utterance-history.jsonl` after pruning
/// expired entries older than 7 days and enforcing `MAX_HISTORY_ENTRIES`.
pub fn append_history_entry(
    app_data_dir: &Path,
    entry: &UtteranceCompleteEvent,
    now_ms: u64,
) -> Result<(), String> {
    let mut entries = load_and_prune_history(app_data_dir, now_ms);
    entries.push(entry.clone());
    if entries.len() > MAX_HISTORY_ENTRIES {
        let drop_count = entries.len() - MAX_HISTORY_ENTRIES;
        entries.drain(0..drop_count);
    }
    write_history_entries(app_data_dir, &entries)
}

/// Updates `flagged_false_trigger` on a persisted utterance in `utterance-history.jsonl`.
pub fn update_history_false_trigger_flag(
    app_data_dir: &Path,
    utterance_id: u64,
    flagged: bool,
) -> Result<(), String> {
    let path = history_file_path(app_data_dir);
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read utterance-history.jsonl: {}", e))?;
    let mut entries: Vec<UtteranceCompleteEvent> = Vec::new();
    let mut updated = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(mut rec) = serde_json::from_str::<UtteranceCompleteEvent>(trimmed) {
            if rec.utterance_id == utterance_id {
                rec.flagged_false_trigger = flagged;
                updated = true;
            }
            entries.push(rec);
        }
    }

    if updated {
        write_history_entries(app_data_dir, &entries)?;
    }
    Ok(())
}

/// Deletes `<app_data_dir>/utterance-history.jsonl` from disk.
pub fn clear_utterance_history_file(app_data_dir: &Path) -> Result<(), String> {
    let path = history_file_path(app_data_dir);
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
    }
    Ok(())
}

fn write_history_entries(
    app_data_dir: &Path,
    entries: &[UtteranceCompleteEvent],
) -> Result<(), String> {
    fs::create_dir_all(app_data_dir).map_err(|e| {
        format!(
            "Failed to create app data dir {}: {}",
            app_data_dir.display(),
            e
        )
    })?;
    let path = history_file_path(app_data_dir);
    if entries.is_empty() {
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;

    for entry in entries {
        let line = serde_json::to_string(entry)
            .map_err(|e| format!("Failed to serialize history entry: {}", e))?;
        writeln!(file, "{}", line)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup::CleanupResult;
    use crate::inject::AppTargetPolicy;
    use crate::transcribe::TranscriptionResult;

    fn sample_event(id: u64, ts_ms: u64, text: &str) -> UtteranceCompleteEvent {
        UtteranceCompleteEvent {
            utterance_id: id,
            transcription: TranscriptionResult {
                text: text.to_string(),
                audio_duration_ms: 1200,
                inference_ms: 110,
                rtf: 0.09,
                model_used: "tiny.en".to_string(),
                peak_vad_prob: 0.96,
                end_phrase_triggered: false,
                discarded_reason: None,
            },
            cleanup: CleanupResult {
                raw_text: text.to_string(),
                cleaned_text: text.to_string(),
                modified: false,
                mode_used: "local_dev_rules".to_string(),
                is_destructive_command: false,
            },
            target_policy: AppTargetPolicy {
                app_name: "Terminal".to_string(),
                bundle_id: Some("com.apple.Terminal".to_string()),
                exe: None,
                pid: Some(1234),
                policy: "auto_send".to_string(),
                will_press_enter: true,
                reason: "Allowlisted".to_string(),
            },
            injection: None,
            injection_error: None,
            trigger_source: "hands_free_multimodal".to_string(),
            timestamp_ms: ts_ms,
            flagged_false_trigger: false,
        }
    }

    #[test]
    fn test_seven_day_history_prunes_expired_entries_and_persists_recent() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "saywhen_history_test_{}_{}",
            std::process::id(),
            1
        ));
        let _ = fs::remove_dir_all(&tmp_dir);

        let now_ms = 1_750_000_000_000u64;
        let eight_days_ago = now_ms - HISTORY_RETENTION_MS - 86_400_000;
        let two_days_ago = now_ms - (2 * 86_400_000);

        let old_ev = sample_event(10, eight_days_ago, "cargo check old");
        let recent_ev = sample_event(11, two_days_ago, "cargo test --release");
        let newest_ev = sample_event(12, now_ms, "git status --short");

        // Write an expired entry and two recent entries
        write_history_entries(&tmp_dir, &[old_ev, recent_ev]).unwrap();
        append_history_entry(&tmp_dir, &newest_ev, now_ms).unwrap();

        let loaded = load_and_prune_history(&tmp_dir, now_ms);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].utterance_id, 11);
        assert_eq!(loaded[1].utterance_id, 12);

        // Verify the expired command was physically removed from disk
        let raw_disk = fs::read_to_string(history_file_path(&tmp_dir)).unwrap();
        assert!(!raw_disk.contains("cargo check old"));
        assert!(raw_disk.contains("cargo test --release"));
        assert!(raw_disk.contains("git status --short"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_history_false_trigger_flag_and_clear_purges_disk() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "saywhen_history_test_{}_{}",
            std::process::id(),
            2
        ));
        let _ = fs::remove_dir_all(&tmp_dir);

        let now_ms = 1_750_000_000_000u64;
        let ev = sample_event(42, now_ms, "npm run build");
        append_history_entry(&tmp_dir, &ev, now_ms).unwrap();

        update_history_false_trigger_flag(&tmp_dir, 42, true).unwrap();
        let loaded = load_and_prune_history(&tmp_dir, now_ms);
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].flagged_false_trigger);

        clear_utterance_history_file(&tmp_dir).unwrap();
        assert!(!history_file_path(&tmp_dir).exists());
        assert!(load_and_prune_history(&tmp_dir, now_ms).is_empty());

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
