# sqliterg
A SQLite remote gateway - query SQLite via HTTP

This is a rewrite of [ws4sqlite](https://github.com/proofrock/ws4sqlite) in Rust, for some reasons:

- To see if it's faster
- To reduce binary size
- To correct some small design flaws
- To learn Rust 😜

But mainly, to be able to compile against an existing SQLite, not (only) embedding one. [rusqlite](https://docs.rs/rusqlite/latest/rusqlite/) has this feature, and I'd like to explore it.

It doesn't hurt that preliminary benchmarks seem to indicate that it's 30-50% faster and uses 1/5th of the memory.

The code is Rust, the tests are in Go, the profiler in Java.

No ETA. Don't hold your breath.
