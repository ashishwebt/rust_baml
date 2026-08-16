//! Source-of-truth ontology layer.
//!
//! `ontology.json` is the canonical schema definition for the domain.
//! The generated BAML file is a derived adapter for the AI extraction layer,
//! not the primary ontology definition.
pub mod baml_converter;
pub mod normalizer;
pub use baml_converter::BamlConverter as JsonOntologyAdapter;
pub use normalizer::OntologyNormalizer;
