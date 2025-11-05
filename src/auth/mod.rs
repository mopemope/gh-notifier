//! Authentication module for GitHub Notifier
//!
//! This module provides functionality for OAuth device flow authentication,
//! token management, validation, and refresh operations.

mod core;
mod refresh;
mod token_management;
mod types;
mod validation;

pub use core::AuthManager;
