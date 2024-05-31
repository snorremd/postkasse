#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, env, path::PathBuf};

use anyhow::Context;
use clap::Parser;
use conf::AuthMode;
use console::style;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use jmap_client::client::{Client, Credentials};

mod core;
mod cli;
mod conf;
mod helpers;
use cli::{Cli, Commands};

mod backup;
use backup::backup;
mod search;
use log::{error, info};
use opendal::{layers::RetryLayer, Operator, Scheme};
use search::search_emails;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let multi = MultiProgress::new();
    // Log setup to avoid clashes with indicatif
    let logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error,postkasse=error")).build();

    LogWrapper::new(multi.clone(), logger).try_init().unwrap();

    info!("Welcome to {}!", style("Postkasse").red().bold());

    let mut conf = conf::Conf::new(&cli).unwrap_or_else(|e| {
        let err = format!("Error reading config file {}", e);
        error!("{}", style(err).red().bold());
        std::process::exit(1);
    });
    
    match cli.command {
        Some(Commands::Backup {}) => {
            // We need to configure the jmap client and operator for backup to work
            conf.set_jmap_secret()?;
            conf.set_storage_secret()?;

            let client = create_client(conf.jmap).await;
            let operator = create_storage_backend(conf.storage.scheme.into(), conf.storage.config);
            let indexer = conf.search.map(|s| {
                if s.enable {
                    Some(search::create_indexer(s.folder).unwrap_or_else(|e| {
                        let err = format!("Error creating indexer. {}", e);
                        error!("{}", style(err).red().bold());
                        std::process::exit(1); // Bail out if indexer cannot be created
                    }))
                } else {
                    None
                }
            }).unwrap_or_default();

            return backup(client, operator, multi, indexer).await.map_err(|e| {
                let err = format!("Error backing up {}. {}", conf.name, e);
                error!("{}", style(err).red().bold());
                std::process::exit(1);
            })
        }
        Some(Commands::Status {}) => {
            return Ok(());
        }
        Some(Commands::Search { query, fields, limit }) => {
            if let Some(search) = conf.search {
                search_emails(search, query, limit, fields);
            } else {
                let err = format!("Search is not enabled in config");
                error!("{}", style(err).red().bold());
                std::process::exit(1);
            }

            Ok(())
        }
        Some(Commands::Open { id }) => {
            conf.set_storage_secret()?;
            let operator = create_storage_backend(conf.storage.scheme.into(), conf.storage.config);
            let blob_path = &format!("/blobs/{}/{}", &id[..2], id);
            let temp_dir: PathBuf = env::temp_dir();
            let temp_file_path = temp_dir.join(format!("{}.eml", id));

            let blob = operator.read(blob_path).await?;
            std::fs::write(&temp_file_path, blob).with_context(|| {
                format!("Error writing blob to file {}", temp_file_path.display())
            })?;

            info!("Email saved to {}", temp_file_path.display());
            
            open::that(temp_file_path)?;

            Ok(())
        }
        None => {
            return Ok(());
        }
    }
}

/**
 * Create a storage backend with the given configuration.
 * Exit the process if the backend cannot be created.
 * Handle exit here to avoid having to handle anyhow::Result in main 
 */
fn create_storage_backend(scheme: Scheme, config: HashMap<String, String>) -> Operator {
    let operator = Operator::via_map(scheme, config);

    operator
        .unwrap_or_else(|e| {
            let err = format!("Error creating storage backend. {}", e);
            error!("{}", style(err).red().bold());
            std::process::exit(1);
        })
        .layer(RetryLayer::new()) // Apply retry layer to avoid transient errors
}

/**
 * Create a JMAP client with the given configuration
 * Exit the process if the client cannot be created
 * Handle exit here to avoid having to handle anyhow::Result in main
 */
async fn create_client(jmap_conf: conf::Jmap) -> Client {
    let username = jmap_conf.username.unwrap_or_default();
    let secret = jmap_conf
        .secret
        .unwrap_or_else(|| {
            let err = format!("No secret found for JMAP client");
            error!("{}", style(err).red().bold());
            std::process::exit(1);
        });

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
        .unwrap_or_else(|e| {
            let err = format!("Error creating JMAP client. {}", e);
            error!("{}", style(err).red().bold());
            std::process::exit(1);
        });

    client
}
