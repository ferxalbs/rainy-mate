# Beam Template Memory

## Template
Basic Game Contract

## Deployment Intent
- Ship a compact onchain session registry for Beam-native game rounds.
- Favor event-rich writes that offchain game services can index cheaply.
- Keep reward distribution simple enough for predictable gas.

## Operational Notes
- Scores only move upward per player best.
- Rewards are funded in native BEAM and claimed separately.
- Session IDs are arbitrary bytes32 values from the game backend.
