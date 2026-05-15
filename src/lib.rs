//! remendo-tui — a terminal-based interface for interacting with
//! agentic patch review mechanisms (Sashiko instances).
//!
//! This library crate exposes the core modules for integration
//! testing. The binary entry point is in `src/main.rs`.
#![warn(clippy::pedantic, clippy::style, clippy::perf)]
#![deny(clippy::unwrap_used)]

/// Application state and lifecycle.
pub mod app;

/// Persistent bookmark storage.
pub mod bookmarks;

/// Sashiko API client: trait, HTTP implementation, error types.
pub mod client;

/// TEA command/effect system for async side-effects.
pub mod cmd;

/// Application configuration: remotes, keybindings, theme, paths.
pub mod config;

/// Terminal events and event-to-message translation.
pub mod event;

/// Domain data models for Sashiko API entities.
pub mod models;

/// Widget renderer.
pub mod ui;

/// Terminal user interface lifecycle.
pub mod tui;

/// Application state updater (TEA update function).
pub mod update;
