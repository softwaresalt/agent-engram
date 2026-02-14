//! Business logic services for the T-Mem daemon.
//!
//! Each service module contains stateless free functions that accept
//! dependencies as parameters. Modules: connection lifecycle management,
//! hydration/dehydration of `.tmem/` files, embedding generation, and
//! hybrid search.

#![allow(dead_code)]

pub mod connection;
pub mod dehydration;
pub mod embedding;
pub mod hydration;
pub mod search;
