# Contributing to Archguard

## Setup

Follow the quick-start steps in the [README](README.md) — install Rust 1.84+
with the `wasm32v1-none` target and the Stellar CLI — to get a working build
before making changes.

## Making changes

1. Fork the repo, create a branch off `main`
2. Make your change with tests (`cargo test --workspace --features testutils`)
3. Open a PR — CI must pass before merge
4. One approval required

## Commit style

Conventional commits: `type(scope): description`
