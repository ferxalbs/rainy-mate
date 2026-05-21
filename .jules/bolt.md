## 2024-05-18 - [Memoization of Complex Chat Components]
**Learning:** In streaming chat interfaces, React.memo alone on child components is insufficient if the parent (`MessageBubble`) passes functions or arrays that re-allocate on every render (due to token streams). We must wrap handlers (like `handleExecuteToolCalls`) in `React.useCallback`.
**Action:** Always verify referential equality of props passed to memoized components in high-frequency update loops.
