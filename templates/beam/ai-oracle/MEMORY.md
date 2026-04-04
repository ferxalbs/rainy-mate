# Beam Template Memory

## Template

AI Oracle

## Deployment Intent

- Expose AI request and fulfillment events on Beam for agentic or game-side consumers.
- Keep the reporter set tightly controlled and replaceable.
- Persist compact response hashes and scores for verifiable offchain payload lookup.

## Operational Notes

- The oracle stores a response URI plus hash, not the full model payload.
- Requesters can create jobs permissionlessly unless you tighten policy.
- Reporters are explicit allowlist accounts.
