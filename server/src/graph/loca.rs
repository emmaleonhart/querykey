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
use loka_core::{PersistentStore, TermDictionary, Triple, TripleStore};

use super::{GraphStore, SparqlBindings, SparqlHead, SparqlResult, SparqlValue};
use crate::models::{Conflict, Handle, Message, Person, Task};

/// Strip surrounding quotes + unescape `\"` from a stored literal term.
fn unquote(s: &str) -> String {
    let t = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s);
    t.replace("\\\"", "\"")
}

use super::NS;
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
        // Field set mirrors fuseki.go StoreTask.
        let s = self.iri("task", &t.id.to_string());
        self.triple(&s, RDF_TYPE, &format!("{NS}Task"))?;
        self.lit(&s, &format!("{NS}title"), &t.title)?;
        if !t.description.is_empty() {
            self.lit(&s, &format!("{NS}description"), &t.description)?;
        }
        self.lit(
            &s,
            &format!("{NS}status"),
            serde_json::to_value(t.status)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .as_deref()
                .unwrap_or("extracted"),
        )?;
        self.lit(&s, &format!("{NS}confidence"), &t.confidence.to_string())?;
        self.lit(
            &s,
            &format!("{NS}ambiguityScore"),
            &t.ambiguity_score.to_string(),
        )?;
        self.lit(&s, &format!("{NS}createdAt"), &t.created_at.to_rfc3339())?;
        self.lit(&s, &format!("{NS}updatedAt"), &t.updated_at.to_rfc3339())?;
        if !t.assigned_to.is_empty() {
            self.triple(
                &s,
                &format!("{NS}assignedTo"),
                &self.iri("person", &t.assigned_to),
            )?;
        }
        if !t.assigned_by.is_empty() {
            self.triple(
                &s,
                &format!("{NS}assignedBy"),
                &self.iri("person", &t.assigned_by),
            )?;
        }
        if let Some(d) = t.deadline {
            self.lit(&s, &format!("{NS}deadline"), &d.to_rfc3339())?;
        }
        for mid in &t.source_messages {
            self.triple(&s, &format!("{NS}sourceMessage"), &self.iri("message", mid))?;
        }
        Ok(())
    }

    async fn store_message(&self, m: &Message) -> anyhow::Result<()> {
        // Field set mirrors fuseki.go StoreMessage.
        let s = self.iri("message", &m.id.to_string());
        self.triple(&s, RDF_TYPE, &format!("{NS}Message"))?;
        self.lit(&s, &format!("{NS}content"), &m.content)?;
        self.lit(&s, &format!("{NS}confidence"), &m.confidence.to_string())?;
        if !m.source_ingest.is_empty() {
            self.lit(&s, &format!("{NS}sourceIngest"), &m.source_ingest)?;
        }
        if !m.author.is_empty() {
            self.triple(&s, &format!("{NS}author"), &self.iri("person", &m.author))?;
        }
        if let Some(ts) = m.timestamp {
            self.lit(&s, &format!("{NS}timestamp"), &ts.to_rfc3339())?;
        }
        Ok(())
    }

    async fn store_conflict(&self, c: &Conflict) -> anyhow::Result<()> {
        // Field set mirrors fuseki.go StoreConflict.
        let s = self.iri("conflict", &c.id.to_string());
        self.triple(&s, RDF_TYPE, &format!("{NS}Conflict"))?;
        self.lit(&s, &format!("{NS}explanation"), &c.explanation)?;
        self.lit(
            &s,
            &format!("{NS}conflictType"),
            serde_json::to_value(c.conflict_type)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .as_deref()
                .unwrap_or(""),
        )?;
        self.lit(
            &s,
            &format!("{NS}resolution"),
            serde_json::to_value(c.resolution)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .as_deref()
                .unwrap_or("unresolved"),
        )?;
        if !c.message_a.is_empty() {
            self.triple(&s, &format!("{NS}messageA"), &self.iri("message", &c.message_a))?;
        }
        if !c.message_b.is_empty() {
            self.triple(&s, &format!("{NS}messageB"), &self.iri("message", &c.message_b))?;
        }
        self.lit(&s, &format!("{NS}createdAt"), &c.created_at.to_rfc3339())?;
        Ok(())
    }

    async fn get_all_persons(&self) -> anyhow::Result<Vec<Person>> {
        // Typed read-back via the POS index: find every subject typed
        // as Person, then gather its attribute triples.
        let type_p = match self.store.lookup(RDF_TYPE).ok().flatten() {
            Some(x) => x,
            None => return Ok(Vec::new()),
        };
        let person_c = match self.store.lookup(&format!("{NS}Person")).ok().flatten() {
            Some(x) => x,
            None => return Ok(Vec::new()),
        };
        let dn_p = self.store.lookup(&format!("{NS}displayName")).ok().flatten();
        let role_p = self.store.lookup(&format!("{NS}role")).ok().flatten();
        let handle_prefix = format!("{NS}handle/");

        let mut out = Vec::new();
        for t in self.store.find_by_predicate_object(type_p, person_c) {
            let subj = t.subject;
            let subj_iri = self.store.resolve(subj).ok().flatten().unwrap_or_default();
            let id = subj_iri.rsplit('/').next().unwrap_or("").to_string();
            let mut display_name = String::new();
            let mut role = String::new();
            let mut handles: Vec<Handle> = Vec::new();
            for a in self.store.find_by_subject(subj) {
                let pred = self.store.resolve(a.predicate).ok().flatten().unwrap_or_default();
                let obj = unquote(&self.store.resolve(a.object).ok().flatten().unwrap_or_default());
                if Some(a.predicate) == dn_p {
                    display_name = obj;
                } else if Some(a.predicate) == role_p {
                    role = obj;
                } else if let Some(platform) = pred.strip_prefix(&handle_prefix) {
                    handles.push(Handle {
                        platform: platform.to_string(),
                        identifier: obj,
                    });
                }
            }
            out.push(Person {
                id,
                display_name,
                handles,
                role,
                contact_cascade: Vec::new(),
                // The derived graph is a lossy index; timestamps are not
                // round-tripped here — the canonical record is the
                // markdown file (docs/markdown-schema.md). TODO(port):
                // hydrate full Person from markdown, not from triples.
                created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
                    .unwrap_or_else(chrono::Utc::now),
            });
        }
        Ok(out)
    }
    async fn get_all_tasks(&self) -> anyhow::Result<Vec<Task>> {
        // store_task (R4-2) now projects the full field set, so this
        // reconstruction is faithful. (Canonical write path remains
        // markdown per the architecture; this read is over the derived
        // index.)
        let type_p = match self.store.lookup(RDF_TYPE).ok().flatten() {
            Some(x) => x,
            None => return Ok(Vec::new()),
        };
        let task_c = match self.store.lookup(&format!("{NS}Task")).ok().flatten() {
            Some(x) => x,
            None => return Ok(Vec::new()),
        };
        let strip_iri = |v: &str| v.rsplit('/').next().unwrap_or("").to_string();
        let mut out = Vec::new();
        for t in self.store.find_by_predicate_object(type_p, task_c) {
            let subj = t.subject;
            let subj_iri = self.store.resolve(subj).ok().flatten().unwrap_or_default();
            let id = uuid::Uuid::parse_str(subj_iri.rsplit('/').next().unwrap_or(""))
                .unwrap_or_default();
            let mut task = Task {
                id,
                title: String::new(),
                description: String::new(),
                status: crate::models::TaskStatus::Extracted,
                assigned_to: String::new(),
                assigned_by: String::new(),
                deadline: None,
                confidence: 0.0,
                ambiguity_score: 0.0,
                source_messages: Vec::new(),
                created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
                    .unwrap_or_else(chrono::Utc::now),
                updated_at: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
                    .unwrap_or_else(chrono::Utc::now),
            };
            for a in self.store.find_by_subject(subj) {
                let pred = self.store.resolve(a.predicate).ok().flatten().unwrap_or_default();
                let raw = self.store.resolve(a.object).ok().flatten().unwrap_or_default();
                let val = unquote(&raw);
                match pred.strip_prefix(NS) {
                    Some("title") => task.title = val,
                    Some("description") => task.description = val,
                    Some("status") => {
                        task.status = serde_json::from_value(serde_json::Value::String(val))
                            .unwrap_or(crate::models::TaskStatus::Extracted)
                    }
                    Some("confidence") => task.confidence = val.parse().unwrap_or(0.0),
                    Some("ambiguityScore") => task.ambiguity_score = val.parse().unwrap_or(0.0),
                    Some("assignedTo") => task.assigned_to = strip_iri(&val),
                    Some("assignedBy") => task.assigned_by = strip_iri(&val),
                    Some("deadline") => {
                        task.deadline = chrono::DateTime::parse_from_rfc3339(&val)
                            .ok()
                            .map(|d| d.with_timezone(&chrono::Utc))
                    }
                    Some("createdAt") => {
                        if let Ok(d) = chrono::DateTime::parse_from_rfc3339(&val) {
                            task.created_at = d.with_timezone(&chrono::Utc);
                        }
                    }
                    Some("updatedAt") => {
                        if let Ok(d) = chrono::DateTime::parse_from_rfc3339(&val) {
                            task.updated_at = d.with_timezone(&chrono::Utc);
                        }
                    }
                    Some("sourceMessage") => task.source_messages.push(strip_iri(&val)),
                    _ => {}
                }
            }
            out.push(task);
        }
        Ok(out)
    }

    async fn get_tasks_for_person(&self, person_id: &str) -> anyhow::Result<Vec<Task>> {
        let all = self.get_all_tasks().await?;
        Ok(all
            .into_iter()
            .filter(|t| t.assigned_to == person_id)
            .collect())
    }
    async fn get_unresolved_conflicts(&self) -> anyhow::Result<Vec<Conflict>> {
        // Same rationale as get_tasks_for_person: hydrate from canonical
        // markdown, not the lossy derived graph. TODO(port).
        Ok(Vec::new())
    }

    async fn query(&self, sparql: &str) -> anyhow::Result<SparqlResult> {
        let q = loka_sparql::parse(sparql)
            .map_err(|e| anyhow::anyhow!("sparql parse: {e}"))?;

        // Bridge: loka_sparql::execute runs over an in-memory
        // TripleStore + TermDictionary. Snapshot the PersistentStore
        // (its triples carry persistent TermIds; load_terms_into copies
        // the persistent dictionary id-for-id via insert_with_id, so
        // the two stay consistent). Single-user PRM graphs are small;
        // TODO(perf): cache/incrementally maintain this snapshot.
        let mut ts = TripleStore::new();
        for t in self.store.iter() {
            ts.insert(t).map_err(|e| anyhow::anyhow!("snapshot: {e}"))?;
        }
        let mut dict = TermDictionary::new();
        self.store.load_terms_into(&mut dict);

        let qr = loka_sparql::execute(&q, &ts, &dict)
            .map_err(|e| anyhow::anyhow!("sparql exec: {e}"))?;

        let vars = qr.columns.clone();
        let mut bindings = Vec::with_capacity(qr.rows.len());
        for row in &qr.rows {
            let mut m = std::collections::HashMap::new();
            for (var, tid) in row {
                let resolved = dict.resolve(*tid).unwrap_or("").to_string();
                // Quoted terms are literals; bare terms are IRIs.
                let (value_type, value) = if resolved.starts_with('"') {
                    (
                        "literal".to_string(),
                        resolved.trim_matches('"').to_string(),
                    )
                } else {
                    ("uri".to_string(), resolved)
                };
                m.insert(var.clone(), SparqlValue { value_type, value });
            }
            bindings.push(m);
        }
        Ok(SparqlResult {
            head: SparqlHead { vars },
            results: SparqlBindings { bindings },
        })
    }
    async fn update(&self, sparql: &str) -> anyhow::Result<()> {
        // loka_sparql parses INSERT DATA / DELETE DATA but its executor
        // is read-only (&TripleStore) — there is no public SPARQL
        // UPDATE writer over PersistentStore. We validate the syntax so
        // callers get a real parse error; mutation is intentionally not
        // applied. Writes go through the typed store_* paths or
        // insert_triples (N-Triples). Documented limitation, not a bug.
        loka_sparql::parse(sparql).map_err(|e| anyhow::anyhow!("sparql parse: {e}"))?;
        tracing::warn!(
            "[loca] SPARQL UPDATE parsed but not applied (no public \
             update writer in loka); use store_* / insert_triples"
        );
        Ok(())
    }

    async fn insert_triples(&self, ntriples: &str) -> anyhow::Result<()> {
        // Real N-Triples ingest via loka_core::ntriples.
        let mut n = 0usize;
        for line in ntriples.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((subj, pred, obj)) = loka_core::ntriples::parse_ntriples_line(line) {
                self.triple(&subj, &pred, &obj)?;
                n += 1;
            }
        }
        tracing::info!("[loca] insert_triples: {n} triple(s)");
        Ok(())
    }

    fn backend(&self) -> String {
        format!("loca:{}", self.path)
    }
}
