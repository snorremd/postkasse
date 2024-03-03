use anyhow::Context;
use config::{Config, Environment, File};
use dialoguer::Password;
use keyring::Entry;
use serde::Deserialize;
use std::collections::HashMap;

use console::style;
use log::{info, warn};


use crate::cli::Cli;

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
pub enum Scheme {
    Azblob,
    Azdls,
    Cos,
    Fs,
    Ftp,
    Gcs,
    Hdfs,
    Obs,
    Onedrive,
    Oss,
    S3,
    Sftp,
    Webdav,
    Webhdfs,
}

// Make a converter from Scheme to opendal::Scheme
impl From<Scheme> for opendal::Scheme {
    fn from(scheme: Scheme) -> Self {
        match scheme {
            Scheme::Azblob => opendal::Scheme::Azblob,
            Scheme::Azdls => opendal::Scheme::Azdls,
            Scheme::Cos => opendal::Scheme::Cos,
            Scheme::Fs => opendal::Scheme::Fs,
            Scheme::Ftp => opendal::Scheme::Ftp,
            Scheme::Gcs => opendal::Scheme::Gcs,
            Scheme::Hdfs => opendal::Scheme::Hdfs,
            Scheme::Obs => opendal::Scheme::Obs,
            Scheme::Onedrive => opendal::Scheme::Onedrive,
            Scheme::Oss => opendal::Scheme::Oss,
            Scheme::S3 => opendal::Scheme::S3,
            Scheme::Sftp => opendal::Scheme::Sftp,
            Scheme::Webdav => opendal::Scheme::Webdav,
            Scheme::Webhdfs => opendal::Scheme::Webhdfs,
        }
    }
}

impl Into<String> for Scheme {
    fn into(self) -> String {
        // Use debug trait to format
        format!("{:?}", self)
    }
}


#[derive(Debug, Deserialize)]
pub struct Conf {
    pub name: String,
    pub jmap: Jmap,
    pub storage: Storage,
    pub search: Option<Search>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum AuthMode {
    /// Use a token for authentication,
    /// String based enum lower case in toml
    Token,
    /// Use basic authentication (username:password)
    Basic,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Jmap {
    pub host: String,
    pub auth_mode: AuthMode,
    pub username: Option<String>,
    pub secret: Option<String>, // Can be None if user does not want to store secret in config
}


#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Storage {
    pub scheme: Scheme,
    pub config: HashMap<String, String>,
}

impl Conf {
    // Read the secret from the config map, depending on the scheme
    pub fn set_storage_secret(&mut self) -> anyhow::Result<()> {

        info!("Setting secret for {:?}", self.storage.scheme);
        
        let secret_from_config = match self.storage.scheme {
            Scheme::S3 => Some(self.storage.config.get("secret_access_key").unwrap().to_string()),
            Scheme::Azblob | Scheme::Azdls => Some(self.storage.config.get("account_key").unwrap().to_string()),
            Scheme::Cos => Some(self.storage.config.get("secret_key").unwrap().to_string()),
            Scheme::Sftp | Scheme::Webdav => Some(self.storage.config.get("password").unwrap().to_string()),
            _ => return Ok(()), // No secret needed for e.g. Fs, return early
        };

        if secret_from_config.is_some() { // If we have a secret in the config, no need to prompt
            // Warn user that storing secrets in config is not recommended
            let err = format!("Storing secrets in config is not recommended. Consider using keyring instead");
            warn!("{}", style(err).yellow().bold());
            return Ok(())
        }

        let scheme: String = self.storage.scheme.try_into()?;

        let secret_from_keyring = secret_from_keyring_or_prompt(&self.name, &scheme).with_context(|| {
            format!("Error getting secret from keyring or prompt")
        })?;

        // Set the secret in the config map
        self.storage.config.insert(self.storage.scheme.into(), secret_from_keyring);

        return Ok(())
    }

    pub fn set_jmap_secret(&mut self) -> anyhow::Result<()> {
        if self.jmap.secret.is_some() { // If we have a secret in the config, no need to prompt
            let err = format!("Storing secrets in plaintext in config is not recommended. Consider using keyring instead");
            warn!("{}", style(err).yellow().bold());
            return Ok(())
        }

        let secret_from_keyring = secret_from_keyring_or_prompt(&self.name, "jmap_secret").with_context(|| {
            format!("Error getting secret from keyring or prompt")
        })?;

        // Set the secret in the config map
        self.jmap.secret = Some(secret_from_keyring);

        return Ok(())
    }
}


#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Search {
    pub index: String,
}

impl Conf {
    pub fn new(cli: &Cli) -> anyhow::Result<Self> {

        let path = match cli.config.as_ref().map(|path| path.to_str()).flatten() {
            Some(path) => path,
            None => "postkasse.toml",
        };

        let conf_builder = Config::builder()
            .add_source(File::with_name("dev.toml").required(false)) // Read dev config file if it exists
            .add_source(Environment::with_prefix("POSTKASSE").separator("__")) // Read any env vars with prefix POSTKASSE__
            .add_source(File::with_name(path).required(false)) // Read config file if it exists 
            .build()?;

        match conf_builder.try_deserialize() {
            Ok(conf) => return Ok(conf),
            Err(e) => {
                anyhow::bail!(e)
            }
        }
    }
}


fn secret_from_keyring_or_prompt(name: &str, secret_name: &str) -> anyhow::Result<String> {
    let secret_key = format!("{}_{}", name, secret_name);
    
    let keyring_entry = Entry::new("postkasse", &secret_key).with_context(|| {
        format!("Error creating keyring entry for {}", secret_key)
    })?;

    let secret = keyring_entry.get_password();

    match secret {
        Ok(secret) => return Ok(secret),
        Err(keyring::Error::NoEntry) => {
            let password = Password::new()
                .with_prompt("Enter your password or token")
                .interact()
                .with_context(|| {
                    format!("Error reading secret {} from prompt", secret_name)
                })?;
        
            keyring_entry.set_password(&password).with_context(|| {
                format!("Error setting secret for {}", secret_key)
            })?;

            return Ok(password)
        },
        Err(e) => return Err(anyhow::anyhow!(e)),
    }





}