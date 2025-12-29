// Version checking and update notifications

use crate::error::SysproxError;
use anyhow::Result;
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

/// Latest release information from GitHub API
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub published_at: String,
    pub html_url: String,
    pub body: Option<String>,
}

/// Update checker
pub struct UpdateChecker {
    current_version: String,
    repository_url: String,
    last_check_file: Option<std::path::PathBuf>,
}

impl UpdateChecker {
    /// Create a new update checker
    pub fn new(current_version: String) -> Self {
        let config_dir = dirs::config_dir().map(|dir| dir.join("sysprox"));
        let last_check_file = config_dir.map(|dir| dir.join(".last_update_check"));

        Self {
            current_version,
            repository_url: "https://api.github.com/repos/yourusername/sysprox/releases/latest".to_string(),
            last_check_file,
        }
    }

    /// Check if we should check for updates (rate limited to once per day)
    pub async fn should_check_for_updates(&self) -> bool {
        if let Some(ref file) = self.last_check_file {
            if let Ok(metadata) = tokio::fs::metadata(file).await {
                if let Ok(modified) = metadata.modified() {
                    let now = SystemTime::now();
                    let duration = now.duration_since(modified).unwrap_or(Duration::ZERO);
                    // Only check once per day
                    return duration > Duration::from_secs(24 * 60 * 60);
                }
            }
        }
        true
    }

    /// Check for updates from GitHub
    pub async fn check_for_updates(&self) -> Result<Option<GitHubRelease>> {
        let client = reqwest::Client::builder()
            .user_agent("sysprox-update-checker")
            .timeout(Duration::from_secs(10))
            .build()?;

        let response = client
            .get(&self.repository_url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API returned status: {}", response.status()));
        }

        let release: GitHubRelease = response.json().await?;

        // Compare versions (strip 'v' prefix if present)
        let latest_version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
        let current_version = self.current_version.strip_prefix('v').unwrap_or(&self.current_version);

        if self.is_newer_version(latest_version, current_version) {
            Ok(Some(release))
        } else {
            Ok(None)
        }
    }

    /// Mark that we've checked for updates
    pub async fn mark_update_checked(&self) -> Result<()> {
        if let Some(ref file) = self.last_check_file {
            if let Some(parent) = file.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(file, chrono::Utc::now().to_rfc3339()).await?;
        }
        Ok(())
    }

    /// Check if a version is newer than current
    fn is_newer_version(&self, latest: &str, current: &str) -> bool {
        // Simple semantic version comparison
        let parse_version = |v: &str| -> Vec<u32> {
            v.split('.')
                .filter_map(|part| part.parse().ok())
                .collect()
        };

        let latest_parts = parse_version(latest);
        let current_parts = parse_version(current);

        // Compare major.minor.patch
        for i in 0..3 {
            let latest_part = latest_parts.get(i).unwrap_or(&0);
            let current_part = current_parts.get(i).unwrap_or(&0);
            
            if latest_part > current_part {
                return true;
            } else if latest_part < current_part {
                return false;
            }
        }
        
        false
    }

    /// Format update notification message
    pub fn format_update_notification(&self, release: &GitHubRelease) -> String {
        format!(
            "🎉 Update available!\nCurrent: v{}\nLatest:  {} ({})\n\nDownload at: {}",
            self.current_version,
            release.tag_name,
            release.name,
            release.html_url
        )
    }
}

/// Background task to check for updates
pub async fn spawn_update_checker(current_version: String) -> tokio::sync::mpsc::UnboundedSender<Option<GitHubRelease>> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let checker = UpdateChecker::new(current_version);

    tokio::spawn(async move {
        // Wait a bit before first check
        sleep(Duration::from_secs(5)).await;

        loop {
            if checker.should_check_for_updates().await {
                match checker.check_for_updates().await {
                    Ok(Some(release)) => {
                        tx.send(Some(release)).ok();
                        checker.mark_update_checked().await.ok();
                    }
                    Ok(None) => {
                        // No update available
                        checker.mark_update_checked().await.ok();
                    }
                    Err(e) => {
                        eprintln!("Failed to check for updates: {}", e);
                    }
                }
            }

            // Check again tomorrow
            sleep(Duration::from_secs(24 * 60 * 60)).await;
        }
    });

    tx
}