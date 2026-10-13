# Testing

This page describes how iso-flasher is tested and how to run the tests yourself.

## Running the tests

```bash
cargo test
```

## What is covered

`cargo test` plus manual testing with non-critical devices and images.

## Adding tests

- Add tests alongside the code they cover.
- Prefer testing behavior over implementation details.
- Keep fixtures free of real personal data, keys, or credentials.

## Current status

Automated test coverage is a work in progress. Contributions that add tests are
welcome.
