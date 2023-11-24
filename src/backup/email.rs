use futures::{stream, StreamExt};
use indicatif::ProgressBar;
use jmap_client::{client::Client, email::{self, Property}};
use opendal::Operator;
use anyhow::{Result, Context};

use super::progress::{read_backup_progress, write_backup_progress};


pub async fn emails(
    client: &Client,
    operator: &Operator,
    max_objects: usize,
    pb: &ProgressBar,
) -> Result<usize> {

    let mut backup_progress = read_backup_progress(operator, "email.json").await.with_context(|| {
        format!("Error reading backup progress")
    })?;

    pb.inc(backup_progress.position.try_into().with_context(|| {
        format!("Could not convert backup progress position to u64")
    })?);

    loop {
        let (total, emails_res) = fetch_email(backup_progress.position, max_objects, &client)
            .await
            .with_context(|| {
                format!(
                    "Error fetching emails from position {}",
                    backup_progress.position
                )
            })?;

        let length = emails_res.len();
        pb.set_length(total.try_into().unwrap());

        // Type as vec of futures
        let blob_futures = stream::iter(
            emails_res
                .iter()
                .filter_map(|email| email.blob_id())
                .map(|id| process_blob(id, &client, &operator)),
        );

        let email_futures = stream::iter(
            emails_res
                .iter()
                .map(|email| process_email(email, operator)),
        );

        // Process emails and blobs in parallel
        email_futures
            .buffer_unordered(50)
            .chain(blob_futures.buffer_unordered(50))
            .collect::<Vec<_>>()
            .await;

        // Update backup progress
        backup_progress.position += length;
        backup_progress.items.extend(
            emails_res
                .iter()
                .map(|email| email.id().unwrap().to_string()),
        );

        write_backup_progress(operator, "email.json", &backup_progress).await.with_context(|| {
            format!("Error writing backup progress")
        })?;

        pb.inc(length.try_into().unwrap());

        if backup_progress.position >= total {
            break;
        }
    }

    Ok(backup_progress.position)
}

async fn process_blob(blob_id: &str, client: &Client, operator: &Operator) -> anyhow::Result<()> {
    let blob_path = format!("/blobs/{}/{}", &blob_id[..2], blob_id);
    let blob = client
        .download(blob_id)
        .await
        .with_context(|| format!("Error downloading blob {}", blob_id))?;

    // Parse the blob to get the email in structured format

    operator
        .write(&blob_path, blob)
        .await
        .with_context(|| format!("Error writing blob {}", blob_path))?;

    Ok(())
}

async fn process_email(email: &email::Email, operator: &Operator) -> anyhow::Result<()> {
    let id = email.id().unwrap();
    // Split the emails into folders based on the first three characters of the id
    // Based on the assumption that the ids are random enough to be evenly distributed
    // Fastmail uses same initial character for all emails, so we use the first 3 characters
    // Worst case scenario is that we have 16^3 = 4096 folders
    let path = format!("/emails/{}/{}.json", &id[..3], id);
    let email_json =
        serde_json::to_string(&email).with_context(|| format!("Error serializing email {}", id))?;

    // Unwrap the result of the write operation, or return a custom error message
    operator
        .write(&path, email_json)
        .await
        .with_context(|| format!("Error writing email {}", id))
}

async fn fetch_email(
    position: usize,
    max_objects: usize,
    client: &Client,
) -> anyhow::Result<(usize, Vec<email::Email>)> {
    let mut request = client.build();
    let result = request
        .query_email()
        .calculate_total(true)
        .position(position.try_into().unwrap())
        .limit(max_objects)
        .result_reference();

    request.get_email().ids_ref(result).properties([
        Property::Id,
        Property::MailboxIds,
        Property::Keywords,
        Property::ReceivedAt,
        Property::BlobId,
        Property::MessageId,
    ]);

    let mut response = request.send().await?.unwrap_method_responses();
    let email_res = response.pop();
    let total_res = response.pop();

    match (total_res, email_res) {
        // Match Vec of two TaggedMethodResponse
        (Some(total_res), Some(email_res)) => {
            let total = total_res.unwrap_query_email()?.total().unwrap_or_default();
            let emails = email_res.unwrap_get_email()?.take_list();
            Ok((total, emails))
        }
        _ => anyhow::bail!("unexpected number of responses"),
    }
}
