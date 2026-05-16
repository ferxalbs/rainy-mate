## 2023-10-27 - [React Performance in Streaming Lists]
**Learning:** Passing inline fallback arrays (like `prop || []`) to child components within message lists defeats `React.memo()` due to referential inequality on every token update, leading to severe performance degradation in streaming chat interfaces.
**Action:** Extract fallback arrays as stable global constants (e.g., `const EMPTY_ARRAY: never[] = [];`) outside the component and wrap complex child components with `React.memo()`. Also memoize callbacks using `useCallback`.
