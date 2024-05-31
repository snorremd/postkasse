use anyhow::Context;
use jmap_client::client::{Client, Credentials};

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
