use std::collections::HashSet;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::{stream, StreamExt, TryStreamExt};
use jmap_client::{
    client::Client,
    core::query::Filter,
    email::{self, Property},
    mailbox,
};
use log::info;
use mail_parser::MessageParser;
use opendal::Operator;
use rayon::prelude::*;
use tantivy::IndexWriter;

use super::{
    helpers::sort_mailboxes,
    progress::{read_backup_progress, write_backup_progress, Progressable},
    search::write_document, storage::get_mailbox_from_storage,
};

pub async fn backup_emails(
    client: &Client,
    operator: &Operator,
    max_objects: usize,
    pb: &dyn Progressable,
    mut indexer: Option<IndexWriter>,
) -> Result<()> {
    info!("Backing up emails");
    let message_parser = MessageParser::default();
    let mut backup_progress = read_backup_progress(operator, "email.json")
        .await
        .with_context(|| format!("Error reading backup progress"))?;

    let total = fetch_total_count(&client, backup_progress.last_processed_date)
        .await
        .with_context(|| format!("Error fetching total count"))?;

    pb.set_length(total.try_into().unwrap());

    loop {
        let emails_res = fetch_email(
            &client,
            backup_progress.last_processed_date,
            pb.position().try_into().unwrap(),
            max_objects,
        )
        .await
        .with_context(|| format!("Error fetching emails from position {}", pb.position()))?;

        let length = emails_res.len();

        stream::iter(emails_res.iter().map(|email| backup_email(email, operator)))
            .buffer_unordered(50)
            .collect::<Vec<_>>()
            .await;

        let blobs = stream::iter(emails_res.iter().map(|id| {
            let blob_id = id.blob_id().unwrap(); // Should always be present in working JMAP implementations
            backup_blob(blob_id, &client, &operator)
        }))
        .buffered(50)
        .collect::<Vec<_>>()
        .await;

        // Update backup progress
        // Get the unwrapped received_at of the last email
        let last_received = emails_res
            .last()
            .map(|email| email.received_at())
            .flatten()
            .map(|date| DateTime::from_timestamp_millis(date * 1000))
            .flatten();

        // Borrow indexer mutably if it exists and write email documents then commit
        if let Some(indexer) = &mut indexer {
            // Index the emails using parallel processing
            index_emails(emails_res, blobs, &message_parser, indexer)?;
        }

        backup_progress.last_processed_date = last_received.unwrap_or_default();

        info!("Writing backup progress");
        write_backup_progress(operator, "email.json", backup_progress)
            .await
            .with_context(|| format!("Error writing backup progress"))?;

        pb.inc(length.try_into().unwrap());

        info!("Processed {} emails", pb.position());

        if pb.position() >= total.try_into().unwrap() {
            break;
        }
    }

    Ok(())
}

fn index_emails(
    emails_res: Vec<email::Email>,
    blobs: Vec<std::prelude::v1::Result<Vec<u8>, anyhow::Error>>,
    message_parser: &MessageParser,
    indexer: &mut IndexWriter,
) -> Result<(), anyhow::Error> {
    let combined = emails_res
        .into_iter()
        .zip(blobs.into_iter())
        .collect::<Vec<_>>();

    combined.par_iter().for_each(|(email, blob)| {
        let _ = blob
            .as_ref()
            .map(|blob| message_parser.parse(blob))
            .map(|message| write_document(indexer, email, &message.unwrap_or_default()));
    });
    indexer
        .commit()
        .with_context(|| format!("Error committing indexer"))?;
    Ok(())
}

async fn backup_blob(
    blob_id: &str,
    client: &Client,
    operator: &Operator,
) -> anyhow::Result<Vec<u8>> {
    let blob_path = format!("/blobs/{}/{}", &blob_id[..2], blob_id);
    let blob = client
        .download(blob_id)
        .await
        .with_context(|| format!("Error downloading blob {}", blob_id))?;

    // Parse the blob to get the email in structured format

    operator
        .write(&blob_path, blob.clone())
        .await
        .with_context(|| format!("Error writing blob {}", blob_path))?;

    Ok(blob)
}

async fn backup_email(email: &email::Email, operator: &Operator) -> anyhow::Result<()> {
    let id = email.id().unwrap();
    // Split the emails into folders based on the first three characters of the id
    // Based on the assumption that the ids are random enough to be evenly distributed
    // Fastmail uses same initial character for all emails, so we use the first 3 characters
    // Worst case scenario is that we have 16^3 = 4096 folders
    let path = format!("/emails/{}/{}.json", &id[..3], id);
    let email_json =
        serde_json::to_string(&email).with_context(|| format!("Error serializing email {}", id))?;

    // Unwrap the result of the write operation, or return a custom error message
    let _ = operator
        .write(&path, email_json)
        .await
        .with_context(|| format!("Error writing email {}", id));

    Ok(())
}

async fn fetch_total_count(
    client: &Client,
    last_processed_date: DateTime<Utc>,
) -> anyhow::Result<usize> {
    let mut request = client.build();
    request
        .query_email()
        .filter(Filter::and([email::query::Filter::after(
            last_processed_date.timestamp(),
        )]))
        .calculate_total(true)
        .result_reference();

    let mut response = request.send().await?.unwrap_method_responses();
    let total_res = response.pop();

    match total_res {
        Some(total_res) => {
            let total = total_res.unwrap_query_email()?.total().unwrap_or_default();
            Ok(total)
        }
        _ => anyhow::bail!("unexpected number of responses"),
    }
}

async fn fetch_email(
    client: &Client,
    last_processed_date: DateTime<Utc>,
    position: usize,
    max_objects: usize,
) -> anyhow::Result<Vec<email::Email>> {
    info!("Fetching emails from position {}", position);
    let mut request = client.build();
    let result = request
        .query_email()
        .filter(Filter::and([email::query::Filter::after(
            last_processed_date.timestamp(),
        )]))
        .sort(vec![
            email::query::Comparator::received_at().is_ascending(true)
        ])
        .position(position.try_into().unwrap())
        .limit(max_objects)
        .result_reference();

    let properties_to_fetch = vec![
        Property::Id,
        Property::MailboxIds,
        Property::Keywords,
        Property::ReceivedAt,
        Property::BlobId,
        Property::MessageId,
        Property::From,
        Property::To,
        Property::Cc,
        Property::Subject,
    ];

    request
        .get_email()
        .ids_ref(result)
        .properties(properties_to_fetch);

    let mut response = request.send().await?.unwrap_method_responses();
    let email_res = response.pop();

    match email_res {
        // Match Vec of two TaggedMethodResponse
        Some(email_res) => {
            let emails = email_res.unwrap_get_email()?.take_list();
            Ok(emails)
        }
        _ => anyhow::bail!("unexpected number of responses"),
    }
}

/**
 * Restore emails from storage backend to JMAP server
 * First we get the emails to restore from the storage backend.
 * Then we get the mailbox ids to restore from the emails.
 * Diff the mailboxes to restore with the mailboxes on the server to get the mailboxes to create.
 * Now we get the mailboxes to restore from the storage backend.
 * Use topological sort to sort the mailboxes so that any parent mailboxes are restored first.
 * Restore the mailboxes.
 * Finally restore the emails.
 */
pub async fn restore_emails(client: &Client, operator: &Operator, ids: Vec<&str>) -> Result<()> {
    let mailboxes_on_server = super::jmap::fetch_mailboxes(0, 10000, client)
        .await
        .with_context(|| "Error fetching mailboxes".to_string())?
        .iter()
        .map(|mailbox| mailbox.id().unwrap_or_default().to_string())
        .collect::<HashSet<String>>();

    // First we get the emails to restore
    let emails = stream::iter(ids)
        .map(|id| {
            let id = id.to_string();
            async move {
                let path = format!("/emails/{}/{}.json", &id[..3], id);
                let json = operator
                    .read(&path)
                    .await
                    .with_context(|| format!("Error reading email {}", id))?;

                let email = serde_json::from_slice::<email::Email>(&json)
                    .with_context(|| format!("Error deserializing email {}", id))?;

                Ok::<jmap_client::email::Email, anyhow::Error>(email)
            }
        })
        .buffer_unordered(10) // Adjust the concurrency level as needed
        .try_collect::<Vec<_>>()
        .await?;

    // Then we get the mailbox ids to restore from the emails
    let mailbox_ids_in_emails_to_restore = emails
        .iter()
        .flat_map(|email| email.mailbox_ids())
        .map(|id| id.to_string())
        .collect::<HashSet<String>>();

    // Diff the mailboxes to restore with the mailboxes on the server to get the mailboxes to create
    let mailbox_ids_to_restore = mailbox_ids_in_emails_to_restore
        .difference(&mailboxes_on_server)
        .map(|id| id.to_string())
        .collect::<Vec<String>>();

    // Now we get the mailboxes to restore
    let mailboxes_to_restore = stream::iter(mailbox_ids_to_restore)
        .map(|id| async move {
            let mailbox = get_mailbox_from_storage(operator, &id).await?;
            Ok::<mailbox::Mailbox, anyhow::Error>(mailbox)
        })
        .buffer_unordered(10) // Adjust the concurrency level as needed
        .try_collect::<Vec<_>>()
        .await?;

    // Use topological sort to sort the mailboxes so that any parent mailboxes are restored first
    let sorted = sort_mailboxes(mailboxes_to_restore)?;

    // Restore the mailboxes
    super::jmap::create_mailboxes(client, sorted).await?;

    Ok(())
}
