//! Authentication module for GitHub Notifier
//!
//! This module provides functionality for Personal Access Token (PAT) authentication,
//! token management, validation, and storage operations.

mod core;
mod refresh;
mod token_management;
mod types;
mod validation;

pub use core::AuthManager;
