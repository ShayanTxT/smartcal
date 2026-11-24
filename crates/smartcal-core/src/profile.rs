use crate::error::{Result, SmartCalError};
use crate::logging::record_log;
use crate::types::{
    AppSettings, BackupRecord, CalibrationProfile, DisplayInfo, InputDeviceInfo, LogLevel,
};
use chrono::Utc;
use directories::{BaseDirs, ProjectDirs};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProfileStore {
    config_dir: PathBuf,
    profiles_dir: PathBuf,
    backups_dir: PathBuf,
}

impl ProfileStore {
    pub fn open_default() -> Self {
        let config_dir = ProjectDirs::from("org", "smartcal", "smartcal")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Self::with_root(config_dir)
    }

    pub fn with_root(config_dir: PathBuf) -> Self {
        let profiles_dir = config_dir.join("profiles");
        let backups_dir = config_dir.join("backups");
        let _ = fs::create_dir_all(&profiles_dir);
        let _ = fs::create_dir_all(&backups_dir);
        Self {
            config_dir,
            profiles_dir,
            backups_dir,
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn profiles_dir(&self) -> &Path {
        &self.profiles_dir
    }

    pub fn backups_dir(&self) -> &Path {
        &self.backups_dir
    }

    pub fn sanitize_name(name: &str) -> String {
        let cleaned: String = name
            .trim()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let trimmed = cleaned.trim_matches('-');
        if trimmed.is_empty() {
            "default-calibration".to_string()
        } else {
            trimmed.to_string()
        }
    }

    pub fn profile_path(&self, name: &str) -> PathBuf {
        let stem = Self::sanitize_name(name.trim_end_matches(".json"));
        self.profiles_dir.join(format!("{stem}.json"))
    }

    pub fn save_profile(&self, profile: &CalibrationProfile) -> Result<PathBuf> {
        fs::create_dir_all(&self.profiles_dir)?;
        let path = self.profile_path(&profile.profile_name);
        let json = serde_json::to_string_pretty(profile)?;
        fs::write(&path, json)?;
        record_log(
            LogLevel::Info,
            "profiles",
            format!(
                "Saved profile '{}' to {}",
                profile.profile_name,
                path.display()
            ),
        );
        Ok(path)
    }

    pub fn load_profile(&self, name_or_path: &str) -> Result<CalibrationProfile> {
        let candidate = Path::new(name_or_path);
        let resolved = if candidate.exists() {
            candidate.to_path_buf()
        } else {
            let in_config = self.profile_path(name_or_path);
            if in_config.exists() {
                in_config
            } else {
                let local_profiles = Path::new("profiles")
                    .join(format!("{}.json", Self::sanitize_name(name_or_path)));
                if local_profiles.exists() {
                    local_profiles
                } else {
                    return Err(SmartCalError::Profile(format!(
                        "Calibration profile '{name_or_path}' not found"
                    )));
                }
            }
        };

        let content = fs::read_to_string(&resolved)?;
        let profile: CalibrationProfile = serde_json::from_str(&content)?;
        Ok(profile)
    }

    pub fn list_profiles(&self) -> Vec<CalibrationProfile> {
        let mut results = Vec::new();
        let Ok(entries) = fs::read_dir(&self.profiles_dir) else {
            return results;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(raw) = fs::read_to_string(&path)
                && let Ok(profile) = serde_json::from_str::<CalibrationProfile>(&raw)
            {
                results.push(profile);
            }
        }
        results.sort_by_key(|a| std::cmp::Reverse(a.timestamp));
        results
    }

    pub fn delete_profile(&self, name: &str) -> Result<()> {
        let path = self.profile_path(name);
        if path.exists() {
            fs::remove_file(&path)?;
            record_log(
                LogLevel::Info,
                "profiles",
                format!("Deleted profile '{name}'"),
            );
        }
        Ok(())
    }

    pub fn create_backup(
        &self,
        reason: &str,
        device_id: &str,
        device_name: &str,
        previous_profile: Option<CalibrationProfile>,
        previous_x11_matrix: Option<[f64; 9]>,
    ) -> Result<BackupRecord> {
        fs::create_dir_all(&self.backups_dir)?;
        let now = Utc::now();
        let backup_id = format!("backup-{}", now.format("%Y%m%dT%H%M%SZ"));
        let record = BackupRecord {
            backup_id: backup_id.clone(),
            created_at: now,
            reason: reason.to_string(),
            previous_profile,
            previous_x11_matrix,
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
        };
        let path = self.backups_dir.join(format!("{backup_id}.json"));
        fs::write(&path, serde_json::to_string_pretty(&record)?)?;
        record_log(
            LogLevel::Info,
            "backup",
            format!(
                "Created calibration backup '{}' for '{}'",
                backup_id, device_name
            ),
        );
        Ok(record)
    }

    pub fn list_backups(&self) -> Vec<BackupRecord> {
        let mut items = Vec::new();
        let Ok(entries) = fs::read_dir(&self.backups_dir) else {
            return items;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(raw) = fs::read_to_string(&path)
                && let Ok(rec) = serde_json::from_str::<BackupRecord>(&raw)
            {
                items.push(rec);
            }
        }
        items.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        items
    }

    pub fn latest_backup(&self) -> Option<BackupRecord> {
        self.list_backups().into_iter().next()
    }

    /// Finds the best valid matching profile for a connected device and display.
    pub fn find_matching_profile(
        &self,
        device: &InputDeviceInfo,
        display: Option<&DisplayInfo>,
    ) -> Option<CalibrationProfile> {
        let profiles = self.list_profiles();
        profiles.into_iter().find(|p| {
            let same_vid_pid = !device.vendor_id.is_empty()
                && p.vendor_id.eq_ignore_ascii_case(&device.vendor_id)
                && p.product_id.eq_ignore_ascii_case(&device.product_id);
            let same_name = p.device_name.eq_ignore_ascii_case(&device.name);
            let device_matches = same_vid_pid || same_name || p.device_id == device.id;

            let display_matches = match display {
                Some(d) => {
                    (p.display_id == d.id || p.display == d.name)
                        && p.resolution == [d.width, d.height]
                }
                None => true,
            };
            device_matches && display_matches
        })
    }

    pub fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn load_settings(&self) -> AppSettings {
        let path = self.settings_path();
        if let Ok(raw) = fs::read_to_string(path)
            && let Ok(s) = serde_json::from_str::<AppSettings>(&raw)
        {
            return s;
        }
        AppSettings::default()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        fs::create_dir_all(&self.config_dir)?;
        fs::write(
            self.settings_path(),
            serde_json::to_string_pretty(settings)?,
        )?;
        Ok(())
    }

    pub fn autostart_desktop_path() -> Option<PathBuf> {
        BaseDirs::new().map(|b| {
            b.config_dir()
                .join("autostart")
                .join("smartcal-autostart.desktop")
        })
    }

    pub fn is_autostart_enabled() -> bool {
        Self::autostart_desktop_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Creates or removes the XDG autostart `.desktop` entry for `smartcal --apply-startup`.
    pub fn set_autostart_enabled(&self, enabled: bool, executable_path: &str) -> Result<()> {
        let Some(path) = Self::autostart_desktop_path() else {
            return Err(SmartCalError::Profile(
                "Could not determine XDG autostart directory".into(),
            ));
        };
        if enabled {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=SmartCal Calibration Applier\n\
                 Comment=Automatically apply Smart Board / Interactive Projector calibration on startup\n\
                 Exec={executable_path} --apply-startup\n\
                 Terminal=false\n\
                 X-GNOME-Autostart-enabled=true\n\
                 StartupNotify=false\n"
            );
            fs::write(&path, content)?;
            record_log(
                LogLevel::Info,
                "autostart",
                format!("Enabled startup auto-apply at {}", path.display()),
            );
        } else if path.exists() {
            fs::remove_file(&path)?;
            record_log(LogLevel::Info, "autostart", "Disabled startup auto-apply");
        }
        Ok(())
    }
}
