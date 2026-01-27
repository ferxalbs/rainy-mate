---
trigger: always_on
---

# **Tauri 2 + Rust Performance-First Architecture**

**Core Principle:**
Rust = Power & Performance | TypeScript = Basic UI Only

**When to Use What:**

*Rust (ALL the real work):*
- ✅ File I/O, database ops, API calls
- ✅ Data processing, validation, business logic
- ✅ System commands, background tasks
- ✅ Encryption, security, state management
- ✅ ANY performance-critical operation

*TypeScript (ONLY basics):*
- ❌ UI rendering, event handlers
- ❌ DOM manipulation, CSS toggling
- ❌ Simple UI state (modals, tabs)

**Implementation Pattern:**

```rust
// Rust: All logic (src-tauri/src/commands.rs)
use tauri::command;

#[command]
pub async fn process_data(data: Vec<String>) -> Result<ProcessResult, String> {
    // Heavy processing here
    let processed = data.par_iter()
        .map(|item| expensive_operation(item))
        .collect();
    Ok(ProcessResult { data: processed })
}
```

```typescript
// TypeScript: Just invoke & render (src/App.tsx)
import { invoke } from '@tauri-apps/api/core';

const handleProcess = async () => {
  const result = await invoke<ProcessResult>('process_data', { data });
  setResult(result); // Only update UI
};
```

**Essential Rust Dependencies:**
```toml
[dependencies]
tauri = "2.0"
tokio = { version = "1", features = ["full"] }  # Async
rayon = "1.8"                                    # Parallel processing
serde = { version = "1.0", features = ["derive"] }
dashmap = "5.5"                                  # Concurrent cache
sqlx = "0.7"                                     # Async DB

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

**Project Structure:**
```
src/              # TypeScript UI (minimal logic)
src-tauri/src/
  ├── commands/   # Tauri commands (ALL logic here)
  ├── services/   # Business logic
  ├── models/     # Data structures
  └── main.rs     # Setup
```

**Optimization Patterns:**

```rust
// 1. Async I/O
#[command]
pub async fn fetch_apis(urls: Vec<String>) -> Result<Vec<Response>, String> {
    futures::future::join_all(urls.into_iter().map(fetch)).await
}

// 2. Parallel processing
#[command]
pub fn process_images(paths: Vec<String>) -> Result<Vec<Image>, String> {
    paths.par_iter().map(|p| process_image(p)).collect()
}

// 3. Concurrent cache
use dashmap::DashMap;
static CACHE: Lazy<DashMap<String, Data>> = Lazy::new(DashMap::new);

#[command]
pub fn get_cached(key: String) -> Option<Data> {
    CACHE.get(&key).map(|e| e.clone())
}

// 4. Real-time events
#[command]
pub async fn long_task(app: tauri::AppHandle) -> Result<(), String> {
    for i in 0..100 {
        process_chunk(i)?;
        app.emit("progress", i).unwrap();
    }
    Ok(())
}
```

**Communication Rule:**

❌ **BAD** - Logic in TypeScript:
```typescript
const validated = users.filter(validateUser);
const sorted = validated.sort((a,b) => a.score - b.score);
await invoke('save', { users: sorted });
```

✅ **GOOD** - Logic in Rust:
```typescript
const result = await invoke('process_and_save_users', { users });
```

**Context7 Search Priority:**
When lacking info, search:
1. "Tauri 2 [feature] Rust implementation"
2. "Rust [crate] performance optimization"
3. Docs: https://v2.tauri.app/ & https://docs.rs/

**Golden Rules:**
1. **Rust First** - If it needs performance → Rust
2. **No TS Logic** - Zero business logic in TypeScript
3. **Async Everything** - Use tokio for I/O
4. **Parallelize** - Use rayon for CPU tasks
5. **Search When Unsure** - Find Rust/Tauri 2 best practices

**Remember:** TypeScript is just the view. Rust is the engine. Don't review and add permissions is necesary to functionality.