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
