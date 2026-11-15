//! This module sets the number of cases that a property test runs, as the
//! Tests section of CONTRIBUTING.md describes.

use proptest::test_runner::Config;

/// `cases` returns the configuration of a property test. The test runs as
/// many cases as `PROPTEST_CASES` holds when that variable is a number, and
/// `n` cases otherwise. As a result, a run can test fewer or more cases than
/// the code sets, whereas `Config::with_cases` on its own ignores the variable.
pub fn cases(n: u32) -> Config {
    let n = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(n);
    Config::with_cases(n)
}
