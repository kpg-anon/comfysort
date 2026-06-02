//! comfysort engine: pure Rust, no Tauri imports.
//!
//! Filesystem mutation lives here (in `operations`); the Tauri command layer is
//! the only bridge to the frontend. Safety invariants (journal-before-mutate,
//! soft-delete, collision-safe rename, cross-volume copy→verify→delete,
//! multi-step undo) are enforced in this module, not in command glue.

pub mod destinations;
pub mod domain;
pub mod media;
pub mod operations;
pub mod session;
