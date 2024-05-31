// Struct to keep track of the progress of the emails being backed up so we can store and resume next time
// Should be JSON serializable and deserializable using serde
use anyhow::Context;
use opendal::Operator;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct BackupProgress {
    pub last_processed_date: DateTime<Utc>,
}

/// Trait to be implemented by any struct that needs to keep track of progress
pub trait Progressable {
    fn position(&self) -> u64;
    fn set_position(&self, position: u64);

    fn inc(&self, increment: u64) {
        self.set_position(self.position() + increment);
    }
    
    fn set_length(&self, total: u64);
}

pub async fn read_backup_progress(operator: &Operator, file: &str) -> anyhow::Result<BackupProgress> {
    let path = format!("/progress/{}", file);
    let exists = operator.is_exist(&path).await.with_context(|| {
        format!("Error checking if backup progress exists")
    })?;

    if !exists {
        return Ok(BackupProgress {
            // Email was invented in 1971, so UNIX epoch should be a safe default barring any time travel shenanigans
            last_processed_date: DateTime::UNIX_EPOCH.into(),
        });
    }

    let progress = operator.read(&path).await.with_context(|| {
        format!("Error reading backup progress")
    })?;

    let mut backup_progress: BackupProgress = serde_json::from_slice(&progress).with_context(|| {
        format!("Error deserializing backup progress")
    })?;

    // Subtract a second from the last processed date to ensure we don't miss any emails
    backup_progress.last_processed_date = backup_progress.last_processed_date - chrono::Duration::seconds(1);

    Ok(backup_progress)
}

pub async fn write_backup_progress(
    operator: &Operator,
    file: &str,
    backup_progress: BackupProgress
) -> anyhow::Result<()> {
    let path = format!("/progress/{}", file);

    // We pretty print the JSON so it can be 
    let backup_progress_json = serde_json::to_string_pretty(&backup_progress)
        .with_context(|| format!("Error serializing backup progress"))?;

    operator
        .write(&path, backup_progress_json)
        .await
        .with_context(|| format!("Error writing backup progress"))
}