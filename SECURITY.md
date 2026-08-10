# Security Policy

Thank you for helping keep Archguard safe. Security is taken seriously — this project handles smart contracts that
may custody real funds.

## ⚠️ Disclaimer

**Archguard is unaudited software.** Use it at your own risk, especially with mainnet funds. The contracts have not
been independently audited, and no guarantee of security is made. Treat anything deployed to mainnet as
experimental.

## Reporting a vulnerability

If you discover a security vulnerability, **do not open a public issue** or a public pull request. Please report it
privately so it can be fixed before it is disclosed.

### How to report

Contact the maintainer directly:

- Telegram: [@smog404](https://t.me/smog404)
- Email: **[FILL IN]**
- GitHub: [@smog123](https://github.com/smog123) (private message)

### What to include

To help us respond quickly and effectively, please include:

- **Description** — what the vulnerability is and where it lives (contract, function, or file).
- **Severity** — your assessment of impact (e.g. loss of funds, griefing, denial of service).
- **Steps to reproduce** — minimal reproduction, including any test snippets or transactions.
- **Impact** — what an attacker could do, and under which conditions.

### What to expect

- We aim to acknowledge your report within **48 hours**.
- We will keep you informed as the issue is triaged and fixed.
- We will credit you (if you wish) once the issue is resolved and disclosed.

## Scope

This policy applies to both Archguard repositories:

- [archguard-contract](https://github.com/smog123/archguard-contract) — the on-chain contracts (`registry` and `extender`)
- [archguard-app](https://github.com/smog123/archguard-app) — the off-chain keeper and dashboard

The following are **in scope**:

- Bugs in the `registry` and `extender` contract logic that could lead to loss of funds, unauthorized access, or
  denial of service.
- Bugs in the off-chain keeper that could lead to incorrect charging, incorrect TTL extensions, or loss of funds.
- Logic errors in authentication, authorization, or balance accounting.

The following are **out of scope** (but still welcome as regular issues):

- Issues that require a compromised admin/operator key or a compromised Stellar network.
- Best-practice suggestions without a demonstrated vulnerability.
- Phishing or social-engineering of end users.

## Supported versions

This is a fast-moving project under active development. Only the latest release on the `main` branch is supported.
Patches are applied to `main`; there are currently no long-term support (LTS) releases.

## Disclosure policy

We follow a coordinated-disclosure approach:

1. **Private disclosure** — the report is kept private until a fix is ready.
2. **Fix** — the maintainer prepares and merges a fix.
3. **Public disclosure** — the vulnerability is announced (including a CVE if applicable) once the fix is deployed,
   giving users time to upgrade.

We ask that you give us a reasonable window (typically **90 days** or as agreed) before publicly disclosing a
reported vulnerability.
