## 2024-06-03 - [React Streaming Chat Performance]
**Learning:** Passing inline fallback arrays (e.g., `prop || []`) to child components defeats `React.memo()` due to referential inequality on every render. Extract fallback arrays as stable global constants (e.g., `const EMPTY_ARRAY: never[] = [];`) outside the component to preserve memoization.
**Action:** Always verify referential stability of props (especially functions via `useCallback` and collections via constants/useMemo) when wrapping heavy child components in `React.memo()` in a streaming update context.
