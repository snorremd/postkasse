// Struct to keep track of the progress of the emails being backed up so we can store and resume next time
// Should be JSON serializable and deserializable using serde
use anyhow::Context;
use opendal::Operator;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupProgress {
    pub position: usize,
    // List of item (e.g. email) ids that have been backed up so far
    pub items: Vec<String>,
}

pub async fn read_backup_progress(operator: &Operator, file: &str) -> anyhow::Result<BackupProgress> {
    let path = format!("/progress/{}", file);
    let exists = operator.is_exist(&path).await.with_context(|| {
        format!("Error checking if backup progress exists")
    })?;

    if !exists {
        return Ok(BackupProgress {
            position: 0,
            items: Vec::new(),
        });
    }

    let progress = operator.read(&path).await.with_context(|| {
        format!("Error reading backup progress")
    })?;

    let backup_progress: BackupProgress = serde_json::from_slice(&progress).with_context(|| {
        format!("Error deserializing backup progress")
    })?;

    Ok(backup_progress)
}

pub async fn write_backup_progress(
    operator: &Operator,
    file: &str,
    backup_progress: &BackupProgress,
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