# Development

This page describes how to set up a development environment and work on
iso-flasher.

## Requirements

- Rust and Cargo
- Linux with permission to write to the target device

## Getting the code

```bash
git clone https://github.com/anshlabs716/iso-flasher.git
cd iso-flasher
```

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run --release -- <image.iso> <device>
```

## Test

```bash
cargo test
```

## Workflow

- Branch from `main` (see [CONTRIBUTING.md](../CONTRIBUTING.md)).
- Keep changes focused and documented.
- Update `CHANGELOG.md` for user-facing changes.
