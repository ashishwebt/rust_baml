pub mod cypher;

pub use cypher::CypherAdapter;

use serde_json::Value;

pub trait PersistenceAdapter {
    fn generate_queries(&self, payload: &Value) -> String;
}
