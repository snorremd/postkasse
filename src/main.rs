use std::collections::HashMap;

use anyhow::Context;
use clap::Parser;
use conf::AuthMode;
use console::style;
use jmap_client::client::{Client, Credentials};


mod cli;
mod helpers;
mod conf;
use cli::{Cli, Commands};

mod backup;
use backup::backup;
use opendal::{Scheme, Operator, layers::RetryLayer};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    println!("Welcome to {}!", style("Brevkasse").red().bold());

    let mut conf = conf::Conf::new(&cli).unwrap_or_else(|e| {
        let err = format!("Error reading config file {}", e);
        eprintln!("{}", style(err).red().bold());
        std::process::exit(1);
    });

    match cli.command {
        Some(Commands::Backup {}) => {
            // We need to configure the jmap client and operator for backup to work
            conf.set_jmap_secret()?;
            conf.set_storage_secret()?;
            
            let client = create_client(conf.jmap).await?;
            let operator = create_storage_backend(conf.storage.scheme.into(), conf.storage.config);
            
            return backup(client, operator).await.map_err(|e| {
                let err = format!("Error backing up {}. {}", conf.name, e);
                eprintln!("{}", style(err).red().bold());
                std::process::exit(1);
            });
        }
        Some(Commands::Status {}) => {
            return Ok(());
        }
        None => {}
    }

    Ok(())

}

// Allow any config map to be passed in
fn create_storage_backend(scheme: Scheme, config: HashMap<String, String>) -> Operator {
    
    let operator = Operator::via_map(scheme, config);
    
    operator.unwrap_or_else(|e| {
        let err = format!("Error creating storage backend. {}", e);
        eprintln!("{}", style(err).red().bold());
        std::process::exit(1);
    }).layer(RetryLayer::new()) // Apply retry layer to avoid transient errors
}

async fn create_client(jmap_conf: conf::Jmap) -> anyhow::Result<Client> {
    let username = jmap_conf.username.unwrap_or_default();
    let secret = jmap_conf.secret.with_context(|| {
        format!("Error getting secret from config")
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
            format!(
                "Error connecting to JMAP server {} with user {}",
                jmap_conf.host,
                username
            )
        })?;

    Ok(client)

}