//! Canonical markdown source of truth — implements
//! `docs/markdown-schema.md`.
//!
//! Markdown files on disk ARE the store of record. The Loca graph is a
//! derived, rebuildable index *projected from* this vault, never the
//! other way round. Layout:
//!
//! ```text
//! <root>/people/<id>.md   tasks/<uuid>.md   events/<uuid>.md   notes/
//! ```
//!
//! Each file is `--- <yaml frontmatter> ---` + a freeform markdown
//! body. Round-trips are lossless: every model field is either in the
//! frontmatter (stable key order via the struct definition) or is the
//! body (the human description). Editable by hand in any editor.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{Event, Handle, Person, Task, TaskStatus};

pub struct Vault {
    root: PathBuf,
}

// ---------- frontmatter structs (key order = field order) ----------

#[derive(Serialize, Deserialize)]
struct PersonFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    display_name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    handles: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    role: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    contact_cascade: Vec<String>,
    created: String,
}

#[derive(Serialize, Deserialize)]
struct TaskFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    title: String,
    status: TaskStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    person: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    assigned_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deadline: Option<String>,
    confidence: f64,
    ambiguity_score: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    source: Vec<String>,
    created: String,
    updated: String,
}

#[derive(Serialize, Deserialize)]
struct EventFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    title: String,
    start: String,
    end: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    people: Vec<String>,
    confidence: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    source: Vec<String>,
    created: String,
}

// ---------- frontmatter (de)serialization ----------

fn compose(yaml: &str, body: &str) -> String {
    format!("---\n{}---\n\n{}\n", yaml, body.trim_end())
}

/// Split `--- yaml --- body`. Returns (yaml, body). If there is no
/// frontmatter the whole input is treated as body.
fn split(content: &str) -> (String, String) {
    let c = content.strip_prefix('\u{feff}').unwrap_or(content);
    if let Some(rest) = c.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let yaml = rest[..end + 1].to_string();
            // Body is canonicalized (leading blank lines + trailing
            // whitespace trimmed) so write→read is idempotent and the
            // model's description round-trips losslessly. `compose`
            // trims the end on write to match.
            let body = rest[end + 4..]
                .trim_start_matches(['\n', '\r'])
                .trim_end()
                .to_string();
            return (yaml, body);
        }
    }
    (String::new(), c.trim_end().to_string())
}

fn rfc3339(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}
fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now))
}

// ---------- Vault ----------

impl Vault {
    pub fn open(root: &str) -> anyhow::Result<Self> {
        let root = PathBuf::from(root);
        for sub in ["people", "tasks", "events", "notes"] {
            fs::create_dir_all(root.join(sub))?;
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, sub: &str, slug: &str, yaml: String, body: &str) -> anyhow::Result<()> {
        let path = self.root.join(sub).join(format!("{slug}.md"));
        fs::write(path, compose(&yaml, body))?;
        Ok(())
    }

    fn read_files(&self, sub: &str) -> Vec<(String, String)> {
        let dir = self.root.join(sub);
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("md") {
                    if let Ok(s) = fs::read_to_string(&p) {
                        let slug = p
                            .file_stem()
                            .and_then(|x| x.to_str())
                            .unwrap_or("")
                            .to_string();
                        out.push((slug, s));
                    }
                }
            }
        }
        out
    }

    // ----- Person -----

    pub fn upsert_person(&self, p: &Person) -> anyhow::Result<()> {
        let fm = PersonFm {
            id: format!("person:{}", p.id),
            kind: "person".into(),
            display_name: p.display_name.clone(),
            handles: p
                .handles
                .iter()
                .map(|h| (h.platform.clone(), h.identifier.clone()))
                .collect(),
            role: p.role.clone(),
            contact_cascade: p.contact_cascade.clone(),
            created: rfc3339(&p.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        let body = format!("# {}\n", p.display_name);
        self.write("people", &p.id, yaml, &body)
    }

    fn person_from(&self, slug: &str, content: &str) -> Option<Person> {
        let (yaml, _body) = split(content);
        let fm: PersonFm = serde_yaml::from_str(&yaml).ok()?;
        let id = fm
            .id
            .strip_prefix("person:")
            .unwrap_or(slug)
            .to_string();
        Some(Person {
            id,
            display_name: fm.display_name,
            handles: fm
                .handles
                .into_iter()
                .map(|(platform, identifier)| Handle {
                    platform,
                    identifier,
                })
                .collect(),
            role: fm.role,
            contact_cascade: fm.contact_cascade,
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn get_person(&self, id: &str) -> Option<Person> {
        let p = self.root.join("people").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
        self.person_from(id, &s)
    }

    pub fn list_persons(&self) -> Vec<Person> {
        self.read_files("people")
            .into_iter()
            .filter_map(|(slug, c)| self.person_from(&slug, &c))
            .collect()
    }

    // ----- Task -----

    pub fn upsert_task(&self, t: &Task) -> anyhow::Result<()> {
        let fm = TaskFm {
            id: format!("task:{}", t.id),
            kind: "task".into(),
            title: t.title.clone(),
            status: t.status,
            person: if t.assigned_to.is_empty() {
                String::new()
            } else {
                format!("person:{}", t.assigned_to)
            },
            assigned_by: t.assigned_by.clone(),
            deadline: t.deadline.map(|d| rfc3339(&d)),
            confidence: t.confidence,
            ambiguity_score: t.ambiguity_score,
            source: t.source_messages.clone(),
            created: rfc3339(&t.created_at),
            updated: rfc3339(&t.updated_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("tasks", &t.id.to_string(), yaml, &t.description)
    }

    fn task_from(&self, content: &str) -> Option<Task> {
        let (yaml, body) = split(content);
        let fm: TaskFm = serde_yaml::from_str(&yaml).ok()?;
        let id = uuid::Uuid::parse_str(fm.id.strip_prefix("task:").unwrap_or(&fm.id)).ok()?;
        Some(Task {
            id,
            title: fm.title,
            description: body,
            status: fm.status,
            assigned_to: fm
                .person
                .strip_prefix("person:")
                .unwrap_or(&fm.person)
                .to_string(),
            assigned_by: fm.assigned_by,
            deadline: fm.deadline.as_deref().map(parse_dt),
            confidence: fm.confidence,
            ambiguity_score: fm.ambiguity_score,
            source_messages: fm.source,
            created_at: parse_dt(&fm.created),
            updated_at: parse_dt(&fm.updated),
        })
    }

    pub fn get_task(&self, id: &uuid::Uuid) -> Option<Task> {
        let p = self.root.join("tasks").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
        self.task_from(&s)
    }

    pub fn list_tasks(&self) -> Vec<Task> {
        self.read_files("tasks")
            .into_iter()
            .filter_map(|(_, c)| self.task_from(&c))
            .collect()
    }

    pub fn tasks_for_person(&self, person_id: &str) -> Vec<Task> {
        self.list_tasks()
            .into_iter()
            .filter(|t| t.assigned_to == person_id)
            .collect()
    }

    // ----- Event -----

    pub fn upsert_event(&self, e: &Event) -> anyhow::Result<()> {
        let fm = EventFm {
            id: format!("event:{}", e.id),
            kind: "event".into(),
            title: e.title.clone(),
            start: rfc3339(&e.start_time),
            end: rfc3339(&e.end_time),
            people: e
                .participants
                .iter()
                .map(|p| format!("person:{p}"))
                .collect(),
            confidence: e.confidence,
            source: e.source_messages.clone(),
            created: rfc3339(&e.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("events", &e.id.to_string(), yaml, &e.description)
    }

    fn event_from(&self, content: &str) -> Option<Event> {
        let (yaml, body) = split(content);
        let fm: EventFm = serde_yaml::from_str(&yaml).ok()?;
        let id = uuid::Uuid::parse_str(fm.id.strip_prefix("event:").unwrap_or(&fm.id)).ok()?;
        Some(Event {
            id,
            title: fm.title,
            description: body,
            start_time: parse_dt(&fm.start),
            end_time: parse_dt(&fm.end),
            participants: fm
                .people
                .into_iter()
                .map(|p| p.strip_prefix("person:").unwrap_or(&p).to_string())
                .collect(),
            confidence: fm.confidence,
            source_messages: fm.source,
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn list_events(&self) -> Vec<Event> {
        self.read_files("events")
            .into_iter()
            .filter_map(|(_, c)| self.event_from(&c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_and_task_round_trip_losslessly() {
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        let p = Person {
            id: "ada-lovelace".into(),
            display_name: "Ada Lovelace".into(),
            handles: vec![Handle {
                platform: "github".into(),
                identifier: "ada".into(),
            }],
            role: "mathematician".into(),
            contact_cascade: vec!["app".into(), "discord".into()],
            created_at: DateTime::parse_from_rfc3339("2026-05-15T00:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
        };
        v.upsert_person(&p).unwrap();
        let got = v.get_person("ada-lovelace").unwrap();
        assert_eq!(got.id, p.id);
        assert_eq!(got.display_name, p.display_name);
        assert_eq!(got.handles.len(), 1);
        assert_eq!(got.handles[0].platform, "github");
        assert_eq!(got.role, p.role);
        assert_eq!(got.contact_cascade, p.contact_cascade);
        assert_eq!(got.created_at, p.created_at);

        let id = uuid::Uuid::new_v4();
        let t = Task {
            id,
            title: "Return the book".into(),
            description: "Ada lent me the Rust book.\n\nMulti-line body.".into(),
            status: TaskStatus::InProgress,
            assigned_to: "ada-lovelace".into(),
            assigned_by: String::new(),
            deadline: Some(
                DateTime::parse_from_rfc3339("2026-05-20T00:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            confidence: 0.7,
            ambiguity_score: 0.2,
            source_messages: vec!["ingest:abc".into()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        v.upsert_task(&t).unwrap();
        let gt = v.get_task(&id).unwrap();
        assert_eq!(gt.id, t.id);
        assert_eq!(gt.title, t.title);
        assert_eq!(gt.description, t.description);
        assert_eq!(gt.status, t.status);
        assert_eq!(gt.assigned_to, "ada-lovelace");
        assert_eq!(gt.deadline, t.deadline);
        assert_eq!(gt.confidence, 0.7);
        assert_eq!(v.tasks_for_person("ada-lovelace").len(), 1);
        assert_eq!(v.list_persons().len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

