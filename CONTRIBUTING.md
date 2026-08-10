# Contributing to Archguard

First off — thank you for taking the time to contribute! Archguard is an open-source project, and this document
outlines how to get set up, what the conventions are, and how to get your changes merged.

## Table of Contents

- [Code of conduct](#code-of-conduct)
- [Development setup](#development-setup)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
- [Repository layout](#repository-layout)
- [Making changes](#making-changes)
  - [Branching](#branching)
  - [Coding conventions](#coding-conventions)
  - [Testing](#testing)
- [Commit style](#commit-style)
- [Pull request process](#pull-request-process)
- [CI](#ci)
- [Security](#security)
- [Questions?](#questions)

## Code of conduct

Be respectful and constructive. This project is a collaborative effort — assume good faith, give clear feedback, and
help each other out. Harassment or abuse of any kind will not be tolerated.

## Development setup

### Prerequisites

- **Rust 1.84+** with the `wasm32v1-none` target
- **stellar-cli** 27.1.0 (for building wasm artifacts and running deploy/invoke commands)
- **git**

### Installation

Follow the quick-start steps in the [README](README.md) to get a working build before making changes:

```bash
# Rust toolchain + wasm target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32v1-none

# Stellar CLI
cargo install --locked stellar-cli --version 27.1.0

# Verify the workspace compiles and tests pass
cargo test --workspace --features testutils
```

## Repository layout

```text
contracts/
├── registry/    # archguard-registry — the on-chain watch-list
└── extender/    # archguard-extender — fund custody & extension accounting
```

Both contracts share the same internal structure:

- `src/lib.rs` — contract entry points (`#[contract]` / `#[contractimpl]`)
- `src/types.rs` — storage keys (`DataKey`), data types, and events
- `src/errors.rs` — contract error codes (`#[contracterror]`)
- `src/test.rs` — unit tests

The workspace root `Cargo.toml` pins `soroban-sdk = "=27.0.5"` (exact version, no caret range) and enables
`overflow-checks` in the release profile (required by `stellar contract build`).

## Making changes

### Branching

1. Fork the repository.
2. Create a feature branch off `main` with a descriptive name, e.g. `feat/add-entry-pagination` or
   `fix/extender-overflow`:

   ```bash
   git checkout -b feat/my-change
   ```

### Coding conventions

- Follow the surrounding style in the file you are editing — the codebase uses the standard `rustfmt` style.
- Run `cargo fmt` before committing.
- Keep `#![no_std]` at the top of contract crates — Soroban contracts must not use the standard library.
- Document every public function with a doc comment, including **Auth** and **Errors** sections, matching the
  existing style in `lib.rs`.
- Never introduce a new dependency without a clear justification. The SDK version is pinned in the workspace
  manifest — do not change it without discussion.
- Storage access must follow the Archguard storage policy (see the [README](README.md#storage--ttl-policy)):
  instance storage for small/always-needed values, persistent storage for scalable data, and an explicit
  `extend_ttl` call after every state-changing write.

### Testing

- Every change must ship with tests. Run the full suite locally:

  ```bash
  cargo test --workspace --features testutils
  ```

- Run `cargo fmt --check` to verify formatting:

  ```bash
  cargo fmt --check
  ```

- If you touch the contract interface (new functions, changed arguments, new events/errors), update the
  corresponding sections in the [README](README.md) (function tables, event tables, error tables).

## Commit style

This repository uses [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description
```

Common types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`.

Examples:

```text
feat(registry): add batch remove for watched entries
fix(extender): guard against balance overflow on deposit
test(extender): cover set_operator auth failure
docs: document the storage & TTL policy
```

## Pull request process

1. Push your branch and open a pull request against `main`.
2. Give the PR a clear title following the [commit style](#commit-style) and describe the change, why it is needed,
   and any design decisions worth noting.
3. Make sure **CI passes** — the workflow runs the workspace tests and builds both wasm artifacts. Fix any failures
   before requesting review.
4. Request review — **one approval is required** before merge.
5. Address review feedback with follow-up commits (or amend and force-push if you prefer a clean history).
6. Once approved and green, the PR can be squashed and merged.

## CI

The repository uses GitHub Actions (see `.github/workflows/ci.yml`). On every push to `main` and every pull request
it:

1. Installs the pinned Rust toolchain with the `wasm32v1-none` target.
2. Runs `cargo test --workspace --features testutils`.
3. Builds the registry and extender wasm artifacts with `stellar contract build`.

Your PR must pass all of these before it can be merged.

## Security

Archguard is **unaudited software** — handle it with care, especially when mainnet funds are involved. If you find a
security vulnerability, **do not open a public issue**. Report it privately — see [SECURITY.md](SECURITY.md) for the
disclosure process.

## Questions?

Open a [GitHub discussion](https://github.com/smog123/archguard-contract/discussions) for questions or ideas, or
reach out to the [maintainers](README.md#maintainers) directly.
