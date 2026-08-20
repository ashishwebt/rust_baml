use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ontology error: {0}")]
    Ontology(#[from] crate::ontology::OntologyError),

    #[error("extraction failed: {0}")]
    Extraction(String),

    #[error("persistence failed: {0}")]
    Persistence(String),
}
