//! This module sets the number of cases that a property test runs, as the
//! Tests section of CONTRIBUTING.md describes.

use proptest::test_runner::Config;

/// `cases` returns the configuration of a property test. When
/// `PROPTEST_CASES` holds a number, the test runs that many cases. Otherwise
/// it runs `n`. So a developer can run more or fewer cases without changing
/// the code.
/// `Config::with_cases` on its own would ignore the variable.
pub fn cases(n: u32) -> Config {
    let n = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(n);
    Config::with_cases(n)
}
