//! Dashboard read-side: parse `applications.md` and `plans.md` at the
//! vault root into structured items for the QueryKey desktop Dashboard
//! view (R22 plan, queue.md).
//!
//! The vault markdown is the source of truth. The dashboard never
//! writes — notes-write is R23.
//!
//! Expected markdown shape (set up in the life-planning vault at
//! commit `3b203f8c`):
//!
//! ```text
//! ## Section title
//!
//! ### Item title
//! - **field-key:** field-value (single line)
//! - **notes:** can span until the next item / section / bullet
//! ```
//!
//! Each item collects: section, title, ordered field-key → field-value
//! pairs. Field values are everything after the `**key:**` marker until
//! the next bullet, blank line, or section break.
//!
//! Sections that look like prose (no `### ` H3 children) are still
//! returned but with an empty items list — useful for the UI to render
//! header text where present.

use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct DashboardItem {
    pub section: String,
    pub title: String,
    /// Ordered (key, value) pairs from the `**key:** value` bullets.
    pub fields: Vec<(String, String)>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Dashboard {
    pub source_path: String,
    pub items: Vec<DashboardItem>,
}

/// Parse a dashboard markdown file from the vault root. Returns `None`
/// if the file is missing.
pub fn parse(vault_root: &Path, file_name: &str) -> Option<Dashboard> {
    let path = vault_root.join(file_name);
    let text = fs::read_to_string(&path).ok()?;
    Some(parse_text(&path.display().to_string(), &text))
}

pub fn parse_text(source_path: &str, text: &str) -> Dashboard {
    let mut items: Vec<DashboardItem> = Vec::new();
    let mut cur_section: String = String::new();
    let mut cur_item: Option<DashboardItem> = None;
    let mut cur_field: Option<(String, String)> = None;

    let finish_field = |cur_field: &mut Option<(String, String)>,
                        cur_item: &mut Option<DashboardItem>| {
        if let Some((k, v)) = cur_field.take() {
            if let Some(it) = cur_item.as_mut() {
                it.fields.push((k, v.trim().to_string()));
            }
        }
    };

    let finish_item = |cur_field: &mut Option<(String, String)>,
                       cur_item: &mut Option<DashboardItem>,
                       items: &mut Vec<DashboardItem>| {
        finish_field(cur_field, cur_item);
        if let Some(it) = cur_item.take() {
            items.push(it);
        }
    };

    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("## ") {
            // Section header
            finish_item(&mut cur_field, &mut cur_item, &mut items);
            cur_section = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            finish_item(&mut cur_field, &mut cur_item, &mut items);
            cur_item = Some(DashboardItem {
                section: cur_section.clone(),
                title: rest.trim().to_string(),
                fields: Vec::new(),
            });
            continue;
        }
        // Bullet (- **key:** value...)
        if cur_item.is_some() {
            if let Some(rest) = trimmed.strip_prefix("- ") {
                if let Some((key, value)) = parse_bullet(rest) {
                    finish_field(&mut cur_field, &mut cur_item);
                    cur_field = Some((key, value));
                    continue;
                }
            }
            // Continuation of the current field's value (indented or
            // continuation lines). Append to the current field value.
            if cur_field.is_some() {
                let stripped = line.trim();
                if stripped.is_empty() {
                    // blank line ends the field
                    finish_field(&mut cur_field, &mut cur_item);
                    continue;
                }
                if let Some((_, v)) = cur_field.as_mut() {
                    if !v.is_empty() {
                        v.push('\n');
                    }
                    v.push_str(stripped);
                    continue;
                }
            }
        }
    }
    finish_item(&mut cur_field, &mut cur_item, &mut items);
    Dashboard {
        source_path: source_path.to_string(),
        items,
    }
}

/// Pull `**key:** value` out of a bullet body. Returns `None` if the
/// bullet doesn't look like a key/value bullet (e.g. a plain bullet).
fn parse_bullet(body: &str) -> Option<(String, String)> {
    let b = body.trim_start();
    let rest = b.strip_prefix("**")?;
    let close = rest.find("**")?;
    let key_with_colon = &rest[..close];
    let key = key_with_colon.trim_end_matches(':').trim().to_string();
    let after = rest[close + 2..].trim();
    let value = after.strip_prefix(':').unwrap_or(after).trim().to_string();
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_structure() {
        let md = r"
# Title

## Active

### SOAR — apply
- **next-action-date:** 2026-06-08
- **status:** drafting

### 80k advising
- **status:** stage 1 not yet
- **notes:**
";
        let d = parse_text("test", md);
        assert_eq!(d.items.len(), 2);
        assert_eq!(d.items[0].section, "Active");
        assert_eq!(d.items[0].title, "SOAR — apply");
        assert_eq!(d.items[0].fields.len(), 2);
        assert_eq!(d.items[0].fields[0], ("next-action-date".to_string(), "2026-06-08".to_string()));
    }

    #[test]
    fn handles_multiline_notes() {
        let md = "## S\n### Item\n- **notes:**\nfirst line\nsecond line\n\n### Next item\n";
        let d = parse_text("test", md);
        assert_eq!(d.items.len(), 2);
        assert_eq!(d.items[0].fields[0].0, "notes");
        assert!(d.items[0].fields[0].1.contains("first line"));
        assert!(d.items[0].fields[0].1.contains("second line"));
    }
}
