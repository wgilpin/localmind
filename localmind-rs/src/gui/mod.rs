//! GUI module for LocalMind egui frontend
//!
//! This module contains all UI components for the native desktop application.

pub mod app;
pub mod state;
pub mod views;
pub mod widgets;

pub use app::LocalMindApp;
pub use state::{InitStatus, Toast, ToastType, View};
