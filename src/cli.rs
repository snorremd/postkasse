use std::path::PathBuf;
use clap::{Parser, Subcommand, arg};

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
    Backup {},

    /// Show the status of the backup, i.e. what was the last message backed up
    Status {},

    /// Search emails
    Search {
        /// Search query
        query: String,

        /// Which fields to show in the search results
        /// Default is id and subject
        #[arg(short, long, value_parser, num_args = 1.., value_delimiter = ' ', default_value = "id subject")]
        fields: Option<Vec<String>>,
        
        /// Limit the number of results
        #[arg(short, long, default_value = "100")]
        limit: Option<usize>,
    },

    Open {
        /// Show the email with the given id
        id: String,
    }

}


