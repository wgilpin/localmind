pub mod db;
pub mod vector;
pub mod rag;
pub mod ollama;
pub mod lmstudio;
pub mod document;
pub mod bookmark;
pub mod bookmark_exclusion;
pub mod fetcher;
pub mod youtube;

use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;