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

/// R23 notes-write. Set (replace) the `notes` field of the item titled
/// `item_title` in `file_name` (must be a dashboard file at the vault
/// root) to `new_notes`, writing the markdown back in place. Returns the
/// number of bytes written on success.
///
/// Local-first like every other vault write in this server: it writes
/// the file, it does NOT git-commit. The vault is a git repo the user
/// (or her flush automation) commits out of band — having the server
/// race those on the git index would be the wrong layer.
pub fn set_note(
    vault_root: &Path,
    file_name: &str,
    item_title: &str,
    new_notes: &str,
) -> Result<usize, String> {
    if file_name != "applications.md" && file_name != "plans.md" {
        return Err(format!("file not allowed: {file_name}"));
    }
    let path = vault_root.join(file_name);
    let text = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let updated = set_note_in_text(&text, item_title, new_notes)
        .ok_or_else(|| format!("item not found: {item_title}"))?;
    fs::write(&path, &updated).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(updated.len())
}

/// Pure text transform behind [`set_note`]. Finds the `### item_title`
/// block, replaces its `- **notes:**` field (and any continuation lines)
/// with the formatted `new_notes`, and returns the new file text.
/// Returns `None` if the item title isn't found. Preserves the file's
/// dominant line ending.
pub fn set_note_in_text(text: &str, item_title: &str, new_notes: &str) -> Option<String> {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.lines().collect();
    let want = item_title.trim();

    // Locate the target item block [start, end).
    let mut start = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("### ") {
            if rest.trim() == want {
                start = Some(i);
                break;
            }
        }
    }
    let start = start?;
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let t = line.trim_start();
        if t.starts_with("### ") || t.starts_with("## ") {
            end = i;
            break;
        }
    }

    // Within the block, find the `- **notes:**` bullet and the extent of
    // its value (up to the next bullet / section / end of block).
    let mut note_at = None;
    for i in (start + 1)..end {
        let t = lines[i].trim_start();
        if let Some(rest) = t.strip_prefix("- ") {
            if let Some((k, _)) = parse_bullet(rest) {
                if k.eq_ignore_ascii_case("notes") {
                    note_at = Some(i);
                    break;
                }
            }
        }
    }

    let formatted = format_note_bullet(new_notes);
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 2);

    if let Some(ni) = note_at {
        // The field value runs from ni until the next bullet / section /
        // blank line — matching the parser, where a blank line ends a
        // field. Stopping at the blank preserves the blank separator
        // before the next section, so a cleared note round-trips clean.
        let mut ve = end;
        for i in (ni + 1)..end {
            let t = lines[i].trim_start();
            if t.is_empty() || t.starts_with("- ") || t.starts_with('#') {
                ve = i;
                break;
            }
        }
        out.extend(lines[..ni].iter().map(|s| s.to_string()));
        out.extend(formatted);
        out.extend(lines[ve..].iter().map(|s| s.to_string()));
    } else {
        // No notes field yet: append after the last bullet in the block
        // (else right after the title line).
        let mut insert_at = start + 1;
        for i in (start + 1)..end {
            if lines[i].trim_start().starts_with("- ") {
                insert_at = i + 1;
            }
        }
        out.extend(lines[..insert_at].iter().map(|s| s.to_string()));
        out.extend(formatted);
        out.extend(lines[insert_at..].iter().map(|s| s.to_string()));
    }

    let mut joined = out.join(nl);
    if text.ends_with('\n') {
        joined.push_str(nl);
    }
    Some(joined)
}

/// Format a (possibly multi-line) note into markdown bullet lines:
/// `- **notes:** <first line>` then 2-space-indented continuation lines.
fn format_note_bullet(notes: &str) -> Vec<String> {
    let trimmed = notes.trim_end_matches(['\n', '\r']);
    if trimmed.trim().is_empty() {
        return vec!["- **notes:**".to_string()];
    }
    let mut lines = trimmed.split('\n').map(|l| l.trim_end_matches('\r'));
    let first = lines.next().unwrap_or("");
    let mut out = vec![format!("- **notes:** {}", first.trim())];
    for l in lines {
        out.push(format!("  {}", l.trim()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_existing_note_single_line() {
        let md = "## Active\n### SOAR — apply\n- **status:** drafting\n- **notes:**\n\n## Next\n";
        let out = set_note_in_text(md, "SOAR — apply", "called them today").unwrap();
        assert!(out.contains("- **notes:** called them today"));
        assert!(out.contains("- **status:** drafting"));
        assert!(out.contains("## Next"));
        // Re-parse round-trips the note value.
        let d = parse_text("t", &out);
        let it = d.items.iter().find(|i| i.title == "SOAR — apply").unwrap();
        let note = it.fields.iter().find(|(k, _)| k == "notes").unwrap();
        assert_eq!(note.1, "called them today");
    }

    #[test]
    fn replaces_multiline_note_and_preserves_following_section() {
        let md = "## A\n### Item\n- **x:** 1\n- **notes:** old note\n  more old\n\n## B\n### Other\n- **y:** 2\n";
        let out = set_note_in_text(md, "Item", "new line").unwrap();
        assert!(out.contains("- **notes:** new line"));
        assert!(!out.contains("old note"));
        assert!(out.contains("## B"));
        assert!(out.contains("### Other"));
        assert!(out.contains("- **y:** 2"));
    }

    #[test]
    fn set_then_clear_preserves_blank_separator() {
        let md = "## A\n### Item\n- **status:** open\n- **notes:**\n\n### Next\n- **z:** 9\n";
        let set = set_note_in_text(md, "Item", "temp").unwrap();
        assert!(set.contains("- **notes:** temp"));
        let cleared = set_note_in_text(&set, "Item", "").unwrap();
        // Clearing returns to the original text byte-for-byte.
        assert_eq!(cleared, md);
    }

    #[test]
    fn missing_item_returns_none() {
        let md = "## A\n### Item\n- **notes:**\n";
        assert!(set_note_in_text(md, "Nope", "x").is_none());
    }

    #[test]
    fn appends_note_when_absent() {
        let md = "## A\n### Item\n- **status:** open\n";
        let out = set_note_in_text(md, "Item", "first note").unwrap();
        assert!(out.contains("- **notes:** first note"));
        let d = parse_text("t", &out);
        let it = &d.items[0];
        assert!(it.fields.iter().any(|(k, v)| k == "notes" && v == "first note"));
    }

    #[test]
    fn rejects_disallowed_file() {
        let p = Path::new(".");
        assert!(set_note(p, "../secrets.md", "x", "y").is_err());
    }

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
