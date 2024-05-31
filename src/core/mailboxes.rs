use anyhow::Context;
use futures::{stream, StreamExt};
use jmap_client::{client::Client, mailbox::Mailbox};
use opendal::Operator;

use super::{jmap::fetch_mailboxes, progress::Progressable};

pub(crate) async fn mailboxes(
    client: &Client,
    operator: &Operator,
    max_objects: usize,
    pb: &dyn Progressable,
) -> anyhow::Result<()> {

    let total = fetch_total_count(&client).await?;
    pb.set_length(total.try_into().unwrap());

    loop {
        let mailboxes_res = fetch_mailboxes(0, max_objects, &client).await?;
        let length = mailboxes_res.len();

        // Iterate with stream over mailboxes and process them
        stream::iter(
            mailboxes_res
                .iter()
                .map(|mailbox| process_mailbox(mailbox, &operator)),
        )
        .buffer_unordered(50)
        .collect::<Vec<_>>()
        .await;

        pb.inc(length.try_into().unwrap());

        // It is doubtful people will ever have more than u64 max mailboxes, so just convert usize to u64
        if pb.position() >= total.try_into().unwrap() {
            break;
        }
    }

    Ok(())
}

/**
 * Fetch total number of mailbox items to be backed up.
 * No date based filters available for mailboxes, so no filters applied.
 */
async fn fetch_total_count(
    client: &Client,
) -> anyhow::Result<usize> {
    let mut request = client.build();
    request.query_mailbox().calculate_total(true).result_reference();

    let mut response = request.send().await?.unwrap_method_responses();
    let total_res = response.pop();
    
    match total_res {
        Some(total_res) => {
            let total = total_res.unwrap_query_mailbox()?.total().unwrap_or_default();
            Ok(total)
        }
        _ => anyhow::bail!("unexpected number of responses"),
    }
}

async fn process_mailbox(mailbox: &Mailbox, operator: &Operator) -> anyhow::Result<()> {
    let id = mailbox.id().unwrap();
    let path = format!("/mailboxes/{}.json", id); // No need to split into subdirectories since we don't expect many mailboxes
    let mailbox_json = serde_json::to_string(&mailbox)
        .with_context(|| format!("Error serializing mailbox {}", id))?;

    // Unwrap the result of the write operation, or return a custom error message
    operator
        .write(&path, mailbox_json)
        .await
        .with_context(|| format!("Error writing mailbox {}", id))
}
