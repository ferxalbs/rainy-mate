---
name: rust-expert
description: Expert Rust code reviewer and developer. Use for Rust code review, writing production-grade Rust, eliminating warnings, modularization, error handling, performance optimization, and following Rust best practices. Activates when working with .rs files, Cargo projects, or when user mentions Rust code quality, warnings, clippy, or production-ready Rust.
allowed-tools: Read, Write, Grep, Glob, Bash
---

# Rust Expert

Expert-level Rust development with focus on:
- Zero warnings (compiler + clippy)
- Production-grade stability
- Idiomatic code patterns
- Proper modularization
- Memory safety guarantees
- Performance optimization

## Core Principles

### 1. Zero-Warning Policy
Every piece of code must compile with:
```bash
cargo build --release
cargo clippy -- -D warnings
cargo fmt --check
```

No warnings allowed. Ever.

### 2. Error Handling
- Use `Result<T, E>` for recoverable errors
- Use `panic!` only for unrecoverable errors
- Implement custom error types with `thiserror` or `anyhow`
- Propagate errors with `?` operator
- Never use `.unwrap()` or `.expect()` in production code without careful justification

### 3. Memory Safety
- Leverage ownership system fully
- Use references over clones when possible
- Prefer `&str` over `String` for function parameters
- Use `Cow<'a, str>` when conditional ownership needed
- Smart pointers: `Box<T>`, `Rc<T>`, `Arc<T>` only when necessary

### 4. Modularization
- Organize code into logical modules (`mod.rs` or `module_name.rs`)
- Keep functions small and focused
- Use trait bounds for generic constraints
- Separate concerns: business logic, I/O, error handling
- Public API in `lib.rs`, implementation details in submodules

## Code Review Checklist

### Compilation & Linting
- [ ] Compiles without warnings (`cargo build`)
- [ ] Passes Clippy with `-D warnings`
- [ ] Formatted with `cargo fmt`
- [ ] No deprecated APIs
- [ ] All `unsafe` blocks justified and documented

### Error Handling
- [ ] All `Result` types properly handled
- [ ] No naked `.unwrap()` or `.expect()` without justification
- [ ] Custom error types for library crates
- [ ] Errors provide context
- [ ] `panic!` only for programmer errors

### Performance
- [ ] No unnecessary clones
- [ ] Efficient iterator usage
- [ ] Appropriate data structures
- [ ] No allocations in hot paths when avoidable
- [ ] Benchmarks for critical code

### Safety & Correctness
- [ ] No unsafe code unless absolutely necessary
- [ ] All unsafe blocks have safety comments
- [ ] No data races
- [ ] No undefined behavior
- [ ] Proper lifetime annotations

### API Design
- [ ] Clear function signatures
- [ ] Consistent naming conventions
- [ ] Documentation comments (`///`) for public items
- [ ] Examples in documentation
- [ ] Semantic versioning considerations

### Testing
- [ ] Unit tests for core functionality
- [ ] Integration tests where appropriate
- [ ] Property-based tests for complex logic (use `proptest` or `quickcheck`)
- [ ] Error cases tested
- [ ] Edge cases covered

## Best Practices Reference

### Module Organization
```rust
// src/lib.rs
pub mod config;
pub mod error;
pub mod processor;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
}

// src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub timeout: u64,
    pub max_retries: usize,
}

impl Config {
    pub fn from_file(path: impl AsRef<std::path::Path>) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content)
            .map_err(|e| crate::Error::Parse(e.to_string()))
    }
}
```

### Idiomatic Error Handling
```rust
// ❌ Bad
fn process(data: &str) -> String {
    data.parse::<i32>().unwrap().to_string()
}

// ✅ Good
fn process(data: &str) -> Result<String, ParseIntError> {
    data.parse::<i32>().map(|n| n.to_string())
}

// ✅ Better (with context)
use anyhow::{Context, Result};

fn process(data: &str) -> Result<String> {
    data.parse::<i32>()
        .context("Failed to parse input as integer")?
        .to_string()
}
```

### Iterator Efficiency
```rust
// ❌ Bad - unnecessary collect
let sum: i32 = vec
    .iter()
    .map(|x| x * 2)
    .collect::<Vec<_>>()
    .iter()
    .sum();

// ✅ Good - direct iteration
let sum: i32 = vec.iter().map(|x| x * 2).sum();

// ✅ Good - use filter_map to combine operations
let valid: Vec<_> = items
    .iter()
    .filter_map(|item| item.parse::<i32>().ok())
    .collect();
```

### Ownership Patterns
```rust
// ❌ Bad - unnecessary clone
fn process_string(s: String) -> String {
    let copy = s.clone();
    copy.to_uppercase()
}

// ✅ Good - consume ownership
fn process_string(s: String) -> String {
    s.to_uppercase()
}

// ✅ Good - borrow when possible
fn process_string(s: &str) -> String {
    s.to_uppercase()
}
```

### Smart Pointer Usage
```rust
use std::rc::Rc;
use std::sync::Arc;

// Single-threaded shared ownership
let data = Rc::new(vec![1, 2, 3]);
let reference1 = Rc::clone(&data);
let reference2 = Rc::clone(&data);

// Multi-threaded shared ownership
let shared = Arc::new(vec![1, 2, 3]);
let thread_ref = Arc::clone(&shared);
std::thread::spawn(move || {
    println!("{:?}", thread_ref);
});
```

## Common Clippy Fixes

### Unused Results
```rust
// ❌ Warning: unused Result
fs::write("file.txt", "content");

// ✅ Fixed
fs::write("file.txt", "content")?;
// or
let _ = fs::write("file.txt", "content"); // if error truly doesn't matter
```

### Needless Return
```rust
// ❌ Warning
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// ✅ Fixed
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

### Single Match
```rust
// ❌ Warning
match x {
    Some(val) => println!("{}", val),
    _ => {}
}

// ✅ Fixed
if let Some(val) = x {
    println!("{}", val);
}
```

### Redundant Pattern Matching
```rust
// ❌ Warning
match result {
    Ok(val) => Some(val),
    Err(_) => None,
}

// ✅ Fixed
result.ok()
```

## Performance Optimization Patterns

### Avoid Allocations
```rust
// ❌ Slow - allocates String
fn format_number(n: i32) -> String {
    format!("Number: {}", n)
}

// ✅ Fast - uses stack buffer
use std::fmt::Write;

fn format_number(n: i32, buf: &mut String) {
    write!(buf, "Number: {}", n).unwrap();
}

// Or use itoa for simple integer formatting
fn format_number_fast(n: i32) -> String {
    itoa::Buffer::new().format(n).to_string()
}
```

### Use SmallVec for Small Collections
```rust
use smallvec::{SmallVec, smallvec};

// Stores up to 4 items on stack, heap for more
type SmallVec4<T> = SmallVec<[T; 4]>;

let mut vec: SmallVec4<i32> = smallvec![1, 2, 3];
vec.push(4); // still on stack
vec.push(5); // now moves to heap
```

### Lazy Static Initialization
```rust
use once_cell::sync::Lazy;

static REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap()
});

fn validate_date(s: &str) -> bool {
    REGEX.is_match(s)
}
```

## Testing Patterns

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_error_handling() {
        let result = parse_invalid_input();
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn test_panic_condition() {
        divide(10, 0);
    }
}
```

### Property-Based Testing
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_reversible(s in ".*") {
        let encoded = encode(&s);
        let decoded = decode(&encoded)?;
        prop_assert_eq!(&s, &decoded);
    }
}
```

## Documentation Standards

```rust
/// Processes user input and returns the result.
///
/// # Arguments
///
/// * `input` - The raw input string to process
/// * `config` - Configuration options for processing
///
/// # Returns
///
/// Returns `Ok(ProcessedData)` on success, or `Error` if processing fails.
///
/// # Errors
///
/// This function will return an error if:
/// * Input is empty
/// * Input contains invalid UTF-8
/// * Configuration is invalid
///
/// # Examples
///
/// ```
/// use mylib::{process, Config};
///
/// let config = Config::default();
/// let result = process("hello", &config)?;
/// assert_eq!(result.value, "HELLO");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn process(input: &str, config: &Config) -> Result<ProcessedData> {
    // Implementation
}
```

## Workflow Instructions

When reviewing or writing Rust code:

1. **First Pass - Compilation**
   - Run `cargo check` to ensure it compiles
   - Address all errors

2. **Second Pass - Warnings**
   - Run `cargo clippy -- -D warnings`
   - Fix ALL warnings, no exceptions
   - Run `cargo fmt` to format

3. **Third Pass - Structure**
   - Review module organization
   - Check separation of concerns
   - Verify API boundaries

4. **Fourth Pass - Safety & Performance**
   - Review error handling
   - Check for unnecessary allocations
   - Verify no unsafe code or document why needed
   - Look for optimization opportunities

5. **Fifth Pass - Documentation & Tests**
   - Ensure public APIs documented
   - Verify test coverage
   - Add examples where helpful

## Common Dependencies

```toml
[dependencies]
# Error handling
anyhow = "1.0"          # Easy error handling for applications
thiserror = "1.0"       # Error types for libraries

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Performance
rayon = "1.8"           # Data parallelism
parking_lot = "0.12"    # Faster synchronization primitives

# Utilities
itertools = "0.12"      # Iterator extensions
once_cell = "1.19"      # Lazy static initialization
regex = "1"

# Testing
proptest = "1.4"        # Property-based testing
criterion = "0.5"       # Benchmarking
```

## Quick Reference Commands

```bash
# Build and check
cargo build --release
cargo check
cargo clippy -- -D warnings
cargo fmt --check

# Testing
cargo test
cargo test --all-features
cargo test --no-default-features

# Documentation
cargo doc --open
cargo doc --no-deps

# Benchmarking
cargo bench

# Security audit
cargo audit

# Dependency management
cargo update
cargo tree
cargo outdated
```

## Examples

See [EXAMPLES.md](EXAMPLES.md) for complete code examples.
See [REFERENCE.md](REFERENCE.md) for advanced patterns and edge cases.