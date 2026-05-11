## 2024-05-11 - React Performance Optimization in MessageBubble

**Learning:** Using inline fallback arrays like `message.trace || []` when passing props to child components is an anti-pattern. Since `[]` evaluates to a new array reference on every render, it triggers an unnecessary re-render of child components even if those components are wrapped in `React.memo`. The `useCallback` hook is also necessary to prevent passing new function references. In highly dynamic components like streaming chats where state updates rapidly, preventing unnecessary React tree reconciliations is critical for good performance.

**Action:** Extract inline array fallbacks to stable global constants (e.g., `const EMPTY_ARRAY: never[] = [];`), use `React.useCallback` for functions, and ensure all non-trivial child components are wrapped in `React.memo()` to effectively avoid re-renders.
