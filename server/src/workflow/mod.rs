//! Status-workflow enforcement.
//!
//! Each lifecycle entity has a small state machine. These rules are
//! enforced **at the API mutation boundary only** — NOT in the vault.
//! The vault must still be able to represent any state, because the
//! markdown files are the user's own and hand-edits (or an Obsidian
//! import) are always legitimate ("the tool serves you"). What we
//! refuse is the *server* making an illegal jump (e.g. silently
//! un-resolving a conflict) behind the user's back.
//!
//! `from == to` is always allowed (idempotent writes).

use crate::models::{ConflictResolution, QuestionStatus, TaskStatus};

/// Task: `extracted → confirmed → in_progress → done`, with `disputed`
/// as a reachable side-state and explicit *reopen* edges out of
/// `done`. A `done` task may be reopened (it turned out not done) but
/// never silently rewound to `extracted` (that would lose its
/// lifecycle).
pub fn task_transition_ok(from: TaskStatus, to: TaskStatus) -> bool {
    use TaskStatus::*;
    if from == to {
        return true;
    }
    match (from, to) {
        (Extracted, Confirmed | InProgress | Disputed | Done) => true,
        (Confirmed, InProgress | Done | Disputed) => true,
        (InProgress, Done | Disputed | Confirmed) => true,
        (Disputed, Extracted | Confirmed | InProgress | Done) => true,
        (Done, InProgress | Disputed) => true, // reopen only
        _ => false,
    }
}

/// Conflict: the user's rule — **a resolved conflict can never return
/// to `unresolved`**. From `unresolved` you may pick any resolution;
/// among resolved states you may *correct* the call
/// (`a_wins → merged`); you may never rewind to `unresolved`.
pub fn conflict_transition_ok(from: ConflictResolution, to: ConflictResolution) -> bool {
    if from == to {
        return true; // idempotent re-write
    }
    // The whole rule: any move is fine EXCEPT (re-)entering
    // `unresolved`. unresolved→resolved and resolved→resolved
    // (correction) both pass; resolved→unresolved is refused.
    to != ConflictResolution::Unresolved
}

/// Open question: `open → resolved | expired`. An `expired` question
/// can be re-asked (`→ open`) or resolved; a `resolved` one is
/// terminal (resolved on any platform → it's gone from the queue).
pub fn question_transition_ok(from: QuestionStatus, to: QuestionStatus) -> bool {
    use QuestionStatus::*;
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        (Open, Resolved | Expired) | (Expired, Open | Resolved)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_machine() {
        use TaskStatus::*;
        assert!(task_transition_ok(Extracted, Confirmed));
        assert!(task_transition_ok(Confirmed, InProgress));
        assert!(task_transition_ok(InProgress, Done));
        assert!(task_transition_ok(Done, InProgress)); // reopen
        assert!(task_transition_ok(Done, Done)); // idempotent
        assert!(!task_transition_ok(Done, Extracted)); // never rewind
        assert!(!task_transition_ok(Confirmed, Extracted));
    }

    #[test]
    fn conflict_never_unresolves() {
        use ConflictResolution::*;
        assert!(conflict_transition_ok(Unresolved, AWins));
        assert!(conflict_transition_ok(AWins, Merged)); // correction
        assert!(conflict_transition_ok(Unresolved, Unresolved));
        assert!(!conflict_transition_ok(AWins, Unresolved)); // the rule
        assert!(!conflict_transition_ok(Dismissed, Unresolved));
    }

    #[test]
    fn question_machine() {
        use QuestionStatus::*;
        assert!(question_transition_ok(Open, Resolved));
        assert!(question_transition_ok(Open, Expired));
        assert!(question_transition_ok(Expired, Open)); // re-ask
        assert!(!question_transition_ok(Resolved, Open)); // terminal
        assert!(!question_transition_ok(Resolved, Expired));
    }
}
