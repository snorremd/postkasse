use std::{path::PathBuf, collections::HashMap};

use clap::Parser;
use console::{Term, style};
use dialoguer::Password;
use jmap_client::{client::{Client, Credentials}, mailbox::{query::Filter, Role}};
use keyring::{Entry, Result};

mod cli;
mod helpers;
use cli::{Cli, Commands, AuthMode};

mod backup;
use backup::backup;
use opendal::{Scheme, Operator};


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    println!("Welcome to {}!", style("Brevkasse").red().bold());

    match cli.command {
        Some(Commands::Backup { username, hostname, prompt_password, auth_mode, storage }) => {
            let client = create_client(&hostname, &username, prompt_password, auth_mode).await;
            let mut operator_config = HashMap::new();
            operator_config.insert("root".to_string(), "./data".to_string());
            let operator = create_storage_backend(storage, operator_config);
            return backup(client, operator).await.map_err(|e| {
                let err = format!("Error backing up {}. {}", username, e);
                eprintln!("{}", style(err).red().bold());
                std::process::exit(1);
            });
        }
        Some(Commands::Status { username, hostname, prompt_password, auth_mode}) => {
            let client = create_client(&hostname, &username, prompt_password, auth_mode).await;
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
    })
}

async fn create_client(url: &str, username: &str, prompt_password: bool, auth_mode: AuthMode) -> Client {

    let keyring_entry = Entry::new("brevkasse", username).unwrap_or_else(|e| {
        // No keyring entry found, create one
        let err = format!("Error creating keyring entry for {}. {}", username, e);
        eprintln!("{}", style(err).red().bold());
        std::process::exit(1);
    });

    let password: String;

    // Prompt if password is not set or user specifies to prompt for password
    match (prompt_password, keyring_entry.get_password()) {
        (true, _) | (false, Err(_)) => {
            // Prompt for password
            password = Password::new()
                .with_prompt("Enter your password or token")
                .interact()
                .unwrap_or_else(|e| {
                    let err = format!("Error reading password from prompt {}", e);
                    eprintln!("{}", style(err).red().bold());
                    std::process::exit(1);
                });
            
            keyring_entry.set_password(&password).unwrap_or_else(|e| {
                let err = format!("Error setting password for {} in keyring. {}", username, e);
                eprintln!("{}", style(err).red().bold());
                std::process::exit(1);
            });
        }
        (false, Ok(_)) => {
            // Password is set and user does not specify to prompt for password
            password = keyring_entry.get_password().unwrap_or_else(
                |e| {
                    let err = format!("Error getting password for {} from keyring. {}", username, e);
                    eprintln!("{}", style(err).red().bold());
                    std::process::exit(1);
                }
            
            );
        }
    }

    let credentials = match auth_mode {
        AuthMode::Basic => Credentials::basic(username, &password),
        AuthMode::Token => Credentials::bearer(password),
    };

    let client: Client = Client::new()
        .credentials(credentials)
        // Takes iterator of hosts to trust
        .follow_redirects(["api.fastmail.com"])
        .connect(url)
        .await
        .unwrap_or_else(|e| {
            let err = format!("Error connecting to JMAP server {} with {}. {}", url, username, e);
            eprintln!("{}", style(err).red().bold());
            std::process::exit(1);
        });

    client

}