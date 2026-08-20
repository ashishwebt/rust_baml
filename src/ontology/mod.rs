pub mod model;
pub mod normalizer;

pub use model::{Entity, Ontology, OntologyError, PropertySpec, Relationship};
pub use normalizer::OntologyNormalizer;
