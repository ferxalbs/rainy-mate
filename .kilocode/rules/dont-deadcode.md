---
trigger: always_on
---

# No Dead Code Rule

## Purpose
Prevent unused code from accumulating in the codebase.

## Requirements
- All code must be actively used or explicitly marked for future use
- Warnings about unused code are not acceptable
- Unmarked dead code is a critical violation

## Marking for Future Use
Use one of these patterns:
```
// @deprecated {reason} - marked for removal in v{version}
// @TODO {reason} - will be implemented in {timeline}
// @RESERVED {reason} - reserved for future use
```

## Violations
- Leaving commented-out code blocks
- Unused functions, variables, or imports without markers
- Code marked as deprecated but never removed
- Accumulating technical debt through dead code

## Enforcement
Remove all unmarked dead code immediately during code review.
