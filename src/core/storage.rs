use std::collections::HashMap;

use anyhow::Context;
use jmap_client::{email::Email, mailbox::Mailbox};
use opendal::{layers::RetryLayer, Operator, Scheme};

/**
 * Create a storage backend with the given configuration.
 * Exit the process if the backend cannot be created.
 * Handle exit here to avoid having to handle anyhow::Result in main 
 */
pub fn create_storage_backend(scheme: Scheme, config: HashMap<String, String>) -> anyhow::Result<Operator> {
    let operator = Operator::via_map(scheme, config);

    let retry_operator = operator
        .with_context(|| "Error creating storage backend")?
        .layer(RetryLayer::new()); // Apply retry layer to avoid transient errors

    Ok(retry_operator)
}

pub async fn get_email_from_storage(operator: &Operator, id: &str) -> anyhow::Result<Email> {
    let path = format!("/emails/{}/{}.json", &id[..3], id);
    let json = operator
        .read(&path)
        .await
        .with_context(|| format!("Error reading email {}", id))?;

    let email = serde_json::from_slice::<Email>(&json)
        .with_context(|| format!("Error deserializing email {}", id))?;

    Ok(email)
}

pub async fn get_mailbox_from_storage(operator: &Operator, id: &str) -> anyhow::Result<Mailbox> {
    let path = format!("/mailboxes/{}.json", id);
    let json = operator
        .read(&path)
        .await
        .with_context(|| format!("Error reading mailbox {}", id))?;

    let mailbox = serde_json::from_slice::<Mailbox>(&json)
        .with_context(|| format!("Error deserializing mailbox {}", id))?;

    Ok(mailbox)
}