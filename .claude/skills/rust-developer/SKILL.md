---
name: rust-developer
description: Rust-specific coding standards, idioms, and best practices. Reference guide for ownership, error handling, traits, concurrency, testing, and common patterns.
---
# Rust Guidelines

Rust-specific coding standards and best practices.

## Style & Standards

- Follow official **Rust API Guidelines** and **Rust Style Guide**
- Use `rustfmt` for automatic formatting (never argue about style)
- Use `snake_case` for functions, variables, modules; `PascalCase` for types, traits; `SCREAMING_SNAKE_CASE` for constants
- Use `clippy` with pedantic lints enabled
- Keep functions focused and under 50 lines
- Prefer expressing invariants through the type system over runtime checks
- Use `unsafe` only when absolutely necessary and always document why

## Best Practices

- **Own data by default**, borrow only when needed
- Prefer `&str` over `&String`, `&[T]` over `&Vec<T>` in function parameters
- Use `impl Trait` in argument position for simple generics
- Return concrete types, accept generic inputs
- Derive standard traits: `Debug`, `Clone`, `PartialEq` on most types
- Use `#[must_use]` on functions whose return values shouldn't be ignored
- Prefer iterators over index-based loops
- Avoid `.unwrap()` and `.expect()` in library code — return `Result`
- Use `.expect("reason")` over `.unwrap()` in binaries when panicking is acceptable
- Use `todo!()` as a placeholder, never ship it
- Prefer `to_owned()` or `to_string()` over `.clone()` for `&str -> String`
- Keep `Cargo.toml` dependencies sorted and version-constrained
- Run `cargo clippy`, `cargo fmt`, and `cargo test` before committing

## Project Layout

```
project/
├── src/
│   ├── main.rs              # Binary entry point (thin)
│   ├── lib.rs               # Library root — re-exports public API
│   ├── config.rs            # Configuration
│   ├── error.rs             # Error types
│   ├── domain/              # Core types and business rules
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   └── order.rs
│   ├── services/            # Business logic orchestration
│   │   ├── mod.rs
│   │   └── user_service.rs
│   ├── db/                  # Persistence layer
│   │   ├── mod.rs
│   │   └── user_repo.rs
│   └── api/                 # HTTP / CLI / gRPC surface
│       ├── mod.rs
│       └── handlers.rs
├── tests/                   # Integration tests
│   ├── common/
│   │   └── mod.rs
│   └── api_tests.rs
├── benches/                 # Benchmarks (criterion)
│   └── bench_main.rs
├── examples/                # Runnable examples
│   └── basic.rs
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── Makefile
└── README.md
```

### Directory Purpose
- `src/` — all production code; `lib.rs` exposes the public API
- `tests/` — integration tests (each file is a separate crate)
- `benches/` — criterion benchmarks
- `examples/` — runnable examples (`cargo run --example basic`)

## Ownership & Borrowing

```rust
// Ownership moves by default for non-Copy types
let s1 = String::from("hello");
let s2 = s1; // s1 is moved, no longer usable

// Borrowing — immutable references (multiple allowed)
fn print_len(s: &str) {
    println!("{}", s.len());
}

// Mutable reference (exclusive — only one at a time)
fn append_world(s: &mut String) {
    s.push_str(" world");
}

// Clone when you genuinely need a copy
let s3 = s2.clone();

// Accept the most general borrow in parameters
fn process(data: &[u8]) { /* works with Vec<u8>, arrays, slices */ }
fn greet(name: &str) { /* works with String, &str, Cow<str> */ }
```

## Lifetimes

```rust
// Compiler infers most lifetimes — annotate only when required
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

// Struct holding a reference must declare lifetime
struct Excerpt<'a> {
    text: &'a str,
}

impl<'a> Excerpt<'a> {
    fn new(text: &'a str) -> Self {
        Self { text }
    }
}

// 'static — lives for entire program (string literals, leaked allocations)
fn returns_static() -> &'static str {
    "I live forever"
}
```

## Error Handling

```rust
use thiserror::Error;

// Define domain errors with thiserror
#[derive(Debug, Error)]
pub enum AppError {
    #[error("user not found: {0}")]
    UserNotFound(i64),

    #[error("validation failed on field `{field}`: {message}")]
    Validation { field: String, message: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

// Use Result<T, AppError> throughout library code
fn find_user(id: i64) -> Result<User, AppError> {
    let user = db::query_user(id)?;  // ? converts via From
    user.ok_or(AppError::UserNotFound(id))
}

// Use anyhow::Result in binary/application code for convenience
fn main() -> anyhow::Result<()> {
    let config = load_config()
        .context("failed to load configuration")?;
    run(config)?;
    Ok(())
}

// Converting between error types
impl From<AppError> for StatusCode {
    fn from(err: AppError) -> Self {
        match err {
            AppError::UserNotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation { .. } => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

## Structs and Enums

```rust
// Struct with derived traits
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// Constructor (Rust convention: associated function named `new`)
impl User {
    pub fn new(email: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            id: 0,
            email: email.into(),
            username: username.into(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.email.is_empty() && !self.username.is_empty()
    }
}

// Display trait instead of toString
impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User({}, {})", self.id, self.username)
    }
}

// Enums — use for closed sets of variants
#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Pending,
    Processing { started_at: chrono::DateTime<chrono::Utc> },
    Shipped(TrackingInfo),
    Delivered,
    Cancelled(String), // reason
}

// Match must be exhaustive
fn status_label(status: &OrderStatus) -> &str {
    match status {
        OrderStatus::Pending => "pending",
        OrderStatus::Processing { .. } => "processing",
        OrderStatus::Shipped(_) => "shipped",
        OrderStatus::Delivered => "delivered",
        OrderStatus::Cancelled(_) => "cancelled",
    }
}
```

## Traits

```rust
// Define small, focused traits
pub trait Repository {
    type Item;
    type Error;

    fn get(&self, id: i64) -> Result<Option<Self::Item>, Self::Error>;
    fn create(&self, item: &Self::Item) -> Result<i64, Self::Error>;
    fn delete(&self, id: i64) -> Result<bool, Self::Error>;
}

// Accept traits as parameters (static dispatch)
fn save_user(repo: &impl Repository<Item = User, Error = AppError>, user: &User) -> Result<i64, AppError> {
    repo.create(user)
}

// Dynamic dispatch when needed (trait objects)
fn notify_all(listeners: &[Box<dyn EventListener>]) {
    for listener in listeners {
        listener.on_event(&event);
    }
}

// Default implementations
pub trait Summary {
    fn title(&self) -> &str;

    fn summary(&self) -> String {
        format!("{}...", &self.title()[..20.min(self.title().len())])
    }
}

// Extension traits for adding methods to foreign types
pub trait StringExt {
    fn truncate_to(&self, max: usize) -> &str;
}

impl StringExt for str {
    fn truncate_to(&self, max: usize) -> &str {
        if self.len() <= max {
            self
        } else {
            &self[..self.floor_char_boundary(max)]
        }
    }
}

// Trait bounds
fn process<T: Serialize + Send + Sync>(data: T) { /* ... */ }

// Where clause for complex bounds
fn merge<K, V>(a: HashMap<K, V>, b: HashMap<K, V>) -> HashMap<K, V>
where
    K: Eq + Hash,
    V: Default,
{
    /* ... */
}
```

## Iterators

```rust
// Prefer iterators over indexing
let names: Vec<String> = users
    .iter()
    .filter(|u| u.is_active)
    .map(|u| u.name.clone())
    .collect();

// Chaining
let total: f64 = orders
    .iter()
    .filter(|o| o.status == Status::Completed)
    .flat_map(|o| &o.items)
    .map(|item| item.price * item.quantity as f64)
    .sum();

// Consuming iterator
for user in users.into_iter() {
    process(user); // takes ownership
}

// Enumerate
for (i, item) in items.iter().enumerate() {
    println!("{i}: {item}");
}

// Creating iterators from custom types
impl IntoIterator for Playlist {
    type Item = Song;
    type IntoIter = std::vec::IntoIter<Song>;

    fn into_iter(self) -> Self::IntoIter {
        self.songs.into_iter()
    }
}

// Useful iterator adaptors
let first_admin = users.iter().find(|u| u.role == Role::Admin);
let any_active = users.iter().any(|u| u.is_active);
let all_valid = users.iter().all(|u| u.is_valid());
let chunks: Vec<&[User]> = users.chunks(10).collect();
let (admins, others): (Vec<_>, Vec<_>) = users.into_iter().partition(|u| u.role == Role::Admin);
```

## Concurrency

### Async / Await (Tokio)
```rust
use tokio::task;

// Async function
async fn fetch_user(client: &reqwest::Client, id: i64) -> Result<User, reqwest::Error> {
    client.get(format!("/users/{id}"))
        .send()
        .await?
        .json()
        .await
}

// Run tasks concurrently
async fn fetch_all(ids: &[i64]) -> Vec<Result<User, Error>> {
    let client = reqwest::Client::new();
    let futures: Vec<_> = ids
        .iter()
        .map(|&id| fetch_user(&client, id))
        .collect();
    futures::future::join_all(futures).await
}

// Spawn background task
let handle = task::spawn(async move {
    heavy_computation().await
});
let result = handle.await?;

// Select first completed future
tokio::select! {
    result = future_a => handle_a(result),
    result = future_b => handle_b(result),
    _ = tokio::time::sleep(Duration::from_secs(5)) => return Err(timeout()),
}
```

### Threads & Channels
```rust
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

// Scoped threads (no 'static requirement)
thread::scope(|s| {
    let handle = s.spawn(|| {
        compute_something()
    });
    let result = handle.join().unwrap();
});

// Channels
let (tx, rx) = mpsc::channel();
let tx2 = tx.clone();

thread::spawn(move || { tx.send(1).unwrap(); });
thread::spawn(move || { tx2.send(2).unwrap(); });

for received in rx {
    println!("{received}");
}

// Shared state with Arc<Mutex<T>>
let counter = Arc::new(Mutex::new(0));

thread::scope(|s| {
    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        s.spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        });
    }
});

// Prefer RwLock when reads vastly outnumber writes
let config = Arc::new(RwLock::new(Config::default()));
```

## Testing

```rust
// Unit tests — inside the module they test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_valid_user() {
        let user = User::new("test@example.com", "testuser");
        assert!(user.is_valid());
        assert_eq!(user.email, "test@example.com");
    }

    #[test]
    fn rejects_empty_email() {
        let user = User::new("", "testuser");
        assert!(!user.is_valid());
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn panics_on_bad_index() {
        let v: Vec<i32> = vec![];
        let _ = v[0];
    }

    // Test that returns Result
    #[test]
    fn parses_config() -> anyhow::Result<()> {
        let config = Config::from_str("key=value")?;
        assert_eq!(config.get("key"), Some("value"));
        Ok(())
    }
}

// Parameterized tests with test_case crate
#[cfg(test)]
mod tests {
    use test_case::test_case;

    #[test_case("valid@example.com", true  ; "valid email")]
    #[test_case("invalid",           false ; "missing @")]
    #[test_case("",                   false ; "empty string")]
    fn validates_email(input: &str, expected: bool) {
        assert_eq!(is_valid_email(input), expected);
    }
}

// Async tests with tokio
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn fetches_data() {
        let result = fetch_data().await.unwrap();
        assert!(!result.is_empty());
    }
}

// Integration tests — in tests/ directory
// tests/api_tests.rs
use myproject::api;

#[test]
fn health_endpoint_returns_ok() {
    let app = api::build_test_app();
    let response = app.get("/health").send();
    assert_eq!(response.status(), 200);
}

// Test fixtures and helpers
fn sample_user() -> User {
    User::new("test@example.com", "testuser")
}

// Benchmarks (criterion)
// benches/bench_main.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_config", |b| {
        b.iter(|| parse_config(black_box(INPUT)))
    });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);

// Doc tests — run with cargo test
/// Adds two numbers together.
///
/// ```
/// use mylib::add;
/// assert_eq!(add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## Common Patterns

### Builder Pattern
```rust
#[derive(Debug)]
pub struct ServerConfig {
    host: String,
    port: u16,
    workers: usize,
}

#[derive(Default)]
pub struct ServerConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    workers: Option<usize>,
}

impl ServerConfigBuilder {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn workers(mut self, n: usize) -> Self {
        self.workers = Some(n);
        self
    }

    pub fn build(self) -> Result<ServerConfig, &'static str> {
        Ok(ServerConfig {
            host: self.host.unwrap_or_else(|| "0.0.0.0".into()),
            port: self.port.ok_or("port is required")?,
            workers: self.workers.unwrap_or(num_cpus::get()),
        })
    }
}

// Usage
let config = ServerConfigBuilder::default()
    .host("127.0.0.1")
    .port(8080)
    .workers(4)
    .build()?;
```

### Newtype Pattern
```rust
// Wrap primitives for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(i64);

impl UserId {
    pub fn new(id: i64) -> Self { Self(id) }
    pub fn inner(self) -> i64 { self.0 }
}

// Compiler prevents mixing UserId and OrderId — no runtime cost
fn get_order(user_id: UserId, order_id: OrderId) -> Result<Order, AppError> { /* ... */ }
```

### Typestate Pattern
```rust
// Encode state in types so invalid transitions don't compile
pub struct Request<S: State> {
    url: String,
    _state: std::marker::PhantomData<S>,
}

pub struct NotSent;
pub struct Sent;

pub trait State {}
impl State for NotSent {}
impl State for Sent {}

impl Request<NotSent> {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), _state: std::marker::PhantomData }
    }

    pub fn send(self) -> Request<Sent> {
        // actually send the request...
        Request { url: self.url, _state: std::marker::PhantomData }
    }
}

impl Request<Sent> {
    pub fn body(&self) -> &str {
        // only available after sending
        "response body"
    }
}

// req.body() won't compile — type system enforces ordering
// let req = Request::new("https://example.com");
// req.body(); // ERROR: method not found for Request<NotSent>
```

### From / Into Conversions
```rust
// Implement From for ergonomic conversions
impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            username: row.username,
            created_at: row.created_at,
        }
    }
}

// Into is automatically available when From is implemented
let user: User = db_row.into();

// TryFrom for fallible conversions
impl TryFrom<&str> for Email {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.contains('@') {
            Ok(Self(value.to_owned()))
        } else {
            Err(ValidationError::new("email", "must contain @"))
        }
    }
}

let email = Email::try_from("user@example.com")?;
```

### Smart Pointers Summary
```rust
Box<T>      // Heap allocation, single owner
Rc<T>       // Reference counted, single thread
Arc<T>      // Atomic reference counted, thread safe
Cow<'a, T>  // Clone-on-write — borrows when possible, clones when mutated

// Cow example — avoid allocations when not needed
fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))
    } else {
        Cow::Borrowed(input)
    }
}
```

## Tools

```bash
# Format code
cargo fmt

# Lint (pedantic)
cargo clippy --all-targets -- -D warnings

# Test
cargo test
cargo test -- --nocapture          # show println output
cargo test module_name             # test specific module
cargo test -- --test-threads=1     # sequential tests

# Coverage (requires cargo-llvm-cov)
cargo llvm-cov --html

# Benchmarks (requires criterion in dev-dependencies)
cargo bench

# Check without building (fast feedback)
cargo check

# Build optimized release
cargo build --release

# Run examples
cargo run --example basic

# Dependency audit (requires cargo-audit)
cargo audit

# Dependency tree
cargo tree

# Show binary size contributors (requires cargo-bloat)
cargo bloat --release -n 20

# Expand macros for debugging
cargo expand             # requires cargo-expand

# Test with sanitizers (nightly)
RUSTFLAGS="-Zsanitizer=address" cargo +nightly test

# Miri — detect undefined behavior (nightly)
cargo +nightly miri test
```

## Anti-Patterns to Avoid

```rust
// ❌ Using .unwrap() in library code
let value = map.get("key").unwrap(); // BAD — panics at runtime

// ✅ Return Result or use pattern matching
let value = map.get("key").ok_or(AppError::KeyNotFound("key"))?;

// ❌ Cloning to satisfy the borrow checker
let data = self.items.clone();
let result = process(&data); // BAD — unnecessary allocation

// ✅ Restructure to avoid the conflict
let result = process(&self.items);

// ❌ String for everything
fn set_status(status: String) { /* ... */ } // BAD — any string accepted

// ✅ Use enums for closed sets
fn set_status(status: OrderStatus) { /* ... */ } // GOOD — type-safe

// ❌ Ignoring Results
let _ = file.write_all(data); // BAD — silently drops error

// ✅ Handle or propagate
file.write_all(data)?; // GOOD

// ❌ &String, &Vec<T>, &Box<T> in parameters
fn count(items: &Vec<String>) { /* ... */ } // BAD — overly specific

// ✅ Accept slices
fn count(items: &[String]) { /* ... */ }  // GOOD — works with Vec, array, slice
fn greet(name: &str) { /* ... */ }        // GOOD — works with String, &str

// ❌ Blocking inside async context
async fn bad() {
    std::thread::sleep(Duration::from_secs(1)); // BAD — blocks the executor
}

// ✅ Use async-aware sleep
async fn good() {
    tokio::time::sleep(Duration::from_secs(1)).await; // GOOD
}

// ❌ Arc<Mutex<Vec<T>>> everywhere
let shared = Arc::new(Mutex::new(vec![])); // BAD if message passing works

// ✅ Prefer channels for communication
let (tx, rx) = mpsc::channel();
tx.send(item)?;

// ❌ Needless collect before iteration
let filtered: Vec<_> = items.iter().filter(|x| x.is_ok()).collect();
for item in filtered { /* ... */ } // BAD — intermediate allocation

// ✅ Chain iterators lazily
for item in items.iter().filter(|x| x.is_ok()) { /* ... */ } // GOOD

// ❌ Manual index loops
for i in 0..items.len() {
    process(&items[i]); // BAD — bounds check on every access
}

// ✅ Use iterators
for item in &items {
    process(item); // GOOD — no bounds checks, idiomatic
}

// ❌ Large enum variants causing size bloat
enum Message {
    Small(u8),
    Large([u8; 4096]), // BAD — all variants are 4096+ bytes
}

// ✅ Box the large variant
enum Message {
    Small(u8),
    Large(Box<[u8; 4096]>), // GOOD — enum stays small
}
```
