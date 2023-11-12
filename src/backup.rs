use futures::stream;
use console::style;
use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use jmap_client::{mailbox::Mailbox, client::Client, email::Email};
use opendal::Operator;
use super::helpers;



pub async fn backup(client: Client, operator: Operator) -> Result<(), Box<dyn std::error::Error>> {
    let max_objects = helpers::max_objects_in_get(&client);
    let progress = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");

    let pb_mailboxes = progress.add(ProgressBar::new(0));
    let pb_emails = progress.add(ProgressBar::new(0));
    // Set style of all progress bars
    pb_mailboxes.set_style(sty.clone());
    pb_emails.set_style(sty.clone());

    // Process mailboxes
    let processed_mailboxes = mailboxes(&client, &operator, max_objects, &pb_mailboxes).await?;
    pb_mailboxes.finish_with_message("Mailboxes done");

    // Process emails
    let processed_emails = emails(&client, &operator, max_objects, &pb_emails).await?;
    pb_emails.finish_with_message("Emails done");
    

    // Print mailboxes
    println!("{} {} mailboxes", style("Found").green(), style(processed_mailboxes).green());
    println!("{} {} emails", style("Found").green(), style(processed_emails).green());


    Ok(())
}

// Returns Result void or error
async fn mailboxes(client: &Client, operator: &Operator, max_objects: usize, pb: &ProgressBar) -> Result<usize, Box<dyn std::error::Error>> {
    let mut position: usize = 0;

    loop {
        let (total, mut mailboxes_res) = fetch_mailboxes(position, max_objects, &client).await?;;
        let length = mailboxes_res.len();
        position += length;

        pb.set_length(total.try_into().unwrap());

        // Iterate over mailboxes and write to storage backend
        for mailbox in mailboxes_res {
            let id = mailbox.id().unwrap();
            let path = format!("/mailboxes/{}/{}.json", &id[..2], id);
            // Ensure static lifetime for the serialized JSON
            let json = serde_json::to_string(&mailbox).unwrap();
            operator.write(&path, json).await.unwrap_or_default();
        }

        pb.inc(length.try_into().unwrap());
        
        if position >= total {
            break;
        }
    };

    Ok(position)
}

async fn fetch_mailboxes(position: usize, max_objects: usize, client: &Client) -> Result<(usize, Vec<Mailbox>), jmap_client::Error> {
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
            let total = total_res.unwrap_query_mailbox()?.total().unwrap_or_default();
            let mailboxes = mailboxes_res.unwrap_get_mailbox()?.take_list();
            Ok((total, mailboxes))
        },
        _ => Err("unexpected number of responses".into()),
    }
}


async fn emails(client: &Client, operator: &Operator, max_objects: usize, pb: &ProgressBar) -> Result<usize, jmap_client::Error> {
    let mut position: usize = 0;

    loop {
        let (total, mut emails_res) = fetch_email(position, max_objects, &client).await?;;
        let length = emails_res.len();
        position += length;
        
        pb.set_length(total.try_into().unwrap());

        // Iterate over emails and write to storage backend
        for email in emails_res {
            let id = email.id().unwrap();
            let path = format!("/emails/{}/{}.json", &id[..3], id);
            // Ensure static lifetime for the serialized JSON
            let json = serde_json::to_string(&email).unwrap();
            operator.write(&path, json).await.unwrap_or_default();
        }

        pb.inc(length.try_into().unwrap());
        
        if position >= total {
            break;
        }
    }
    
    Ok(position)
}

async fn fetch_email(position: usize, max_objects: usize, client: &Client) -> Result<(usize, Vec<Email>), jmap_client::Error> {
    let mut request = client.build();
    let result = request
        .query_email()
        .calculate_total(true)
        .position(position.try_into().unwrap())
        .limit(max_objects)
        .result_reference();
    
    request.get_email().ids_ref(result);

    let mut response = request.send().await?.unwrap_method_responses();
    let email_res = response.pop();
    let total_res = response.pop();

    match (total_res, email_res) {
        // Match Vec of two TaggedMethodResponse
        (Some(total_res), Some(email_res)) => {
            let total = total_res.unwrap_query_email()?.total().unwrap_or_default();
            let emails = email_res.unwrap_get_email()?.take_list();
            Ok((total, emails))
        },
        _ => Err("unexpected number of responses".try_into().unwrap()),
    }
}

        