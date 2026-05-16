//! Graph store abstraction. Port of the *interface* exercised by
//! server-go-old/internal/graph/fuseki.go — but Fuseki is gone. The
//! production backend is **Loca/SutraDB** (embedded; feature `loca`),
//! and the graph is a derived index over the canonical markdown
//! (see docs/markdown-schema.md). An in-memory backend is the default
//! so the crate always builds and runs without the SutraDB checkout.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Conflict, Message, Person, Task};

pub mod memory;
#[cfg(feature = "loca")]
pub mod loca;

/// Graph IRI namespace. Single source of truth (loca.rs + the
/// wikilink projector both use it) so derived triples are consistent.
pub const NS: &str = "http://querykey.dev/ns/";

/// Project resolved `[[wikilink]]` edges (canonical: the vault) into
/// N-Triples for the derived store. Dangling targets still get an
/// edge plus a `label` literal so they stay queryable. The graph is
/// rebuilt from the vault on startup, so this is a derived view —
/// never the source of truth.
pub fn link_ntriples(edges: &[crate::vault::LinkEdge]) -> String {
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let iri = |kind: &str, id: &str| format!("<{NS}{kind}/{}>", id.replace(' ', "%20"));
    let mut out = String::new();
    for e in edges {
        let s = iri(&e.from_kind, &e.from_id);
        let p = format!("<{NS}{}>", e.predicate);
        let o = iri(&e.to_kind, &e.to_id);
        out.push_str(&format!("{s} {p} {o} .\n"));
        if !e.resolved {
            out.push_str(&format!("{o} <{NS}label> \"{}\" .\n", esc(&e.to_label)));
        }
    }
    out
}

/// Shape mirrors fuseki.go's SPARQLResult so the /api/graph/query
/// passthrough keeps the same JSON contract.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparqlResult {
    pub head: SparqlHead,
    pub results: SparqlBindings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparqlHead {
    #[serde(default)]
    pub vars: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparqlBindings {
    #[serde(default)]
    pub bindings: Vec<HashMap<String, SparqlValue>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparqlValue {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: String,
}

#[cfg(test)]
mod link_projection_tests {
    use super::*;
    use crate::vault::LinkEdge;

    #[test]
    fn emits_edge_and_dangling_label_with_escaping() {
        let nt = link_ntriples(&[
            LinkEdge {
                from_kind: "note".into(),
                from_id: "salon".into(),
                predicate: "knows".into(),
                to_kind: "person".into(),
                to_id: "ada-lovelace".into(),
                to_label: "Ada".into(),
                resolved: true,
            },
            LinkEdge {
                from_kind: "task".into(),
                from_id: "t1".into(),
                predicate: "references".into(),
                to_kind: "thing".into(),
                to_id: "babbage".into(),
                to_label: "Babbage \"Engine\"".into(),
                resolved: false,
            },
        ]);
        assert!(nt.contains(
            "<http://querykey.dev/ns/note/salon> \
             <http://querykey.dev/ns/knows> \
             <http://querykey.dev/ns/person/ada-lovelace> ."
        ));
        // Dangling target carries a label literal, quotes escaped.
        assert!(nt.contains(
            "<http://querykey.dev/ns/thing/babbage> \
             <http://querykey.dev/ns/label> \"Babbage \\\"Engine\\\"\" ."
        ));
    }
}

/// The operations the rest of the server needs from the graph layer.
/// Method set mirrors the domain methods of the old FusekiClient.
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// True if the backend is reachable/usable.
    async fn ping(&self) -> anyhow::Result<()>;
    /// Ensure the store/dataset exists (no-op for embedded backends).
    async fn ensure_dataset(&self) -> anyhow::Result<()>;

    async fn store_person(&self, p: &Person) -> anyhow::Result<()>;
    async fn store_task(&self, t: &Task) -> anyhow::Result<()>;
    async fn store_message(&self, m: &Message) -> anyhow::Result<()>;
    async fn store_conflict(&self, c: &Conflict) -> anyhow::Result<()>;

    async fn get_all_persons(&self) -> anyhow::Result<Vec<Person>>;
    async fn get_all_tasks(&self) -> anyhow::Result<Vec<Task>>;
    async fn get_tasks_for_person(&self, person_id: &str) -> anyhow::Result<Vec<Task>>;
    async fn get_unresolved_conflicts(&self) -> anyhow::Result<Vec<Conflict>>;

    /// SPARQL SELECT passthrough (POST /api/graph/query).
    async fn query(&self, sparql: &str) -> anyhow::Result<SparqlResult>;
    /// SPARQL UPDATE passthrough.
    async fn update(&self, sparql: &str) -> anyhow::Result<()>;
    /// Bulk N-Triples insert.
    async fn insert_triples(&self, ntriples: &str) -> anyhow::Result<()>;

    /// Human label for logs ("in-memory", "loca:<path>").
    fn backend(&self) -> String;
}
