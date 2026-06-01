//! Shared DSP building blocks for Foxplugs plugins.
//!
//! This crate is intentionally minimal for now. Move DSP code here only when it is
//! actually shared by more than one plugin, or when a helper is clearly reusable
//! across future processors.
//!
//! Design constraints:
//!
//! - keep this crate free of plugin-framework and GUI dependencies;
//! - prefer zero-cost abstractions suitable for real-time audio paths;
//! - avoid heap allocation, locking, logging, or panicking in helpers intended for
//!   audio callbacks;
//! - keep plugin-specific sound design inside the plugin crate until reuse is
//!   concrete.
