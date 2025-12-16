pub mod bookmark;
pub mod bookmark_exclusion;
pub mod db;
pub mod document;
pub mod fetcher;
pub mod gui;
pub mod local_embedding;
pub mod rag;
pub mod vector;
pub mod youtube;

use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
