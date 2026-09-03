//! Structural stub for the Engram out-of-process indexing supervisor.
//!
//! Plan unit F12a: this crate exists solely so the workspace gate can run
//! F12's RED harness at the moment that harness is declared. It contains no
//! supervisor logic and does not depend on the `engram` crate or any of
//! F07-F10; that arrives later with plan unit F12.
#![forbid(unsafe_code)]

fn main() {}
