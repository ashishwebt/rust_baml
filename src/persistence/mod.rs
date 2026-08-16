pub mod cypher;

pub use cypher::CypherAdapter;

use crate::extraction::EntitySchema;
use serde_json::Value;

pub trait PersistenceAdapter {
    fn generate_queries(&self, schemas: &[EntitySchema], payload: &Value) -> String;

    #[allow(dead_code)]
    fn save(&self, queries: &str, out_path: Option<&std::path::Path>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = out_path {
            std::fs::write(path, queries)?;
        }
        Ok(())
    }
}
