//! This module sets the number of cases that a property test runs, as the
//! Tests section of CONTRIBUTING describes.

use proptest::test_runner::Config;

/// `cases` returns a proptest configuration that runs `PROPTEST_CASES` cases
/// when that variable holds a number, and `n` cases otherwise. With the
/// variable, a single test run can use a different number of cases without
/// anyone changing the code. `Config::with_cases` alone can't do that, because
/// it ignores the variable.
pub fn cases(n: u32) -> Config {
    let n = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(n);
    Config::with_cases(n)
}
