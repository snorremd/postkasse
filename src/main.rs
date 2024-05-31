#[macro_use]
extern crate lazy_static;

mod core;
mod conf;
mod cli;

use core::{jmap::create_client, storage::create_storage_backend};
use std::{env, path::PathBuf};
use anyhow::Context;
use clap::Parser;
use cli::{backup::backup, cli::{Cli, Commands}, search::search_emails};
use console::style;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use log::{error, info};



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

            let client = create_client(conf.jmap).await.unwrap_or_else(|e| {
                let err = format!("{}", e);
                error!("{}", style(err).red().bold());
                std::process::exit(1);
            });

            let operator = create_storage_backend(conf.storage.scheme.into(), conf.storage.config).unwrap_or_else(|e| {
                let err = format!("{}", e);
                error!("{}", style(err).red().bold());
                std::process::exit(1);
            });

            let indexer = conf.search.map(|s| {
                if s.enable {
                    Some(core::search::create_indexer(s.folder).unwrap_or_else(|e| {
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
        Some(Commands::Restore { cmd } ) => {
                        // We need to configure the jmap client and operator for backup to work
            conf.set_jmap_secret()?;
            conf.set_storage_secret()?;

            let client = create_client(conf.jmap).await.unwrap_or_else(|e| {
                let err = format!("{}", e);
                error!("{}", style(err).red().bold());
                std::process::exit(1);
            });

            let operator = create_storage_backend(conf.storage.scheme.into(), conf.storage.config).unwrap_or_else(|e| {
                let err = format!("{}", e);
                error!("{}", style(err).red().bold());
                std::process::exit(1);
            });

            // TODO: Implement subcommands for restore
            Ok(())
        }
        Some(Commands::Status {}) => {
            Ok(())
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
            let operator = create_storage_backend(conf.storage.scheme.into(), conf.storage.config).unwrap_or_else(|e| {
                let err = format!("{}", e);
                error!("{}", style(err).red().bold());
                std::process::exit(1);
            });
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

