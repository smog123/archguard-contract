<!-- BANNER — manual follow-up: replace with a real banner image (e.g. repo
     social preview) before Wave review. Do not ship the placeholder. -->
![Archguard Contracts](https://placehold.co/1200x300/090d16/6366f1/png?text=Archguard+Contracts)

# Archguard Contracts

On-chain half of **Archguard** — automatically watch deployed Soroban
contracts' storage TTL and extend it before entries get archived.

## Table of Contents

- [What it does](#what-it-does)
- [Maintainers](#maintainers)
- [Quick start](#quick-start)
- [Architecture](#architecture)
- [Building and testing](#building-and-testing)
- [Contributing](#contributing)
- [Contributors](#contributors)
- [About](#about)

## What it does

Archguard keeps Stellar Soroban contract instances and persistent data
entries alive by tracking their storage TTL and re-extending them before
they expire. This repo holds the two on-chain contracts: `registry` (the
watch-list) and `extender` (prepaid custody balances). The off-chain keeper
in [archguard-app](https://github.com/smog123/archguard-app) does the actual
extension work and settles costs against the extender.

## Maintainers

| Photo | Name | Role | GitHub | Telegram |
| --- | --- | --- | --- | --- |
| ![avatar](https://github.com/smog123.png?size=64) | **[Your Name]** | Maintainer | [@smog123](https://github.com/smog123) | **[Your Telegram]** |

## Quick start

```bash
# 1. Install Rust 1.84+ and the Stellar WebAssembly target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32v1-none

# 2. Install the Stellar CLI (27.x)
cargo install --locked stellar-cli --version 27.1.0

# 3. Configure testnet and an identity
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
stellar config identity generate admin
stellar config identity fund admin

# 4. Run the tests
cargo test --workspace --features testutils

# 5. Build the wasm artifacts
cd contracts/registry && stellar contract build
cd ../extender && stellar contract build

# 6. Deploy to testnet (wasm lands in target/wasm32v1-none/release/)
stellar contract deploy \
  --wasm target/wasm32v1-none/release/archguard_registry.wasm \
  --source admin \
  --network testnet
```

## Architecture

Two contracts that never call each other on-chain. `archguard-registry` is
the pure source of truth: orgs register watched entries (contract + storage
key) with a per-entry extension policy. `archguard-extender` holds per-org
prepaid XLM and records extension costs charged by the keeper. All
coordination happens off-chain — the keeper reads due entries, extends TTL
via `ExtendFootprintTTLOp` (which must be the sole operation of its
transaction), then debits the org's balance.

## Building and testing

- Toolchain: Rust 1.84+ with the `wasm32v1-none` target.
- SDK: `soroban-sdk 27.0.5` (pinned exact version, no caret range).
- Tests: `cargo test --workspace --features testutils`.
- Wasm artifacts: `stellar contract build` (never `wasm32-unknown-unknown`).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Please report security issues
privately — see [SECURITY.md](SECURITY.md).

## Contributors

[![Contributors](https://contrib.rocks/image?repo=smog123/archguard-contract)](https://github.com/smog123/archguard-contract/graphs/contributors)

## About

Live dashboard: [archguard-app on Vercel](https://archguard-app-e67xx0s07-smog3.vercel.app)
(testnet).

Deployed contract addresses — testnet explorer links to be added:

| Network | Registry | Extender |
| --- | --- | --- |
| Testnet | `CDAONHGO63LZKXO42LJTZWGFP5VZRZJXREMU7VCAWDILXKHSPBZXZ6RA` | `CBCK4CYVWNPVC3SJAQXUYYZUOWEKX7DQQJNNU25KZWHD43TICNIORWRF` |
| Mainnet | *not deployed* | *not deployed* |
