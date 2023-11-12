use std::path::PathBuf;
use opendal::Scheme;
use clap::{Parser, Subcommand, arg, ValueEnum};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Optional name to operate on
    pub name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,
}


#[derive(Subcommand)]
pub enum Commands {
    /// Backup JMAP data from a JMAP server
    Backup {  
        /// The hostname of the JMAP server
        #[arg(short('H'), long, env("BREVKASSE_HOSTNAME"))]
        hostname: String,

        /// Basic or token authentication?
        #[arg(short, long, env("BREVKASSE_AUTH_MODE"), default_value = "token")]
        auth_mode: AuthMode,

        /// Username/email identifying the user 
        /// Used as identifier in keyring to store password/token
        #[arg(short, long, env("BREVKASSE_USERNAME"))]
        username: String,

        /// Prompt for new password/token?
        #[arg(short, long, env("BREVKASSE_PROMPT_PASSWORD"), default_value_t = false)]
        prompt_password: bool,

        /// OpenDAL service to use for storage.
        /// Defaults to Fs (local filesystem), but can be set to any of the available backends.
        ///
        /// See https://opendal.apache.org/docs/category/services for a list of available backends.
        ///
        /// See https://docs.rs/opendal/latest/opendal/enum.Scheme.html for the backend schema names
        #[arg(short, long, env("BREVKASSE_STORAGE"), default_value = "Fs")]
        storage: opendal::Scheme,
    },

    /// Show the status of the backup, i.e. what was the last message backed up
    Status {
        /// The hostname of the JMAP server
        #[arg(short('H'), long, env("BREVKASSE_HOSTNAME"))]
        hostname: String,

        /// The username to use for authentication
        #[arg(short, long, env("BREVKASSE_USERNAME"))]
        username: String,

        /// Basic or token authentication?
        #[arg(short, long, env("BREVKASSE_AUTH_MODE"), default_value = "token")]
        auth_mode: AuthMode,

        /// Prompt for new password/token?
        #[arg(short, long, env("BREVKASSE_PROMPT_PASSWORD"), default_value_t = false)]
        prompt_password: bool,
    },


}


#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum AuthMode {
    /// Use a token for authentication
    Token,
    /// Use basic authentication (username:password)
    Basic,
}