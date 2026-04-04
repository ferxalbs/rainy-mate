# Beam Template Memory

## Template
Simple ERC20

## Deployment Intent
- Ship a compact fungible token on Beam with no external import graph.
- Keep mint authority explicit and easy to audit.
- Prefer immutable metadata and custom errors over string-heavy revert paths.

## Operational Notes
- Minting is owner-gated.
- Transfers are standard ERC-20 semantics.
- Beam gas is the native cost surface, so keep follow-up admin actions minimal.
