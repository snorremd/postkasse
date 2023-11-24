use futures::{stream, StreamExt};
use indicatif::ProgressBar;
use jmap_client::{client::Client, mailbox::Mailbox};
use opendal::Operator;
use anyhow::{Result, Context};

use super::progress::{read_backup_progress, write_backup_progress};

pub(crate) async fn mailboxes(
    client: &Client,
    operator: &Operator,
    max_objects: usize,
    pb: &ProgressBar,
) -> Result<usize, Box<dyn std::error::Error>> {
    
    let mut backup_progress = read_backup_progress(operator, "mailboxes.json").await.with_context(|| {
        format!("Error reading backup progress")
    })?;

    pb.inc(backup_progress.position.try_into().with_context(|| {
        format!("Could not convert backup progress position to u64")
    })?);

    loop {
        let (total, mailboxes_res) = fetch_mailboxes(backup_progress.position, max_objects, &client).await?;
        let length = mailboxes_res.len();
        backup_progress.position += length;

        pb.set_length(total.try_into().unwrap());

        // Iterate with stream over mailboxes and process them
        stream::iter(mailboxes_res.iter().map(|mailbox| process_mailbox(mailbox, &operator)))
            .buffer_unordered(50)
            .collect::<Vec<_>>()
            .await;

        pb.inc(length.try_into().unwrap());

        backup_progress.items.extend(
            mailboxes_res
                .iter()
                .map(|email| email.id().unwrap().to_string()),
        );

        
        write_backup_progress(operator, "mailboxes.json", &backup_progress).await.with_context(|| {
            format!("Error writing backup progress")
        })?;

        if backup_progress.position >= total {
            break;
        }
    }

    Ok(backup_progress.position)
}

async fn fetch_mailboxes(
    position: usize,
    max_objects: usize,
    client: &Client,
) -> anyhow::Result<(usize, Vec<Mailbox>)> {
    let mut request = client.build();
    let result = request
        .query_mailbox()
        .calculate_total(true)
        .position(position.try_into().unwrap())
        .limit(max_objects)
        .result_reference();

    request.get_mailbox().ids_ref(result);

    let mut response = request.send().await?.unwrap_method_responses();
    let mailboxes_res = response.pop();
    let total_res = response.pop();

    match (total_res, mailboxes_res) {
        // Match Vec of two TaggedMethodResponse
        (Some(total_res), Some(mailboxes_res)) => {
            let total = total_res
                .unwrap_query_mailbox()?
                .total()
                .unwrap_or_default();
            let mailboxes = mailboxes_res.unwrap_get_mailbox()?.take_list();
            Ok((total, mailboxes))
        }
        _ => anyhow::bail!("unexpected number of responses"),
    }
}

async fn process_mailbox(mailbox: &Mailbox, operator: &Operator) -> anyhow::Result<()> {
    let id = mailbox.id().unwrap();
    let path = format!("/mailboxes/{}.json", id); // No need to split into subdirectories since we don't expect many mailboxes
    let mailbox_json =
        serde_json::to_string(&mailbox).with_context(|| format!("Error serializing mailbox {}", id))?;

    // Unwrap the result of the write operation, or return a custom error message
    operator
        .write(&path, mailbox_json)
        .await
        .with_context(|| format!("Error writing mailbox {}", id))
}