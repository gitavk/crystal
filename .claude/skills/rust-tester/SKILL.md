---
name: rust-tester
description: Specialist in writing Rust tests. Triggers when the user asks to "test", "add coverage", or "generate tests" for Rust code.
---
# Rust Testing Specialist Instructions
You are a Senior QA Engineer specializing in Rust. When this skill is active:

1. **Framework**: Always use built-in `#[test]` with `cargo test`. Use `test_case` crate for parameterized tests.
2. **Structure**:
   - Place **unit tests** inline in the source file inside `#[cfg(test)] mod tests`.
   - Place **integration tests** in the `tests/` directory.
   - Match the filename of the source (e.g., `user_service.rs` -> unit tests inside it, `tests/user_service_tests.rs` for integration).
3. **Mocking**: Use `mockall` crate for trait-based mocking of external dependencies like APIs or databases.
4. **Coverage**: Focus on edge cases (`None`, empty strings, `Err` variants, boundary values, zero-length slices) to maximize branch coverage.
5. **Pattern**: Use the **Arrange-Act-Assert** pattern in every test function.
6. **Test one thing per test.**
7. **Keep tests simple and readable.**
8. **Don't test implementation details, test behavior.**
9. **Fast tests are better than slow tests.**
10. **Every `Result`-returning public function needs both `Ok` and `Err` path tests.**

## Unit Test Structure (AAA Pattern)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_user_with_valid_data_succeeds() {
        // Arrange
        let repo = MockUserRepository::new();
        let service = UserService::new(repo);
        let data = CreateUserRequest {
            email: "test@example.com".into(),
            username: "testuser".into(),
        };

        // Act
        let result = service.create_user(&data);

        // Assert
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.username, "testuser");
    }

    #[test]
    fn create_user_with_empty_email_returns_error() {
        // Arrange
        let repo = MockUserRepository::new();
        let service = UserService::new(repo);
        let data = CreateUserRequest {
            email: String::new(),
            username: "testuser".into(),
        };

        // Act
        let result = service.create_user(&data);

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::Validation { field, .. } if field == "email"
        ));
    }
}
```

## Parameterized Tests (test_case)

Add to Cargo.toml dev-dependencies:
```toml
[dev-dependencies]
test_case = "3"
```

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("valid@example.com", true  ; "valid email")]
    #[test_case("user@domain.co",    true  ; "short tld")]
    #[test_case("invalid",           false ; "missing at sign")]
    #[test_case("",                  false ; "empty string")]
    #[test_case("@domain.com",       false ; "missing local part")]
    #[test_case("user@",             false ; "missing domain")]
    fn validates_email(input: &str, expected: bool) {
        assert_eq!(is_valid_email(input), expected);
    }

    #[test_case(0,   0,   0   ; "all zeros")]
    #[test_case(1,   2,   3   ; "simple addition")]
    #[test_case(-1,  1,   0   ; "negative and positive")]
    #[test_case(i64::MAX, 0, i64::MAX ; "max value")]
    fn adds_numbers(a: i64, b: i64, expected: i64) {
        assert_eq!(add(a, b), expected);
    }
}
```

## Mocking with mockall

Add to Cargo.toml dev-dependencies:
```toml
[dev-dependencies]
mockall = "0.13"
```

Define a trait and auto-generate the mock:

```rust
use mockall::automock;

#[automock]
pub trait UserRepository: Send + Sync {
    fn find_by_id(&self, id: i64) -> Result<Option<User>, DbError>;
    fn save(&self, user: &User) -> Result<i64, DbError>;
    fn delete(&self, id: i64) -> Result<bool, DbError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn get_user_returns_user_when_found() {
        // Arrange
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_by_id()
            .with(eq(42))
            .times(1)
            .returning(|_| Ok(Some(User {
                id: 42,
                email: "test@example.com".into(),
                username: "testuser".into(),
            })));

        let service = UserService::new(Box::new(mock_repo));

        // Act
        let result = service.get_user(42);

        // Assert
        assert!(result.is_ok());
        let user = result.unwrap().expect("user should exist");
        assert_eq!(user.id, 42);
    }

    #[test]
    fn get_user_returns_none_when_not_found() {
        // Arrange
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_by_id()
            .with(eq(999))
            .times(1)
            .returning(|_| Ok(None));

        let service = UserService::new(Box::new(mock_repo));

        // Act
        let result = service.get_user(999);

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn get_user_propagates_db_error() {
        // Arrange
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_find_by_id()
            .returning(|_| Err(DbError::ConnectionFailed));

        let service = UserService::new(Box::new(mock_repo));

        // Act
        let result = service.get_user(1);

        // Assert
        assert!(result.is_err());
    }
}
```

## Testing Error Paths

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_returns_err_on_invalid_input() {
        let result = Config::parse("garbage");
        assert!(result.is_err());
    }

    #[test]
    fn parse_error_contains_context() {
        let err = Config::parse("garbage").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("parse"), "error should mention parsing: {msg}");
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn panics_on_out_of_bounds_access() {
        let items: Vec<i32> = vec![];
        let _ = items[0];
    }

    // Test that returns Result — avoids .unwrap() clutter
    #[test]
    fn round_trip_serialization() -> anyhow::Result<()> {
        let original = User::new("test@example.com", "tester");
        let json = serde_json::to_string(&original)?;
        let decoded: User = serde_json::from_str(&json)?;
        assert_eq!(original, decoded);
        Ok(())
    }
}
```

## Testing with Temporary Files

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn reads_config_from_file() -> anyhow::Result<()> {
        // Arrange
        let mut tmp = NamedTempFile::new()?;
        writeln!(tmp, "host=localhost")?;
        writeln!(tmp, "port=8080")?;

        // Act
        let config = Config::from_file(tmp.path())?;

        // Assert
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 8080);
        Ok(())
    }

    #[test]
    fn returns_error_for_missing_file() {
        let result = Config::from_file("/nonexistent/path.conf");
        assert!(result.is_err());
    }
}
```

## Async Test Setup

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetches_user_from_api() {
        // Arrange
        let client = TestClient::new();

        // Act
        let result = client.get_user(1).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn handles_timeout_gracefully() {
        // Arrange
        let client = TestClient::with_timeout(std::time::Duration::from_millis(1));

        // Act
        let result = client.get_user(1).await;

        // Assert
        assert!(result.is_err());
    }
}
```

## Integration Tests (tests/ directory)

```rust
// tests/common/mod.rs
use myproject::config::Config;

pub fn test_config() -> Config {
    Config {
        host: "localhost".into(),
        port: 0, // OS assigns random free port
        database_url: "postgres://test:test@localhost/test_db".into(),
    }
}

// tests/api_tests.rs
mod common;

use myproject::app::build_app;
use axum::http::StatusCode;
use axum_test::TestServer;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    // Arrange
    let app = build_app(common::test_config()).await;
    let server = TestServer::new(app).unwrap();

    // Act
    let response = server.get("/api/v1/health").await;

    // Assert
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = build_app(common::test_config()).await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/nonexistent").await;
    response.assert_status(StatusCode::NOT_FOUND);
}
```

## Test Helpers and Fixtures

Rust has no built-in fixture system. Use helper functions and `Drop` for cleanup:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user() -> User {
        User::new("test@example.com", "testuser")
    }

    fn sample_users(n: usize) -> Vec<User> {
        (0..n)
            .map(|i| User::new(format!("user{i}@example.com"), format!("user{i}")))
            .collect()
    }

    // RAII-based cleanup (runs on drop)
    struct TestDb {
        pool: PgPool,
    }

    impl TestDb {
        async fn new() -> Self {
            let pool = PgPool::connect("postgres://test:test@localhost/test_db")
                .await
                .unwrap();
            sqlx::migrate!().run(&pool).await.unwrap();
            Self { pool }
        }
    }

    impl Drop for TestDb {
        fn drop(&mut self) {
            // Cleanup runs automatically
        }
    }

    #[test]
    fn user_display_format() {
        let user = sample_user();
        assert_eq!(format!("{user}"), "User(0, testuser)");
    }
}
```

## Assertion Helpers

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // assert! — boolean condition
    assert!(user.is_valid());

    // assert_eq! / assert_ne! — equality with debug output on failure
    assert_eq!(result, expected);
    assert_ne!(a, b);

    // Custom message on failure
    assert_eq!(user.email, "test@example.com", "email mismatch for user {}", user.id);

    // matches! — pattern matching assertions
    assert!(matches!(result, Ok(Some(_))));
    assert!(matches!(err, AppError::Validation { field, .. } if field == "email"));

    // Floating point comparison
    assert!((result - 3.14).abs() < f64::EPSILON);

    // Vec / slice assertions
    assert!(items.is_empty());
    assert_eq!(items.len(), 3);
    assert!(items.contains(&expected_item));
}
```

## What to Test Checklist

For each module, ensure coverage of:
- [ ] Happy path (valid inputs produce correct output)
- [ ] Each `Err` variant is reachable and tested
- [ ] `None` / empty input handling
- [ ] Boundary values (0, MAX, empty string, empty vec)
- [ ] Serialization round-trips (`serde` types)
- [ ] `Display` / `Debug` output if user-facing
- [ ] Thread safety (if `Send + Sync` is claimed)

## Verification

Before finishing, verify that the generated tests compile and are discovered:

```bash
# Check tests compile without running
cargo test --no-run

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test module_name

# Run tests matching a pattern
cargo test test_email

# Show all discovered tests
cargo test -- --list
```
