## 2026-04-29 - [React Component Re-render Optimization]
**Learning:** Passing referentially unequal inline fallback arrays (like `prop || []`) or callback functions down to heavily optimized child components (like `React.memo`) causes unnecessary teardown and re-parsing, defeating the memoization benefits.
**Action:** Use stable global constants for fallback empty arrays and wrap callback functions in `React.useCallback` when passing them to memoized complex child components.
