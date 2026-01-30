# Rust Expert - Advanced Reference

## Advanced Patterns

### Newtype Pattern for Type Safety

```rust
use std::fmt;

// Prevents mixing up different ID types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(u64);

impl UserId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User({})", self.0)
    }
}

// Now this won't compile:
// let user_id = UserId::new(1);
// let order_id = OrderId::new(1);
// if user_id == order_id { } // Compile error!
```

### Typestate Pattern

```rust
// Encode state in the type system
pub struct Connection<State> {
    state: State,
}

pub struct Disconnected;
pub struct Connected {
    stream: TcpStream,
}

impl Connection<Disconnected> {
    pub fn new() -> Self {
        Self {
            state: Disconnected,
        }
    }
    
    pub fn connect(self, addr: &str) -> Result<Connection<Connected>> {
        let stream = TcpStream::connect(addr)?;
        Ok(Connection {
            state: Connected { stream },
        })
    }
}

impl Connection<Connected> {
    // Only available when connected
    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        self.state.stream.write_all(data)?;
        Ok(())
    }
    
    pub fn disconnect(self) -> Connection<Disconnected> {
        Connection {
            state: Disconnected,
        }
    }
}

// Usage:
// let conn = Connection::new();
// conn.send(b"data"); // Compile error - can't send while disconnected!
// let conn = conn.connect("127.0.0.1:8080")?;
// conn.send(b"data"); // OK
```

### Phantom Data for Zero-Cost Abstractions

```rust
use std::marker::PhantomData;

pub struct Metric<T> {
    value: f64,
    _phantom: PhantomData<T>,
}

pub struct Celsius;
pub struct Fahrenheit;

impl Metric<Celsius> {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }
    
    pub fn to_fahrenheit(self) -> Metric<Fahrenheit> {
        Metric {
            value: self.value * 9.0 / 5.0 + 32.0,
            _phantom: PhantomData,
        }
    }
}

impl<T> Metric<T> {
    pub fn value(&self) -> f64 {
        self.value
    }
}

// Prevents accidental unit mixing at compile time
```

### Sealed Traits

```rust
mod sealed {
    pub trait Sealed {}
}

// Public trait that can't be implemented outside this crate
pub trait MyTrait: sealed::Sealed {
    fn do_something(&self);
}

// Only types we define can implement MyTrait
impl sealed::Sealed for MyType {}
impl MyTrait for MyType {
    fn do_something(&self) {
        // implementation
    }
}
```

## Advanced Error Handling

### Custom Error with Context Chain

```rust
use std::fmt;

#[derive(Debug)]
pub struct ErrorContext {
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ErrorContext {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }
    
    pub fn with_source(
        mut self,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        
        if let Some(ref source) = self.source {
            write!(f, ": {}", source)?;
        }
        
        Ok(())
    }
}

impl std::error::Error for ErrorContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}

// Usage:
fn process_file(path: &Path) -> Result<(), ErrorContext> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| {
            ErrorContext::new(format!("Failed to read file: {}", path.display()))
                .with_source(e)
        })?;
    
    Ok(())
}
```

## Memory Management Patterns

### Arena Allocation

```rust
use typed_arena::Arena;

pub struct Parser<'arena> {
    arena: &'arena Arena<Node>,
}

pub struct Node {
    value: String,
    children: Vec<*const Node>,
}

impl<'arena> Parser<'arena> {
    pub fn new(arena: &'arena Arena<Node>) -> Self {
        Self { arena }
    }
    
    pub fn parse(&self, input: &str) -> &'arena Node {
        let node = self.arena.alloc(Node {
            value: input.to_string(),
            children: Vec::new(),
        });
        
        // All nodes allocated in the same arena
        // Freed all at once when arena drops
        node
    }
}
```

### Object Pool

```rust
use std::sync::{Arc, Mutex};

pub struct Pool<T> {
    objects: Arc<Mutex<Vec<T>>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T: Send + 'static> Pool<T> {
    pub fn new<F>(factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            objects: Arc::new(Mutex::new(Vec::new())),
            factory: Box::new(factory),
        }
    }
    
    pub fn acquire(&self) -> PoolGuard<T> {
        let obj = self.objects.lock().unwrap().pop()
            .unwrap_or_else(|| (self.factory)());
        
        PoolGuard {
            object: Some(obj),
            pool: Arc::clone(&self.objects),
        }
    }
}

pub struct PoolGuard<T> {
    object: Option<T>,
    pool: Arc<Mutex<Vec<T>>>,
}

impl<T> Drop for PoolGuard<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.object.take() {
            self.pool.lock().unwrap().push(obj);
        }
    }
}

impl<T> std::ops::Deref for PoolGuard<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.object.as_ref().unwrap()
    }
}

impl<T> std::ops::DerefMut for PoolGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object.as_mut().unwrap()
    }
}
```

## Concurrency Patterns

### Message Passing with Channels

```rust
use tokio::sync::mpsc;
use std::time::Duration;

pub struct Worker {
    name: String,
}

impl Worker {
    pub async fn run(
        mut self,
        mut rx: mpsc::Receiver<Task>,
        tx: mpsc::Sender<Result<String>>,
    ) {
        while let Some(task) = rx.recv().await {
            let result = self.process_task(task).await;
            let _ = tx.send(result).await;
        }
    }
    
    async fn process_task(&mut self, task: Task) -> Result<String> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(format!("{} processed task: {}", self.name, task.id))
    }
}

pub struct Task {
    id: u64,
    data: String,
}

// Usage:
async fn example() {
    let (task_tx, task_rx) = mpsc::channel(100);
    let (result_tx, mut result_rx) = mpsc::channel(100);
    
    // Spawn worker
    let worker = Worker { name: "Worker-1".to_string() };
    tokio::spawn(async move {
        worker.run(task_rx, result_tx).await;
    });
    
    // Send tasks
    for i in 0..10 {
        task_tx.send(Task {
            id: i,
            data: format!("data-{}", i),
        }).await.unwrap();
    }
    
    // Collect results
    drop(task_tx); // Close channel
    while let Some(result) = result_rx.recv().await {
        println!("{:?}", result);
    }
}
```

### Actor Pattern

```rust
use tokio::sync::{mpsc, oneshot};

pub enum Message {
    Get { respond_to: oneshot::Sender<u64> },
    Increment,
    Reset,
}

pub struct Counter {
    value: u64,
    receiver: mpsc::Receiver<Message>,
}

impl Counter {
    pub fn new() -> (Self, CounterHandle) {
        let (sender, receiver) = mpsc::channel(100);
        
        let actor = Counter {
            value: 0,
            receiver,
        };
        
        let handle = CounterHandle { sender };
        
        (actor, handle)
    }
    
    pub async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                Message::Get { respond_to } => {
                    let _ = respond_to.send(self.value);
                }
                Message::Increment => {
                    self.value += 1;
                }
                Message::Reset => {
                    self.value = 0;
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct CounterHandle {
    sender: mpsc::Sender<Message>,
}

impl CounterHandle {
    pub async fn get(&self) -> u64 {
        let (tx, rx) = oneshot::channel();
        let _ = self.sender.send(Message::Get { respond_to: tx }).await;
        rx.await.unwrap_or(0)
    }
    
    pub async fn increment(&self) {
        let _ = self.sender.send(Message::Increment).await;
    }
    
    pub async fn reset(&self) {
        let _ = self.sender.send(Message::Reset).await;
    }
}

// Usage:
async fn example() {
    let (actor, handle) = Counter::new();
    
    tokio::spawn(async move {
        actor.run().await;
    });
    
    handle.increment().await;
    handle.increment().await;
    
    let value = handle.get().await;
    assert_eq!(value, 2);
}
```

## Unsafe Code Guidelines

### FFI Wrapper

```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" {
    fn external_function(input: *const c_char) -> *mut c_char;
    fn free_string(ptr: *mut c_char);
}

pub struct SafeWrapper;

impl SafeWrapper {
    /// SAFETY: This function is safe as long as:
    /// - external_function returns a valid null-terminated string
    /// - The returned pointer must be freed with free_string
    pub fn call_external(input: &str) -> Result<String, NulError> {
        // Convert Rust string to C string
        let c_input = CString::new(input)?;
        
        // SAFETY: c_input is valid and null-terminated
        let result_ptr = unsafe {
            external_function(c_input.as_ptr())
        };
        
        if result_ptr.is_null() {
            return Err(NulError::new());
        }
        
        // SAFETY: 
        // - result_ptr is non-null (checked above)
        // - external_function guarantees null-terminated string
        // - We own this pointer and will free it
        let result = unsafe {
            let c_str = CStr::from_ptr(result_ptr);
            let rust_str = c_str.to_string_lossy().into_owned();
            
            // Free the C string
            free_string(result_ptr);
            
            rust_str
        };
        
        Ok(result)
    }
}
```

### Custom Smart Pointer

```rust
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

pub struct MyBox<T> {
    ptr: NonNull<T>,
}

impl<T> MyBox<T> {
    pub fn new(value: T) -> Self {
        let boxed = Box::new(value);
        let ptr = Box::into_raw(boxed);
        
        Self {
            // SAFETY: Box::into_raw never returns null
            ptr: unsafe { NonNull::new_unchecked(ptr) },
        }
    }
}

impl<T> Deref for MyBox<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        // SAFETY: ptr is valid and aligned
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for MyBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: ptr is valid, aligned, and we have exclusive access
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for MyBox<T> {
    fn drop(&mut self) {
        // SAFETY: ptr was allocated with Box and is valid
        unsafe {
            let _ = Box::from_raw(self.ptr.as_ptr());
        }
    }
}

// SAFETY: MyBox owns its data
unsafe impl<T: Send> Send for MyBox<T> {}
// SAFETY: Access to data is synchronized through &/&mut
unsafe impl<T: Sync> Sync for MyBox<T> {}
```

## Macro Patterns

### Declarative Macro for DSL

```rust
macro_rules! html {
    // Base case: text node
    ($text:expr) => {
        Element::Text($text.to_string())
    };
    
    // Element with attributes and children
    ($tag:ident { $($attr:ident: $val:expr),* } [ $($child:tt)* ]) => {
        Element::Node {
            tag: stringify!($tag).to_string(),
            attributes: vec![
                $((stringify!($attr).to_string(), $val.to_string())),*
            ],
            children: vec![
                $(html!($child)),*
            ],
        }
    };
}

pub enum Element {
    Text(String),
    Node {
        tag: String,
        attributes: Vec<(String, String)>,
        children: Vec<Element>,
    },
}

// Usage:
let page = html! {
    div { class: "container" } [
        h1 { } [ "Hello World" ]
        p { id: "intro" } [ "Welcome to my page" ]
    ]
};
```

### Procedural Macro Example

```rust
// In a separate proc-macro crate

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder_name = format!("{}Builder", name);
    let builder_ident = syn::Ident::new(&builder_name, name.span());
    
    let fields = match input.data {
        syn::Data::Struct(ref data) => match data.fields {
            syn::Fields::Named(ref fields) => &fields.named,
            _ => panic!("Builder only works with named fields"),
        },
        _ => panic!("Builder only works with structs"),
    };
    
    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! { #name: Option<#ty> }
    });
    
    let builder_methods = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            pub fn #name(mut self, #name: #ty) -> Self {
                self.#name = Some(#name);
                self
            }
        }
    });
    
    let build_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name: self.#name.ok_or(concat!("Field not set: ", stringify!(#name)))?
        }
    });
    
    let expanded = quote! {
        impl #name {
            pub fn builder() -> #builder_ident {
                #builder_ident::default()
            }
        }
        
        #[derive(Default)]
        pub struct #builder_ident {
            #(#builder_fields),*
        }
        
        impl #builder_ident {
            #(#builder_methods)*
            
            pub fn build(self) -> Result<#name, &'static str> {
                Ok(#name {
                    #(#build_fields),*
                })
            }
        }
    };
    
    TokenStream::from(expanded)
}
```

## Testing Strategies

### Property-Based Testing with Proptest

```rust
use proptest::prelude::*;

fn encode(s: &str) -> String {
    s.chars().map(|c| ((c as u8) + 1) as char).collect()
}

fn decode(s: &str) -> String {
    s.chars().map(|c| ((c as u8) - 1) as char).collect()
}

proptest! {
    #[test]
    fn test_encode_decode_roundtrip(s in "\\PC*") {
        let encoded = encode(&s);
        let decoded = decode(&encoded);
        prop_assert_eq!(&s, &decoded);
    }
    
    #[test]
    fn test_encode_changes_all_chars(s in "\\PC+") {
        let encoded = encode(&s);
        prop_assert_ne!(&s, &encoded);
    }
}
```

### Fuzzing with cargo-fuzz

```rust
// fuzz/fuzz_targets/parse.rs

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = my_crate::parse(s);
    }
});
```

## Performance Optimization

### SIMD Operations

```rust
use std::simd::{Simd, SimdPartialOrd};

pub fn simd_max(slice: &[f32]) -> f32 {
    const LANES: usize = 8;
    
    if slice.len() < LANES {
        return slice.iter().fold(f32::MIN, |a, &b| a.max(b));
    }
    
    let mut max_vec = Simd::from_array([f32::MIN; LANES]);
    
    let chunks = slice.chunks_exact(LANES);
    let remainder = chunks.remainder();
    
    for chunk in chunks {
        let vec = Simd::from_slice(chunk);
        max_vec = max_vec.simd_max(vec);
    }
    
    let mut max = max_vec.reduce_max();
    
    for &val in remainder {
        max = max.max(val);
    }
    
    max
}
```

### Compile-Time Computation

```rust
const fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0;
            let mut b = 1;
            let mut i = 2;
            
            while i <= n {
                let temp = a + b;
                a = b;
                b = temp;
                i += 1;
            }
            
            b
        }
    }
}

// Computed at compile time!
const FIB_10: u64 = fibonacci(10);
```

## Edge Cases & Gotchas

### Integer Overflow

```rust
// Always handle potential overflow
pub fn safe_multiply(a: u32, b: u32) -> Option<u32> {
    a.checked_mul(b)
}

// Or use wrapping semantics explicitly
pub fn wrapping_add(a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}
```

### Lifetime Elision Pitfalls

```rust
// This won't compile - ambiguous lifetimes
// fn bad<'a, 'b>(x: &'a str, y: &'b str) -> &str {
//     if x.len() > y.len() { x } else { y }
// }

// Correct - explicit lifetime relationship
fn good<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

### Trait Object Lifetime Bounds

```rust
// Won't compile - missing lifetime bound
// fn bad(v: Vec<Box<dyn Display>>) -> Box<dyn Display> {
//     v.into_iter().next().unwrap()
// }

// Correct
fn good(v: Vec<Box<dyn Display + 'static>>) -> Box<dyn Display + 'static> {
    v.into_iter().next().unwrap()
}
```