# Beam Deployment Guardrails

- Do not store large blobs onchain when an event plus digest will do.
- Keep stream identifiers stable or downstream indexers will fork their state.
- Favor append-only records over mutable history when auditing matters.
- If records have economic impact, add authorization instead of trusting anyone to write them.
