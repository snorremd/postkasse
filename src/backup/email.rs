use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::{stream, StreamExt};
use indicatif::ProgressBar;
use jmap_client::{
    client::Client,
    core::query::Filter,
    email::{self, Property},
};
use log::info;
use opendal::Operator;

use super::progress::{read_backup_progress, write_backup_progress};

pub async fn emails(
    client: &Client,
    operator: &Operator,
    max_objects: usize,
    pb: &ProgressBar,
) -> Result<()> {
    info!("Backing up emails");
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
        // Get the unwrapped received_at of the last email
        let last_received = emails_res
            .last()
            .map(|email| email.received_at())
            .flatten()
            .map(|date| DateTime::from_timestamp_millis(date * 1000))
            .flatten();

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

    match email_res {
        // Match Vec of two TaggedMethodResponse
        Some(email_res) => {
            let emails = email_res.unwrap_get_email()?.take_list();
            Ok(emails)
        }
        _ => anyhow::bail!("unexpected number of responses"),
    }
}
