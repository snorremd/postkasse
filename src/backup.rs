mod progress;
mod email;
mod mailboxes;

use super::helpers;
use anyhow::Result;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use jmap_client::client::Client;
use opendal::Operator;
use serde::{Deserialize, Serialize};




pub async fn backup(client: Client, operator: Operator) -> Result<(), Box<dyn std::error::Error>> {
    let max_objects = helpers::max_objects_in_get(&client);
    let progress = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "{msg:10} {bar:40.cyan/blue} {pos:>7}/{len:7} {elapsed_precise}/{eta_precise} ",
    )
    .unwrap()
    .progress_chars("##-");

    let pb_mailboxes = progress.add(ProgressBar::new(0));
    let pb_emails = progress.add(ProgressBar::new(0));
    // Set style of all progress bars
    pb_mailboxes.set_style(sty.clone());
    pb_mailboxes.set_message("Mailboxes:");
    pb_emails.set_style(sty.clone());
    pb_emails.set_message("Emails:");
    

    // Process mailboxes
    let processed_mailboxes = mailboxes::mailboxes(&client, &operator, max_objects, &pb_mailboxes).await?;

    // Process emails
    let processed_emails = email::emails(&client, &operator, max_objects, &pb_emails).await?;


    // Print mailboxes
    println!(
        "{} {} mailboxes",
        style("Found").green(),
        style(processed_mailboxes).green()
    );
    println!(
        "{} {} emails",
        style("Found").green(),
        style(processed_emails).green()
    );

    Ok(())
}


