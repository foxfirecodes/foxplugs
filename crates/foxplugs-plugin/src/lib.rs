//! Shared plugin-integration helpers for Foxplugs plugins.
//!
//! This crate is intentionally minimal for now. Move NIH-plug-oriented helpers
//! here only when a second plugin needs the same parameter builders, metadata
//! conventions, validation utilities, or other host-integration glue.
//!
//! Design constraints:
//!
//! - do not move Foxcrush-specific parameter IDs or sound-design choices here;
//! - keep real-time audio helpers out of this crate unless they are zero-cost and
//!   appropriate for plugin callback use;
//! - prefer explicit, plugin-local wiring over premature generic frameworks;
//! - shared helpers should make future plugins more consistent without hiding
//!   important audio-thread behavior.
