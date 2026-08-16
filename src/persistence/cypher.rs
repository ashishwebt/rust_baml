use crate::extraction::EntitySchema;
use serde_json::Value;
use std::collections::HashMap;

pub struct CypherAdapter;

impl CypherAdapter {
    fn fmt_value(v: &Value) -> String {
        match v {
            Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            other => format!("\"{}\"", other.to_string().replace('"', "\\\"")),
        }
    }

    #[allow(dead_code)]
    fn record_props_for_schema(schema: &EntitySchema, record: &Value) -> Option<HashMap<String, Value>> {
        // Expect record to be an object with either a single object for the schema name
        // or an array of objects. This helper extracts matching entries.
        if let Value::Object(map) = record {
            if let Some(v) = map.get(&schema.name) {
                if v.is_object() {
                    if let Value::Object(obj) = v {
                        let mut out = HashMap::new();
                        for (k, _) in &schema.properties {
                            if let Some(val) = obj.get(k) {
                                out.insert(k.clone(), val.clone());
                            }
                        }
                        return Some(out);
                    }
                }
            }
        }
        None
    }

    pub fn generate_for_payload(_schemas: &[EntitySchema], payload: &Value) -> String {
        let mut node_parts: Vec<String> = Vec::new();
        let mut rel_parts: Vec<String> = Vec::new();

        // Build a quick lookup of source_id -> label so we can resolve parents
        let mut id_to_label: HashMap<String, String> = HashMap::new();

        if let Value::Object(map) = payload {
            for (label, items) in map {
                let records: Vec<Value> = match items {
                    Value::Array(arr) => arr.clone(),
                    Value::Object(_) => vec![items.clone()],
                    _ => continue,
                };

                for rec in &records {
                    if let Value::Object(obj) = rec {
                        if let Some(Value::String(sid)) = obj.get("source_id") {
                            id_to_label.insert(sid.clone(), label.clone());
                        }
                    }
                }
            }

            // Create/merge nodes and set properties (exclude parent_source_ids)
            for (label, items) in map {
                let records: Vec<Value> = match items {
                    Value::Array(arr) => arr.clone(),
                    Value::Object(_) => vec![items.clone()],
                    _ => continue,
                };

                for rec in records {
                    if let Value::Object(obj) = rec {
                        // source_id is authoritative key for upserts
                        let source_id = obj.get("source_id").and_then(Value::as_str).map(|s| s.to_string())
                            .unwrap_or_else(|| {
                                // fallback: generate a stable id from name if present
                                obj.get("name").and_then(Value::as_str).map(|n| n.to_string()).unwrap_or_else(|| "generated".to_string())
                            });

                        // build a props map excluding parent_source_ids and source_id
                        let mut set_props: Vec<String> = Vec::new();
                        for (k, v) in &obj {
                            if k == "parent_source_ids" || k == "source_id" { continue; }
                            set_props.push(format!("{k}: {val}", k = k, val = Self::fmt_value(v)));
                        }

                        // MERGE on source_id then SET other props
                        let merge_stmt = format!("MERGE (n:{label} {{ source_id: {sid} }})", label = label, sid = Self::fmt_value(&Value::String(source_id.clone())));
                        node_parts.push(merge_stmt + ";");

                        if !set_props.is_empty() {
                            let set_map = format!("{{ {} }}", set_props.join(", "));
                            node_parts.push(format!("SET n += {map};", map = set_map));
                        }

                        // relationships
                        if let Some(Value::Array(parents)) = obj.get("parent_source_ids") {
                            for parent in parents {
                                if let Value::String(pid) = parent {
                                    // determine parent label if known
                                    if let Some(parent_label) = id_to_label.get(pid) {
                                        // choose relationship type heuristically
                                        let rel_type = match (label.as_str(), parent_label.as_str()) {
                                            ("Person", "Company") => "WORKS_AT",
                                            ("Person", "Skill") => "HAS_SKILLS",
                                            _ => "RELATED_TO",
                                        };

                                        rel_parts.push(format!(
                                            "MATCH (a:{cl} {{ source_id: {cid} }}), (b:{pl} {{ source_id: {pid} }}) MERGE (a)-[:{rel}]->(b);",
                                            cl = label,
                                            cid = Self::fmt_value(&Value::String(source_id.clone())),
                                            pl = parent_label,
                                            pid = Self::fmt_value(&Value::String(pid.clone())),
                                            rel = rel_type
                                        ));
                                    } else {
                                        // unknown parent label: match by property only
                                        rel_parts.push(format!(
                                            "MATCH (a:{cl} {{ source_id: {cid} }}), (b {{ source_id: {pid} }}) MERGE (a)-[:RELATED_TO]->(b);",
                                            cl = label,
                                            cid = Self::fmt_value(&Value::String(source_id.clone())),
                                            pid = Self::fmt_value(&Value::String(pid.clone()))
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // combine nodes then relationships
        [node_parts, rel_parts].concat().join("\n")
    }
}

impl super::PersistenceAdapter for CypherAdapter {
    fn generate_queries(&self, schemas: &[EntitySchema], payload: &Value) -> String {
        Self::generate_for_payload(schemas, payload)
    }
}
