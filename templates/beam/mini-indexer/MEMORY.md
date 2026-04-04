# Beam Template Memory

## Template
Mini Indexer

## Deployment Intent
- Emit normalized Beam events that offchain services can ingest without replay ambiguity.
- Keep onchain storage minimal while preserving checkpoint state.
- Use indexed topics deliberately for high-selectivity reads.

## Operational Notes
- Streams are arbitrary bytes32 namespaces.
- Checkpoints can represent sync height, sequence number, or domain-specific cursors.
- The contract is best paired with a lightweight offchain consumer.
