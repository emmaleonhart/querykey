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

use crate::models::{
    Conflict, ConflictResolution, ConflictType, DeliveryAttempt, Event, FollowUp, FollowUpStatus,
    Handle, OpenQuestion, Person, QuestionStatus, Task, TaskStatus, TriggerType, Urgency,
};

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

#[derive(Serialize, Deserialize)]
struct ConflictFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    conflict_type: ConflictType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    message_a: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    message_b: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    task: String,
    resolution: ConflictResolution,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolved_by: String,
    created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolved: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OpenQuestionFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    context: String,
    urgency: Urgency,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    urgency_deadline: Option<String>,
    trigger_type: TriggerType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    trigger_id: String,
    status: QuestionStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolution: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolved_by: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    resolved_via: String,
    created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolved: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct FollowUpFm {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    trigger_type: TriggerType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    trigger_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    context: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    delivery_attempts: Vec<DeliveryAttempt>,
    status: FollowUpStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    response: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    response_channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    response_at: Option<String>,
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
        for sub in [
            "people",
            "tasks",
            "events",
            "notes",
            "conflicts",
            "questions",
            "followups",
        ] {
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

    // ----- Conflict -----
    //
    // Body = the human `explanation` (why these two messages clash).
    // Everything structured/queryable is frontmatter.

    pub fn upsert_conflict(&self, c: &Conflict) -> anyhow::Result<()> {
        let fm = ConflictFm {
            id: format!("conflict:{}", c.id),
            kind: "conflict".into(),
            conflict_type: c.conflict_type,
            message_a: c.message_a.clone(),
            message_b: c.message_b.clone(),
            task: if c.task_id.is_empty() {
                String::new()
            } else {
                format!("task:{}", c.task_id)
            },
            resolution: c.resolution,
            resolved_by: c.resolved_by.clone(),
            created: rfc3339(&c.created_at),
            resolved: c.resolved_at.map(|d| rfc3339(&d)),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("conflicts", &c.id.to_string(), yaml, &c.explanation)
    }

    fn conflict_from(&self, content: &str) -> Option<Conflict> {
        let (yaml, body) = split(content);
        let fm: ConflictFm = serde_yaml::from_str(&yaml).ok()?;
        let id = uuid::Uuid::parse_str(fm.id.strip_prefix("conflict:").unwrap_or(&fm.id)).ok()?;
        Some(Conflict {
            id,
            conflict_type: fm.conflict_type,
            message_a: fm.message_a,
            message_b: fm.message_b,
            task_id: fm
                .task
                .strip_prefix("task:")
                .unwrap_or(&fm.task)
                .to_string(),
            explanation: body,
            resolution: fm.resolution,
            resolved_by: fm.resolved_by,
            created_at: parse_dt(&fm.created),
            resolved_at: fm.resolved.as_deref().map(parse_dt),
        })
    }

    pub fn get_conflict(&self, id: &uuid::Uuid) -> Option<Conflict> {
        let p = self.root.join("conflicts").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
        self.conflict_from(&s)
    }

    pub fn list_conflicts(&self) -> Vec<Conflict> {
        self.read_files("conflicts")
            .into_iter()
            .filter_map(|(_, c)| self.conflict_from(&c))
            .collect()
    }

    // ----- OpenQuestion -----
    //
    // Body = the human-facing `question`. The id is a readable slug
    // (string), not a UUID, so it doubles as the filename like Person.

    pub fn upsert_question(&self, q: &OpenQuestion) -> anyhow::Result<()> {
        let fm = OpenQuestionFm {
            id: format!("question:{}", q.id),
            kind: "question".into(),
            target: q.target.clone(),
            context: q.context.clone(),
            urgency: q.urgency,
            urgency_deadline: q.urgency_deadline.map(|d| rfc3339(&d)),
            trigger_type: q.trigger_type,
            trigger_id: q.trigger_id.clone(),
            status: q.status,
            resolution: q.resolution.clone(),
            resolved_by: q.resolved_by.clone(),
            resolved_via: q.resolved_via.clone(),
            created: rfc3339(&q.created_at),
            resolved: q.resolved_at.map(|d| rfc3339(&d)),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("questions", &q.id, yaml, &q.question)
    }

    fn question_from(&self, slug: &str, content: &str) -> Option<OpenQuestion> {
        let (yaml, body) = split(content);
        let fm: OpenQuestionFm = serde_yaml::from_str(&yaml).ok()?;
        let id = fm
            .id
            .strip_prefix("question:")
            .unwrap_or(slug)
            .to_string();
        Some(OpenQuestion {
            id,
            target: fm.target,
            question: body,
            context: fm.context,
            urgency: fm.urgency,
            urgency_deadline: fm.urgency_deadline.as_deref().map(parse_dt),
            trigger_type: fm.trigger_type,
            trigger_id: fm.trigger_id,
            status: fm.status,
            resolution: fm.resolution,
            resolved_by: fm.resolved_by,
            resolved_via: fm.resolved_via,
            created_at: parse_dt(&fm.created),
            resolved_at: fm.resolved.as_deref().map(parse_dt),
        })
    }

    pub fn get_question(&self, id: &str) -> Option<OpenQuestion> {
        let p = self.root.join("questions").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
        self.question_from(id, &s)
    }

    pub fn list_questions(&self) -> Vec<OpenQuestion> {
        self.read_files("questions")
            .into_iter()
            .filter_map(|(slug, c)| self.question_from(&slug, &c))
            .collect()
    }

    // ----- FollowUp -----
    //
    // Body = the human-facing `question`. Delivery attempts are a
    // nested frontmatter list (small, structured, round-trips via serde).

    pub fn upsert_followup(&self, f: &FollowUp) -> anyhow::Result<()> {
        let fm = FollowUpFm {
            id: format!("followup:{}", f.id),
            kind: "followup".into(),
            trigger_type: f.trigger_type,
            trigger_id: f.trigger_id.clone(),
            target: f.target.clone(),
            context: f.context.clone(),
            delivery_attempts: f.delivery_attempts.clone(),
            status: f.status,
            response: f.response.clone(),
            response_channel: f.response_channel.clone(),
            response_at: f.response_at.map(|d| rfc3339(&d)),
            created: rfc3339(&f.created_at),
        };
        let yaml = serde_yaml::to_string(&fm)?;
        self.write("followups", &f.id, yaml, &f.question)
    }

    fn followup_from(&self, slug: &str, content: &str) -> Option<FollowUp> {
        let (yaml, body) = split(content);
        let fm: FollowUpFm = serde_yaml::from_str(&yaml).ok()?;
        let id = fm
            .id
            .strip_prefix("followup:")
            .unwrap_or(slug)
            .to_string();
        Some(FollowUp {
            id,
            trigger_type: fm.trigger_type,
            trigger_id: fm.trigger_id,
            target: fm.target,
            question: body,
            context: fm.context,
            delivery_attempts: fm.delivery_attempts,
            status: fm.status,
            response: fm.response,
            response_channel: fm.response_channel,
            response_at: fm.response_at.as_deref().map(parse_dt),
            created_at: parse_dt(&fm.created),
        })
    }

    pub fn get_followup(&self, id: &str) -> Option<FollowUp> {
        let p = self.root.join("followups").join(format!("{id}.md"));
        let s = fs::read_to_string(p).ok()?;
        self.followup_from(id, &s)
    }

    pub fn list_followups(&self) -> Vec<FollowUp> {
        self.read_files("followups")
            .into_iter()
            .filter_map(|(slug, c)| self.followup_from(&slug, &c))
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

    #[test]
    fn conflict_question_followup_round_trip_losslessly() {
        use crate::models::{
            ConflictResolution, ConflictType, DeliveryAttempt, DeliveryStatus, FollowUpStatus,
            QuestionStatus, TriggerType, Urgency,
        };
        let dir = std::env::temp_dir().join(format!("qk-vault-{}", uuid::Uuid::new_v4()));
        let v = Vault::open(dir.to_str().unwrap()).unwrap();

        // --- Conflict ---
        let cid = uuid::Uuid::new_v4();
        let c = Conflict {
            id: cid,
            conflict_type: ConflictType::DeadlineChange,
            message_a: "msg-a".into(),
            message_b: "msg-b".into(),
            task_id: "11111111-1111-1111-1111-111111111111".into(),
            explanation: "Alice said Friday, Bob said Monday.\n\nNeeds a human call.".into(),
            resolution: ConflictResolution::AWins,
            resolved_by: "immanuelle".into(),
            created_at: DateTime::parse_from_rfc3339("2026-05-15T09:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
            resolved_at: Some(
                DateTime::parse_from_rfc3339("2026-05-15T10:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
        };
        v.upsert_conflict(&c).unwrap();
        let gc = v.get_conflict(&cid).unwrap();
        assert_eq!(gc.id, c.id);
        assert_eq!(gc.conflict_type, c.conflict_type);
        assert_eq!(gc.message_a, "msg-a");
        assert_eq!(gc.task_id, c.task_id);
        assert_eq!(gc.explanation, c.explanation);
        assert_eq!(gc.resolution, c.resolution);
        assert_eq!(gc.resolved_by, "immanuelle");
        assert_eq!(gc.created_at, c.created_at);
        assert_eq!(gc.resolved_at, c.resolved_at);
        assert_eq!(v.list_conflicts().len(), 1);

        // --- OpenQuestion ---
        let q = OpenQuestion {
            id: "deadline-for-johns-book".into(),
            target: "person:john-smith".into(),
            question: "When does John actually need the book back?".into(),
            context: "Casual mention; deadline is a guess.".into(),
            urgency: Urgency::ByTime,
            urgency_deadline: Some(
                DateTime::parse_from_rfc3339("2026-05-20T00:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            trigger_type: TriggerType::Ambiguity,
            trigger_id: "task:return-johns-book".into(),
            status: QuestionStatus::Open,
            resolution: String::new(),
            resolved_by: String::new(),
            resolved_via: String::new(),
            created_at: DateTime::parse_from_rfc3339("2026-05-15T09:00:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
            resolved_at: None,
        };
        v.upsert_question(&q).unwrap();
        let gq = v.get_question("deadline-for-johns-book").unwrap();
        assert_eq!(gq.id, q.id);
        assert_eq!(gq.target, q.target);
        assert_eq!(gq.question, q.question);
        assert_eq!(gq.context, q.context);
        assert_eq!(gq.urgency, q.urgency);
        assert_eq!(gq.urgency_deadline, q.urgency_deadline);
        assert_eq!(gq.trigger_type, q.trigger_type);
        assert_eq!(gq.status, q.status);
        assert_eq!(gq.created_at, q.created_at);
        assert_eq!(gq.resolved_at, None);
        assert_eq!(v.list_questions().len(), 1);

        // --- FollowUp ---
        let f = FollowUp {
            id: "ping-john-about-book".into(),
            trigger_type: TriggerType::UnconfirmedTask,
            trigger_id: "task:return-johns-book".into(),
            target: "person:john-smith".into(),
            question: "Still good to drop the book off Saturday?".into(),
            context: "No reply to the first nudge.".into(),
            delivery_attempts: vec![DeliveryAttempt {
                channel: "discord".into(),
                status: DeliveryStatus::Delivered,
                sent_at: DateTime::parse_from_rfc3339("2026-05-15T11:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            }],
            status: FollowUpStatus::Sent,
            response: String::new(),
            response_channel: String::new(),
            response_at: None,
            created_at: DateTime::parse_from_rfc3339("2026-05-15T10:30:00+00:00")
                .unwrap()
                .with_timezone(&Utc),
        };
        v.upsert_followup(&f).unwrap();
        let gf = v.get_followup("ping-john-about-book").unwrap();
        assert_eq!(gf.id, f.id);
        assert_eq!(gf.trigger_type, f.trigger_type);
        assert_eq!(gf.trigger_id, f.trigger_id);
        assert_eq!(gf.target, f.target);
        assert_eq!(gf.question, f.question);
        assert_eq!(gf.context, f.context);
        assert_eq!(gf.delivery_attempts.len(), 1);
        assert_eq!(gf.delivery_attempts[0].channel, "discord");
        assert_eq!(gf.delivery_attempts[0].status, DeliveryStatus::Delivered);
        assert_eq!(gf.delivery_attempts[0].sent_at, f.delivery_attempts[0].sent_at);
        assert_eq!(gf.status, f.status);
        assert_eq!(gf.created_at, f.created_at);
        assert_eq!(v.list_followups().len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

