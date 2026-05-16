//! Loca/SutraDB-backed GraphStore (feature `loca`).
//!
//! Loca is the author's embedded Rust RDF-star triple store
//! (SutraDB/loka-core). The QueryKey graph is **derived** from the
//! canonical markdown (docs/markdown-schema.md); this backend persists
//! that derived projection as triples in a `.sdb` directory.
//!
//! Status: store_* persist real triples via PersistentStore. SPARQL
//! `query()` and the typed read-backs are honest TODOs — loka_sparql's
//! executor runs over an in-memory TripleStore + TermDictionary, so a
//! persistent-store query bridge needs wiring (tracked in todo.md /
//! queue.md). The old Fuseki SPARQL passthrough was a thin feature;
//! parity is deferred, not faked.

use async_trait::async_trait;
use loka_core::{PersistentStore, Triple};
use uuid::Uuid;

use super::{GraphStore, SparqlResult};
use crate::models::{Conflict, ConflictResolution, Message, Person, Task, TaskStatus};

const NS: &str = "http://querykey.dev/ns/";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

pub struct LocaGraph {
    store: PersistentStore,
    path: String,
}

impl LocaGraph {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let store = PersistentStore::open(path)
            .map_err(|e| anyhow::anyhow!("loca: open {path}: {e}"))?;
        Ok(Self {
            store,
            path: path.to_string(),
        })
    }

    fn iri(&self, kind: &str, id: &str) -> String {
        format!("{NS}{kind}/{id}")
    }

    fn triple(&self, s: &str, p: &str, o: &str) -> anyhow::Result<()> {
        let sid = self.store.intern(s).map_err(|e| anyhow::anyhow!("{e}"))?;
        let pid = self.store.intern(p).map_err(|e| anyhow::anyhow!("{e}"))?;
        let oid = self.store.intern(o).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.store
            .insert(Triple::new(sid, pid, oid))
            .map_err(|e| anyhow::anyhow!("loca insert: {e}"))
    }

    fn lit(&self, s: &str, p: &str, value: &str) -> anyhow::Result<()> {
        // Literals stored as quoted strings, matching N-Triples style.
        self.triple(s, p, &format!("\"{}\"", value.replace('"', "\\\"")))
    }
}

#[async_trait]
impl GraphStore for LocaGraph {
    async fn ping(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn ensure_dataset(&self) -> anyhow::Result<()> {
        Ok(()) // embedded; the .sdb dir is created on open()
    }

    async fn store_person(&self, p: &Person) -> anyhow::Result<()> {
        let s = self.iri("person", &p.id);
        self.triple(&s, RDF_TYPE, &format!("{NS}Person"))?;
        self.lit(&s, &format!("{NS}displayName"), &p.display_name)?;
        if !p.role.is_empty() {
            self.lit(&s, &format!("{NS}role"), &p.role)?;
        }
        for h in &p.handles {
            self.lit(&s, &format!("{NS}handle/{}", h.platform), &h.identifier)?;
        }
        Ok(())
    }

    async fn store_task(&self, t: &Task) -> anyhow::Result<()> {
        let s = self.iri("task", &t.id.to_string());
        self.triple(&s, RDF_TYPE, &format!("{NS}Task"))?;
        self.lit(&s, &format!("{NS}title"), &t.title)?;
        self.lit(
            &s,
            &format!("{NS}status"),
            &serde_json::to_string(&t.status).unwrap_or_default(),
        )?;
        if !t.assigned_to.is_empty() {
            self.triple(
                &s,
                &format!("{NS}assignedTo"),
                &self.iri("person", &t.assigned_to),
            )?;
        }
        Ok(())
    }

    async fn store_message(&self, m: &Message) -> anyhow::Result<()> {
        let s = self.iri("message", &m.id.to_string());
        self.triple(&s, RDF_TYPE, &format!("{NS}Message"))?;
        self.lit(&s, &format!("{NS}content"), &m.content)?;
        if !m.author.is_empty() {
            self.triple(&s, &format!("{NS}author"), &self.iri("person", &m.author))?;
        }
        Ok(())
    }

    async fn store_conflict(&self, c: &Conflict) -> anyhow::Result<()> {
        let s = self.iri("conflict", &c.id.to_string());
        self.triple(&s, RDF_TYPE, &format!("{NS}Conflict"))?;
        self.lit(&s, &format!("{NS}explanation"), &c.explanation)?;
        Ok(())
    }

    async fn get_all_persons(&self) -> anyhow::Result<Vec<Person>> {
        // TODO(port): typed read-back from triples. loka_sparql executes
        // over TripleStore+TermDictionary, not PersistentStore — needs a
        // persistent query bridge. See todo.md.
        Ok(Vec::new())
    }
    async fn get_tasks_for_person(&self, _person_id: &str) -> anyhow::Result<Vec<Task>> {
        Ok(Vec::new()) // TODO(port): see get_all_persons note
    }
    async fn get_unresolved_conflicts(&self) -> anyhow::Result<Vec<Conflict>> {
        Ok(Vec::new()) // TODO(port): see get_all_persons note
    }

    async fn update_task_status(
        &self,
        _id: Uuid,
        _status: TaskStatus,
    ) -> anyhow::Result<Option<Task>> {
        // TODO(port): SPARQL UPDATE delete+insert of querykey:status
        // (handlers.go handleUpdateTask). Needs the persistent query
        // bridge so we can read back the modified task. See todo.md
        // Phase 11.
        Ok(None)
    }

    async fn resolve_conflict_with(
        &self,
        _id: Uuid,
        _resolution: ConflictResolution,
        _resolved_by: &str,
    ) -> anyhow::Result<Option<Conflict>> {
        // TODO(port): SPARQL UPDATE of querykey:resolution / resolvedBy
        // / resolvedAt (handlers.go handleResolveConflict). Same
        // persistent bridge requirement.
        Ok(None)
    }

    async fn query(&self, sparql: &str) -> anyhow::Result<SparqlResult> {
        // Validate the query so callers get a real parse error, but
        // execution over the persistent store is not yet bridged.
        loka_sparql::parse(sparql).map_err(|e| anyhow::anyhow!("sparql parse: {e}"))?;
        tracing::warn!(
            "[loca] SPARQL parsed but persistent execution not yet wired \
             (TODO: persistent query bridge); returning empty result"
        );
        Ok(SparqlResult::default())
    }
    async fn update(&self, _sparql: &str) -> anyhow::Result<()> {
        Ok(()) // TODO(port): SPARQL UPDATE over PersistentStore
    }
    async fn insert_triples(&self, _ntriples: &str) -> anyhow::Result<()> {
        Ok(()) // TODO(port): use loka_core::ntriples parser
    }

    fn backend(&self) -> String {
        format!("loca:{}", self.path)
    }
}
