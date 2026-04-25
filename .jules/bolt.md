## 2024-06-25 - Prevent MessageBubble list child component re-renders
**Learning:** In React streaming chat interfaces, defining fallback arrays inline (e.g., `prop || []`) passed to child components defeats `React.memo()` due to referential inequality on every render.
**Action:** Extract fallback arrays as stable global constants (e.g., `const EMPTY_ARRAY: never[] = [];`) outside the component to preserve memoization.
