<!-- BANNER — manual follow-up: replace with a real banner image (e.g. repo
     social preview) before Wave review. Do not ship the placeholder. -->
![Archguard Contracts](https://placehold.co/1200x300/090d16/6366f1/png?text=Archguard+Contracts)

# Archguard Contracts

[![CI](https://img.shields.io/github/actions/workflow/status/smog123/archguard-contract/ci.yml?branch=main&label=CI&logo=github)](https://github.com/smog123/archguard-contract/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.84%2B-dea584?logo=rust)](https://www.rust-lang.org/)
[![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-27.0.5-7c5cff)](https://soroban.stellar.org/)
[![Stellar CLI](https://img.shields.io/badge/Stellar%20CLI-27.1.0-3e3e3e)](https://github.com/stellar/stellar-cli)

**On-chain half of [Archguard](https://github.com/smog123/archguard-app)** — automatically watch deployed Soroban
contracts' storage TTL and extend it before entries get archived.

Stellar's Soroban platform restores expired ledger entries from archives at a cost. If a contract instance or its
storage is not extended in time, the data is archived and any subsequent access becomes slow and expensive.
Archguard prevents this by continuously monitoring TTLs and re-extending them — this repository contains the two
on-chain contracts that power the watch-list and the funding model.

---

## Table of Contents

- [About](#about)
- [Features](#features)
- [Architecture](#architecture)
  - [How it works](#how-it-works)
  - [Diagram](#diagram)
- [Repository structure](#repository-structure)
- [Contracts](#contracts)
  - [Registry (`archguard-registry`)](#registry-archguard-registry)
  - [Extender (`archguard-extender`)](#extender-archguard-extender)
  - [Storage & TTL policy](#storage--ttl-policy)
- [Tech stack](#tech-stack)
- [Getting started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
  - [Running the tests](#running-the-tests)
  - [Building the WASM artifacts](#building-the-wasm-artifacts)
  - [Deploying to testnet](#deploying-to-testnet)
- [Interacting with the contracts](#interacting-with-the-contracts)
- [Deployed addresses](#deployed-addresses)
- [Contributing](#contributing)
- [Security](#security)
- [Maintainers](#maintainers)
- [Contributors](#contributors)
- [Related projects](#related-projects)

---

## About

Archguard keeps Stellar Soroban contract instances and persistent data entries alive by tracking their storage TTL
and re-extending them before they expire. This repo holds the two on-chain contracts:

| Contract | Purpose |
| --- | --- |
| `archguard-registry` | The **watch-list** — orgs register and track "watched entries" (a contract + storage key pair) with a per-entry extension policy. |
| `archguard-extender` | **Prepaid custody balances** — holds per-org prepaid XLM and records extension costs charged by the off-chain keeper. |

The off-chain keeper lives in [archguard-app](https://github.com/smog123/archguard-app). It performs the actual
extension work (via the `ExtendFootprintTTLOp` ledger operation) and settles the cost against each org's balance in
the extender.

## Features

- **Watch-list registry** — orgs self-register, add/remove watched entries, and update per-entry extension policies on-chain.
- **Per-entry extension policy** — each watched entry configures its own `extend_threshold_ledgers` and `extend_to_ledgers` values, so orgs control how aggressively their storage is kept alive.
- **Prepaid funding** — orgs deposit native XLM (in stroops) into the extender's custody and the keeper debits extension costs against it.
- **Auto-extend flag** — entries can be flagged `auto_extend` so the keeper extends them without asking.
- **Event-driven** — every state change publishes a Soroban event (org registered, entry added/removed, deposit, withdrawal, charge, …) so off-chain tooling can react and index.
- **TTL self-maintenance** — every state-changing call extends the contract's own instance TTL, and every write extends its persistent-entry TTL, per the [storage & TTL policy](#storage--ttl-policy).
- **Durability-aware** — entries declare whether they watch `Instance` or `Persistent` storage; `Temporary` storage is excluded by design.
- **Webhook-ready** — only the SHA-256 hash of a notification webhook is stored on-chain (never the raw URL), keeping storage costs low.

## Architecture

The two contracts **never call each other on-chain**; all coordination happens off-chain through the keeper.

- `archguard-registry` is the **pure source of truth**: orgs register watched entries (contract + storage key) with a
  per-entry extension policy. Reads are public; writes require org authentication.
- `archguard-extender` is the **fund custody** layer: per-org prepaid XLM balances and the extension costs the keeper
  charges. Money logic is isolated here so it is auditable separately from the mostly-public registry.

### How it works

1. An org registers with the registry and adds one or more watched entries, each describing a contract storage key
   to keep alive plus a TTL policy.
2. The off-chain keeper scans the registry for due entries — those whose remaining TTL has dropped below
   `extend_threshold_ledgers`.
3. The keeper submits an `ExtendFootprintTTLOp` transaction (which **must be the sole operation** of its transaction)
   to extend the target storage out to `extend_to_ledgers`.
4. The keeper then calls `record_extension_cost` on the extender to debit the org's prepaid balance for the work
   performed.
5. If the org's balance is insufficient, the extender publishes an `insufficient_balance` event instead of reverting —
   the keeper learns the org is underfunded and can alert it.

### Diagram

```text
┌──────────────┐   register / add entries    ┌──────────────────────┐
│     Org      │ ───────────────────────────▶ │                      │
│ (off-chain)  │                              │  archguard-registry  │
└──────────────┘                              │  (watch-list, on-chain)│
        │                                     └──────────┬───────────┘
        │ deposit XLM                                    │ reads due entries
        ▼                                                │
┌──────────────┐   charge costs      ┌───────────────────▼───────────┐
│  extender    │ ◀─────────────────  │         Keeper                │
│ (custody,    │                     │      (off-chain,             │
│  on-chain)   │ ──────────────────▶ │    archguard-app)            │
└──────────────┘   events/status     └───────────────────┬───────────┘
                                                        │ ExtendFootprintTTLOp
                                                        ▼
                                              ┌──────────────────────┐
                                              │  Watched Soroban     │
                                              │  contract storage    │
                                              └──────────────────────┘
```

## Repository structure

```text
.
├── contracts/
│   ├── registry/              # archguard-registry — the watch-list contract
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs         # contract entry points
│   │       ├── types.rs       # storage keys, data types, events
│   │       ├── errors.rs      # contract error codes
│   │       └── test.rs        # unit tests
│   └── extender/              # archguard-extender — fund custody contract
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs         # contract entry points
│           ├── types.rs       # storage keys, data types, events
│           ├── errors.rs      # contract error codes
│           └── test.rs        # unit tests
├── .github/workflows/ci.yml   # CI: workspace tests + wasm builds
├── Cargo.toml                 # workspace manifest (SDK pinned, release profile)
├── CONTRIBUTING.md
└── SECURITY.md
```

## Contracts

### Registry (`archguard-registry`)

The watch-list. Teams register orgs and add watched entries (a contract + storage key pair) whose TTL the keeper
monitors and auto-extends.

#### Functions

| Function | Auth | Description |
| --- | --- | --- |
| `init()` | none (first call wins) | Initializes the registry; seeds the entry-id counter. A second call panics ("already initialized"). |
| `register_org(org, admin, notify_webhook)` | `org` | Registers a new org with an admin address and the SHA-256 hash of its notification webhook. |
| `add_watched_entry(org, contract_id, durability, key, extend_threshold_ledgers, extend_to_ledgers, auto_extend)` | `org` | Adds a watched entry and returns its id (starting at 1). `key = None` watches the whole contract instance. |
| `remove_watched_entry(org, entry_id)` | `org` | Removes a watched entry and drops its id from the org's entry list. |
| `update_entry_policy(org, entry_id, extend_threshold_ledgers, extend_to_ledgers, auto_extend)` | `org` | Updates the extension policy of an existing entry. |
| `get_org_entries(org)` | none (read-only) | Returns all entries owned by an org, in insertion order. |
| `get_entry(entry_id)` | none (read-only) | Returns a single watched entry by id. |
| `deactivate_org(org)` | `org` | Deactivates an org so it can no longer add entries (existing entries stay readable). |

#### Events

| Event | Topics |
| --- | --- |
| `OrgRegistered` | `["org_registered", org]` |
| `EntryAdded` | `["entry_added", id]` |
| `EntryRemoved` | `["entry_removed", id]` |
| `EntryPolicyUpdated` | `["entry_policy_updated", id]` |
| `OrgDeactivated` | `["org_deactivated", org]` |

#### Errors

| Code | Error | Meaning |
| --- | --- | --- |
| 1 | `OrgNotFound` | The org has not been registered yet. |
| 2 | `EntryNotFound` | No watched entry exists with the given id. |
| 3 | `NotEntryOwner` | The org does not own the entry it is trying to modify. |
| 4 | `OrgInactive` | The org has been deactivated and cannot make changes. |
| 5 | `InvalidThreshold` | `extend_threshold_ledgers` is not strictly below `extend_to_ledgers`. |

### Extender (`archguard-extender`)

Fund custody and extension accounting. Holds per-org prepaid XLM balances and records the extension costs the keeper
charges against them.

#### Functions

| Function | Auth | Description |
| --- | --- | --- |
| `init(operator, native_asset)` | none (first call wins) | Initializes the extender with the keeper (operator) address and the native XLM SAC address for the network in use. A second call panics. |
| `deposit(org, amount)` | `org` | Transfers native XLM (stroops) from the org into the extender's custody and credits the org's balance. |
| `withdraw(org, amount)` | `org` | Transfers native XLM back from custody to the org, debiting its balance. |
| `record_extension_cost(org, cost)` | **operator** (the keeper) | Debits the org's balance for extension work the keeper performed. |
| `get_balance(org)` | none (read-only) | Returns the org's prepaid balance in stroops (0 when never funded). |
| `set_operator(new_operator)` | current operator | Replaces the keeper address (native asset address is preserved). |

> **Underfunded orgs — deliberate design choice:** when the org's balance is less than a recorded cost, the
> transaction does **not** revert. The contract publishes an `insufficient_balance` event and returns `Ok(())` with
> the balance unchanged, so the keeper learns the org is underfunded without a destructive rollback.

#### Events

| Event | Topics |
| --- | --- |
| `Deposited` | `["deposited", org]` |
| `Withdrawn` | `["withdrawn", org]` |
| `ExtensionCharged` | `["extension_charged", org]` |
| `InsufficientBalance` | `["insufficient_balance", org]` |

#### Errors

| Code | Error | Meaning |
| --- | --- | --- |
| 1 | `InsufficientBalance` | The org's prepaid balance cannot cover the requested amount. |
| 2 | `NotOperator` | No operator configured (not initialized) or the caller is not the operator. |
| 3 | `InvalidAmount` | The amount is not a positive, valid amount. |

### Storage & TTL policy

Both contracts follow the same storage discipline:

- **Instance storage** for small, few, always-needed values (org configs, id counter, balances, operator config).
- **Persistent storage** for data that scales with usage (watched entries, per-org entry id lists).
- **Temporary storage is never used** — temporary data cannot be meaningfully TTL-watched.
- After every state-changing call, the contract extends its **instance entry TTL**, and after every write it extends
  the **persistent entry TTL**:
  - `TTL_THRESHOLD = 17,280` ledgers (~1 day at 5s/ledger) — re-extend when remaining TTL drops below this.
  - `TTL_EXTEND_TO = 518,400` ledgers (~30 days at 5s/ledger) — extend out to this value.

## Tech stack

- **Language:** Rust 1.84+ (edition 2021)
- **SDK:** `soroban-sdk 27.0.5` — pinned exact version, no caret range (do not use `-rc` pre-releases)
- **Target:** `wasm32v1-none` (never `wasm32-unknown-unknown`)
- **CLI:** `stellar-cli 27.1.0`
- **CI:** GitHub Actions — runs the workspace tests and builds both wasm artifacts on push/PR to `main`

## Getting started

### Prerequisites

- Rust 1.84+ with the `wasm32v1-none` target
- `stellar-cli` 27.1.0
- A funded Stellar testnet account (identity) for deployment

### Installation

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
```

### Running the tests

```bash
cargo test --workspace --features testutils
```

### Building the WASM artifacts

```bash
cd contracts/registry && stellar contract build
cd ../extender && stellar contract build
```

The wasm artifacts land in `target/wasm32v1-none/release/`.

### Deploying to testnet

```bash
# Registry
stellar contract deploy \
  --wasm target/wasm32v1-none/release/archguard_registry.wasm \
  --source admin \
  --network testnet

# Extender
stellar contract deploy \
  --wasm target/wasm32v1-none/release/archguard_extender.wasm \
  --source admin \
  --network testnet
```

Record the deployed contract ids — they are needed for the interaction examples below.

## Interacting with the contracts

> The exact `stellar contract invoke` argument syntax varies by CLI version; check `stellar contract invoke --help`
> for your installed version. Addresses below are placeholders.

```bash
# Initialize the registry
stellar contract invoke \
  --id <REGISTRY_ID> --source admin --network testnet \
  -- init

# Register an org (webhook = SHA-256 hash of the webhook URL, as bytes)
stellar contract invoke \
  --id <REGISTRY_ID> --source admin --network testnet \
  -- register_org \
  --org <ORG_ADDRESS> \
  --admin <ADMIN_ADDRESS> \
  --notify_webhook <32_BYTE_HASH>

# Add a watched entry (omit --key to watch the whole contract instance)
stellar contract invoke \
  --id <REGISTRY_ID> --source admin --network testnet \
  -- add_watched_entry \
  --org <ORG_ADDRESS> \
  --contract_id <WATCHED_CONTRACT_ADDRESS> \
  --durability Instance \
  --key <raw_key_bytes> \
  --extend_threshold_ledgers 17280 \
  --extend_to_ledgers 518400 \
  --auto_extend true

# Read a watched entry (no auth required)
stellar contract invoke \
  --id <REGISTRY_ID> --source admin --network testnet \
  -- get_entry --entry_id 1

# Initialize the extender (native asset = the network's XLM SAC address)
stellar contract invoke \
  --id <EXTENDER_ID> --source admin --network testnet \
  -- init \
  --operator <KEEPER_ADDRESS> \
  --native_asset <NATIVE_XLM_SAC_ADDRESS>

# Deposit XLM into the extender (amount in stroops)
stellar contract invoke \
  --id <EXTENDER_ID> --source admin --network testnet \
  -- deposit \
  --org <ORG_ADDRESS> \
  --amount 1000000000

# Check a balance (no auth required)
stellar contract invoke \
  --id <EXTENDER_ID> --source admin --network testnet \
  -- get_balance --org <ORG_ADDRESS>
```

## Deployed addresses

Deployed contract addresses — testnet explorer links to be added:

| Network | Registry | Extender |
| --- | --- | --- |
| Testnet | `CDAONHGO63LZKXO42LJTZWGFP5VZRZJXREMU7VCAWDILXKHSPBZXZ6RA` | `CBCK4CYVWNPVC3SJAQXUYYZUOWEKX7DQQJNNU25KZWHD43TICNIORWRF` |
| Mainnet | *not deployed* | *not deployed* |

## Contributing

We welcome contributions! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for setup, conventions (conventional
commits), and the pull-request process.

## Security

Archguard is **unaudited software** — use at your own risk, especially with mainnet funds. If you discover a
vulnerability, do **not** open a public issue. Report it privately per [SECURITY.md](SECURITY.md).

## Maintainers

| Photo | Name | Role | GitHub | Telegram |
| --- | --- | --- | --- | --- |
| ![avatar](https://github.com/smog123.png?size=64) | **smog** | Maintainer | [@smog123](https://github.com/smog123) | [@smog404](https://t.me/smog404) |

## Contributors

[![Contributors](https://contrib.rocks/image?repo=smog123/archguard-contract)](https://github.com/smog123/archguard-contract/graphs/contributors)

## Related projects

- [archguard-app](https://github.com/smog123/archguard-app) — the off-chain keeper and dashboard
  ([live dashboard on Vercel](https://archguard-app-e67xx0s07-smog3.vercel.app), testnet)
- [Stellar Soroban](https://soroban.stellar.org/) — smart-contract platform this project builds on

---

<p align="center">
  <sub>Built with Rust & Soroban</sub>
</p>
