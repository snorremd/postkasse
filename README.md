# Postkasse

<!-- Insert nice HTML with warning about work in progress status -->

<div style="color: black; background-color: orange; padding: 1rem; border-radius: 0.5rem; margin-bottom: 1rem;">
    <h2>Work in progress</h2>
    <p>
        Postkasse is still in early development and is not ready for use.
        It is not feature complete and may contain bugs.
        Use at your own risk.
    </p>
</div>

A simple CLI tool to take incremental backups of your email.
Custom built for providers using JMAP protocol to enable fast and efficient backups.
Postkasse will store your emails locally or in a storage provider of your choice.
`Postkasse` is the Norwegian word for mailbox, but it's also a play on the word `kasse` which also translates to crate.
Thus the name Postkasse, a crate to keep your letters in.

## Features

### Incremental Backups

Postkasse will only download new mailboxes and emails since the last backup.
The backups are immutable in the sense that emails deleted on the server will not be deleted in the backup.
This is to ensure that you can always restore your emails from a backup.

### Supports multiple storage providers

Postkasse supports multiple storage providers, including local filesystem, s3, Google Drive, Dropbox and more via [OpenDAL](https://opendal.apache.org/).
Any storage provider supporting reads, writes should be supported.


### Local indexing and search

Postkasse will index your emails locally and allow you to search through them using [Tantivy](https://github.com/quickwit-oss/tantivy).
This allows for fast and efficient search of your email archive so you can find that one email you're looking for.


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

## License

[MIT](https://choosealicense.com/licenses/mit/)