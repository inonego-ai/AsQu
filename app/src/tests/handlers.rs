// ============================================================
// tests/handlers.rs — IPC handler routing (resolve_ids)
// ============================================================

use crate::ipc::handlers::resolve_ids;

use super::{ask_item, make_state};

// ------------------------------------------------------------
// resolve_ids: explicit ids take priority
// ------------------------------------------------------------
#[test]
fn test_resolve_ids_explicit() {
    let shared = make_state();
    shared.lock().unwrap().add_question_to_session("s", ask_item("Q?"));

    let ids = resolve_ids(&shared, "s", vec!["99".to_string()]);
    assert_eq!(ids, vec!["99"], "explicit ids must be returned as-is");
}

// ------------------------------------------------------------
// resolve_ids: falls back to session question_ids when ids empty
// ------------------------------------------------------------
#[test]
fn test_resolve_ids_from_session() {
    let shared = make_state();
    let q = shared.lock().unwrap().add_question_to_session("s", ask_item("Q?"));

    let ids = resolve_ids(&shared, "s", vec![]);
    assert_eq!(ids, vec![q.id], "should return session question ids");
}

// ------------------------------------------------------------
// resolve_ids: fallback to questions map after session auto-cleanup
// ------------------------------------------------------------
#[test]
fn test_resolve_ids_fallback_after_session_removed() {
    let shared = make_state();
    let q = shared.lock().unwrap().add_question_to_session("s", ask_item("Q?"));

    // Dismiss → triggers cleanup → session removed, question kept
    shared.lock().unwrap().apply_denied(&[q.id.clone()], "test");
    assert!(
        !shared.lock().unwrap().sessions.contains_key("s"),
        "session should be auto-cleaned"
    );
    assert!(
        shared.lock().unwrap().questions.contains_key(&q.id),
        "question must be preserved after auto-cleanup"
    );

    // resolve_ids with empty ids must find the question via fallback search
    let ids = resolve_ids(&shared, "s", vec![]);
    assert_eq!(ids, vec![q.id], "fallback search must find question by session_id field");
}

// ------------------------------------------------------------
// resolve_ids: unknown session with no questions returns empty
// ------------------------------------------------------------
#[test]
fn test_resolve_ids_unknown_session() {
    let shared = make_state();
    let ids = resolve_ids(&shared, "nonexistent", vec![]);
    assert!(ids.is_empty(), "unknown session with no questions must return empty");
}

// ------------------------------------------------------------
// resolve_ids: multiple questions in auto-cleaned session
// ------------------------------------------------------------
#[test]
fn test_resolve_ids_multiple_fallback() {
    let shared = make_state();
    let q1 = shared.lock().unwrap().add_question_to_session("s", ask_item("Q1"));
    let q2 = shared.lock().unwrap().add_question_to_session("s", ask_item("Q2"));

    // Dismiss both → session auto-removed
    shared.lock().unwrap().apply_denied(&[q1.id.clone(), q2.id.clone()], "done");

    let mut ids = resolve_ids(&shared, "s", vec![]);
    ids.sort();
    let mut expected = vec![q1.id.clone(), q2.id.clone()];
    expected.sort();
    assert_eq!(ids, expected, "all questions must be found via fallback");
}
