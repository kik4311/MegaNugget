//! Orchestration of public, third-party jailbreak components for user-owned
//! research devices. This crate contains **no exploit code**: it holds an
//! honestly-labelled compatibility matrix and verifies/downloads artifacts
//! of the upstream projects unchanged, so a human can act on them.

pub mod matrix;
pub mod releases;
