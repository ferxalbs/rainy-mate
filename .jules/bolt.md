## 2024-05-19 - [React Performance Anti-Pattern]
**Learning:** In React streaming chat interfaces, passing inline fallback arrays (e.g., `trace || []`) to child components defeats `React.memo()` due to referential inequality on every render. This causes complex child components to re-render constantly even when memoized.
**Action:** Extract fallback arrays as stable global constants (e.g., `const EMPTY_ARRAY: never[] = [];`) outside the component to preserve memoization and wrap complex components with `React.memo` and callbacks passed to them with `useCallback`.
