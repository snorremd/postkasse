use anyhow::Context;
use jmap_client::{client::{Client, Credentials}, mailbox::Mailbox};

use crate::conf::{self, AuthMode};

/**
 * Create a JMAP client with the given configuration
 * Return error if the client cannot be created
 */
pub async fn create_client(jmap_conf: conf::Jmap) -> anyhow::Result<Client> {
    let username = jmap_conf.username.unwrap_or_default();
    let secret = jmap_conf
        .secret
        .with_context(|| {
            "No secret found for JMAP client"
        })?;

    let credentials = match jmap_conf.auth_mode {
        AuthMode::Basic => Credentials::basic(&username, &secret),
        AuthMode::Token => Credentials::bearer(&secret),
    };

    let client: Client = Client::new()
        .credentials(credentials)
        // Takes iterator of hosts to trust
        .follow_redirects(["api.fastmail.com"])
        .connect(&jmap_conf.host)
        .await
        .with_context(|| {
            "Error creating JMAP client."
        })?;

    Ok(client)
}


/// A helper function to fetch mailboxes from the JMAP server
/// Returns as many mailboxes as max_objects or less
pub async fn fetch_mailboxes(
    position: usize,
    max_objects: usize,
    client: &Client,
) -> anyhow::Result<Vec<Mailbox>> {
    let mut request = client.build();
    let result = request
        .query_mailbox()
        .position(position.try_into().unwrap())
        .limit(max_objects)
        .result_reference();

    request.get_mailbox().ids_ref(result);

    let mut response = request.send().await?.unwrap_method_responses();
    let mailboxes_res = response.pop();

    match mailboxes_res {
        // Match Vec of two TaggedMethodResponse
        Some(mailboxes_res) => {
            let mailboxes = mailboxes_res.unwrap_get_mailbox()?.take_list();
            Ok(mailboxes)
        }
        _ => anyhow::bail!("unexpected number of responses"),
    }
}

/// A helper function that creates a mailbox on the JMAP server
pub async fn create_mailboxes(
    client: &Client,
    mailboxes: Vec<Mailbox>,
) -> anyhow::Result<()> {
    let mut request = client.build();
    let set_request = request.set_mailbox();

    for mailbox in mailboxes {
        set_request
            .create()
            .name(mailbox.name().unwrap_or_default())
            .role(mailbox.role())
            .parent_id(mailbox.parent_id());
    }

    let response = request
        .send_set_mailbox()
        .await?
        .unwrap_create_errors()
        .with_context(|| "Error creating mailboxes")?;

    Ok(())
} 