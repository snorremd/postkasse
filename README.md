# Postkasse

Postkasse (Norwegian translation for mail crate) is a simple CLI tool for backing up an inbox over JMAP.
It takes incremental backups of your emails and attachments, and stores them locally.
Fastmail users and users of other email providers supporting JMAP can use this tool to backup their emails.
It stores passwords and tokens in your keyring for security and convenience.

## Installation

### From source

```bash
git clone
cd brevkasse
cargo install --path .
```

### From crates.io

```bash
cargo install brevkasse
```

## Usage


```bash
brevkasse --help
```

## Configuration

TODO

## Development

You'll need Rust and Cargo installed, you can install them with [rustup](https://rustup.rs/).

```bash
git clone
cd brevkasse
cargo build
```

## License

[MIT](https://choosealicense.com/licenses/mit/)