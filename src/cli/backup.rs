use anyhow::Result;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use jmap_client::client::Client;
use log::info;
use opendal::Operator;
use tantivy::IndexWriter;

use crate::core::email::backup_emails;
use crate::core::mailboxes::mailboxes;
use crate::core::helpers;
use crate::core::progress::Progressable;

/// Implement the progressable trait for ProgressBar
/// This way we can use progress bar to track progress
/// without the core depending on the ProgressBar crate
impl Progressable for ProgressBar {

    fn inc(&self, amount: u64) {
        self.inc(amount);
    }

    fn position(&self) -> u64 {
        self.position()
    }

    fn set_position(&self, position: u64) {
        self.set_position(position);
    }
    
    fn set_length(&self, total: u64) {
        self.set_length(total);
    }
}

pub async fn backup(client: Client, operator: Operator, multi: MultiProgress, indexer: Option<IndexWriter>) -> Result<(), Box<dyn std::error::Error>> {
    let max_objects = helpers::max_objects_in_get(&client);
    let progress = multi;
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
    mailboxes(&client, &operator, max_objects, &pb_mailboxes).await?;

    // Process emails
    backup_emails(&client, &operator, max_objects, &pb_emails, indexer).await?;


    // Print mailboxes
    info!(
        "{} {} mailboxes",
        style("Found").green(),
        style(pb_mailboxes.position()).green()
    );
    info!(
        "{} {} emails",
        style("Found").green(),
        style(pb_emails.position()).green()
    );

    Ok(())
}


