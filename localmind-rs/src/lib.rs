pub mod db;
pub mod vector;
pub mod rag;
pub mod ollama;
pub mod document;

use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;