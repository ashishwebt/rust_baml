pub mod baml;

pub use baml::BamlExtractor;

use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct PropertySpec {
    pub data_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EntitySchema {
    pub name: String,
    pub properties: HashMap<String, PropertySpec>,
}

pub trait Extractor {
    fn extract(&self, text: &str) -> Result<Value, Box<dyn Error>>;
}
