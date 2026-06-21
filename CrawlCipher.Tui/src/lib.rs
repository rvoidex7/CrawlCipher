//! CrawlCipher Terminal - Rust TUI for CrawlCipher simulation
//!
//! This crate provides the terminal user interface for CrawlCipher.
//! All game logic lives in the the proprietary Native Engine shared library.
//! This Rust crate handles ONLY: terminal rendering, user input, and FFI calls.

pub mod ffi;
pub mod input;
pub mod ui;
pub mod inventory_ui;
pub mod stellar;
pub mod config;
pub mod background;
