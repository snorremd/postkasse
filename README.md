# Postkasse

<!-- Insert nice HTML with warning about work in progress status -->


> [!CAUTION]
> Postkasse is still in early development and is not ready for use.
> Check back later for updates!


A simple CLI tool to take incremental backups of your email.
Custom built for providers using JMAP protocol to enable fast and efficient backups.
Postkasse will store your emails locally or in a storage provider of your choice.
`Postkasse` is the Norwegian word for mailbox, but it's also a play on the word `kasse` which also translates to crate.
Thus the name Postkasse, a crate to keep your letters in.

## Features

### Incremental Backups

Postkasse will only download new emails since the last backup.
The backups are immutable in the sense that emails deleted on the server will not be deleted in the backup.
This is to ensure that you can always restore your emails from a backup.
Similarily emails moved to other mailboxes will not be moved in the backup as this would be prohibitively expensive.

### Supports multiple storage providers

Postkasse supports multiple storage providers via [OpenDAL](https://opendal.apache.org/).
Only providers that support listing, reading, and writing files are supported.
Thus certain providers like `etcd`, `FoundationDB` and others are not supported.

Currently the following storage providers are supported:

- Azblob (Azure Blob Storage)
- Azdls (Azure Data Lake Storage)
- Cos (Tencent Cloud Object Storage)
- Fs (Local filesystem)
- Ftp (File Transfer Protocol)
- Gcs (Google Cloud Storage)
- Hdfs (Hadoop Distributed File System)
- Obs (Huawei Cloud Object Storage)
- Onedrive (Microsoft OneDrive)
- Oss (Alibaba Cloud Object Storage Service)
- S3 (Amazon S3)
- Sftp (Secure File Transfer Protocol)
- Webdav (Web Distributed Authoring and Versioning)
- Webhdfs (WebHDFS - REST implementation of HDFS)


### Local indexing and search

Postkasse will index your emails locally and allow you to search through them using [Tantivy](https://github.com/quickwit-oss/tantivy).
This allows for fast and efficient search of your email archive so you can find that one email you're looking for.
Note that this feature is optional and can be disabled in the configuration.
If not enabled you can still list and read emails from the backup.


## Installation

### From source

```bash
git clone
cd postkasse
cargo install --path .
```

### From crates.io

```bash
cargo install post
```

## Usage


```bash
postkasse --help
```


## Configuration

Postkasse can be configured using a toml file.
By default it looks for a file called `postkasse.toml` in `$HOME/.config/postkasse.toml`.


```toml
# Name of the account/backup/profile, used to identify the backup
name = "personal"

[jmap] # JMAP configuration, usually only the host is needed
host = "https://jmap.fastmail.com"
auth_mode = "token" # Can be token or password

[storage]
scheme = "Fs"

[storage.config] # See OpenDAL for provider specific settings https://opendal.apache.org/
root = "/home/johndoe/postkasse"

[search]
enabled = true # Enable local indexing and search
folder = "/home/johndoe/postkasse/search" # Where to store the index
```

### Secrets, tokens, passwords, and other sensitive information

It is usually a bad idea to store secrets in plain text in configuration files.
To that end Postkasse supports keyring and password prompts to avoid storing secrets in plain text.

Order of precedence for secrets:

1. Config file or env (always takes precedence)
2. Keyring (if available)
3. Prompt


## Development

You'll need Rust and Cargo installed, you can install them with [rustup](https://rustup.rs/).

```bash
git clone
cd postkasse

# List available commands
cargo run -- --help

# Run the backup command with the example config, requires Fastmail account and token
cargo run -- backup
```

## Aknowledgements

This project is essentially glue code between three great projects without which this little CLI tool would not be possible.

- [Stalwart Labs](https://stalw.art/) with their JMAP client and JMAP parser libraries 
- [OpenDAL](https://opendal.apache.org/) for the storage provider abstraction allowing pluggable storage
- [Tantivy](https://github.com/quickwit-oss/tantivy) for the local indexing and search capabilities by [Quickwit](https://quickwit.io/)


## License

[MIT](https://choosealicense.com/licenses/mit/)