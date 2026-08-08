# Archguard Contracts

On-chain half of **Archguard** — a tool that watches deployed Soroban
contracts' storage TTL and auto-extends it before entries get archived.

This workspace contains two Soroban contracts:

| Contract | Purpose |
| --- | --- |
| `contracts/registry` (`archguard-registry`) | On-chain watch-list: orgs register and maintain "watched entries" (contract + storage key) with per-entry extension policy. Pure source of truth for the off-chain keeper; holds no funds. |
| `contracts/extender` (`archguard-extender`) | Fund custody and accounting: holds per-org prepaid XLM balances and records extension costs charged by the keeper. |

The two contracts never call each other on-chain. All coordination happens
off-chain: the keeper service (part of `archguard-app`, a separate repo)
reads due entries from the registry, performs the TTL extension via
`ExtendFootprintTTLOp` (which must be the sole operation of its
transaction, so it cannot be wrapped in a contract-to-contract call), then
debits the org's balance on the extender via `record_extension_cost`.

## Building and testing

- Toolchain: Rust 1.84+ with the `wasm32v1-none` target.
- SDK: `soroban-sdk 27.0.5` (pinned exact version, no caret range).
- Tests: `cargo test --features testutils` (per contract, e.g.
  `cargo test -p archguard-registry --features testutils`).
- Wasm artifacts: `stellar contract build` (never `wasm32-unknown-unknown`).

## Contract addresses

Placeholders — filled in after deployment:

| Network | Registry | Extender |
| --- | --- | --- |
| Testnet | `TODO` | `TODO` |
| Mainnet | `TODO` | `TODO` |
