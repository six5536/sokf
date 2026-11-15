# JMESPath compliance suite

This directory holds the official JMESPath compliance tests. The tests come
from the `jmespath` crate (MIT license), which takes them from the JMESPath
project's `jmespath.test` repository (MIT license). The unit test in
`src/query/compliance.rs` runs the evaluator in
`crates/lib/sokf-core/src/query/` against every case in these files. It skips
the `bench` cases, which are timing fixtures with no assertion.

The files are verbatim copies: don't edit them here.
