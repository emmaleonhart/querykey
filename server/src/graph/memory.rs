//! Default in-memory GraphStore. Always available so the crate builds
//! and runs without the SutraDB checkout. The Go server likewise
//! "continued without graph store" when Fuseki was unreachable.

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

use super::{GraphStore, SparqlResult};
use crate::models::{Conflict, ConflictResolution, Message, Person, Task, TaskStatus};

#[derive(Default)]
pub struct InMemoryGraph {
    persons: Mutex<Vec<Person>>,
    tasks: Mutex<Vec<Task>>,
    messages: Mutex<Vec<Message>>,
    conflicts: Mutex<Vec<Conflict>>,
}

impl InMemoryGraph {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl GraphStore for InMemoryGraph {
    async fn ping(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn ensure_dataset(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_person(&self, p: &Person) -> anyhow::Result<()> {
        let mut v = self.persons.lock().unwrap();
        if let Some(slot) = v.iter_mut().find(|x| x.id == p.id) {
            *slot = p.clone();
        } else {
            v.push(p.clone());
        }
        Ok(())
    }

    async fn store_task(&self, t: &Task) -> anyhow::Result<()> {
        let mut v = self.tasks.lock().unwrap();
        if let Some(slot) = v.iter_mut().find(|x| x.id == t.id) {
            *slot = t.clone();
        } else {
            v.push(t.clone());
        }
        Ok(())
    }

    async fn store_message(&self, m: &Message) -> anyhow::Result<()> {
        self.messages.lock().unwrap().push(m.clone());
        Ok(())
    }

    async fn store_conflict(&self, c: &Conflict) -> anyhow::Result<()> {
        self.conflicts.lock().unwrap().push(c.clone());
        Ok(())
    }

    async fn get_all_persons(&self) -> anyhow::Result<Vec<Person>> {
        Ok(self.persons.lock().unwrap().clone())
    }

    async fn get_tasks_for_person(&self, person_id: &str) -> anyhow::Result<Vec<Task>> {
        Ok(self
            .tasks
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.assigned_to == person_id)
            .cloned()
            .collect())
    }

    async fn get_unresolved_conflicts(&self) -> anyhow::Result<Vec<Conflict>> {
        Ok(self
            .conflicts
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.resolution == ConflictResolution::Unresolved)
            .cloned()
            .collect())
    }

    async fn update_task_status(
        &self,
        id: Uuid,
        status: TaskStatus,
    ) -> anyhow::Result<Option<Task>> {
        let mut v = self.tasks.lock().unwrap();
        let Some(task) = v.iter_mut().find(|t| t.id == id) else {
            return Ok(None);
        };
        task.status = status;
        task.updated_at = chrono::Utc::now();
        Ok(Some(task.clone()))
    }

    async fn resolve_conflict_with(
        &self,
        id: Uuid,
        resolution: ConflictResolution,
        resolved_by: &str,
    ) -> anyhow::Result<Option<Conflict>> {
        let mut v = self.conflicts.lock().unwrap();
        let Some(conflict) = v.iter_mut().find(|c| c.id == id) else {
            return Ok(None);
        };
        conflict.resolution = resolution;
        conflict.resolved_by = resolved_by.to_string();
        conflict.resolved_at = Some(chrono::Utc::now());
        Ok(Some(conflict.clone()))
    }

    async fn query(&self, _sparql: &str) -> anyhow::Result<SparqlResult> {
        // No SPARQL engine in the in-memory backend. Use --features loca.
        Ok(SparqlResult::default())
    }
    async fn update(&self, _sparql: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn insert_triples(&self, _ntriples: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn backend(&self) -> String {
        "in-memory".to_string()
    }
}
