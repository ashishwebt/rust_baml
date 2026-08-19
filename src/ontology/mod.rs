//! Source-of-truth ontology layer.
//!
//! `ontology.json` is the canonical schema definition for the domain.
//! The generated BAML file is a derived adapter for the AI extraction layer,
//! not the primary ontology definition.
pub mod normalizer;
pub use normalizer::OntologyNormalizer;
