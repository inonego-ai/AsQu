// ============================================================
// tests/question_store.rs — Session lifecycle, answer/dismiss, wait
// ============================================================

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::question_store::wait_for_answers_sync;
use crate::types::QuestionStatus;

use super::{answer, ask_item, make_state};

// ============================================================
// Session lifecycle
// ============================================================

#[test]
fn test_ensure_session_creates_once() {
    let shared = make_state();
    let mut st = shared.lock().unwrap();
    st.ensure_session("s1");
    st.ensure_session("s1"); // idempotent
    assert_eq!(st.session_order.len(), 1);
}

#[test]
fn test_add_question_links_to_session() {
    let shared = make_state();
    let mut st = shared.lock().unwrap();
    let q = st.add_question_to_session("s1", ask_item("Q?"));
    assert_eq!(q.session_id, "s1");
    assert_eq!(q.status, QuestionStatus::Pending);
    assert!(st.sessions["s1"].question_ids.contains(&q.id));
}

#[test]
fn test_cleanup_removes_session_keeps_questions() {
    let shared = make_state();
    let (q1, q2) = {
        let mut st = shared.lock().unwrap();
        let q1 = st.add_question_to_session("cleanup", ask_item("Q1"));
        let q2 = st.add_question_to_session("cleanup", ask_item("Q2"));
        (q1, q2)
    };
    // Answer q1 only — session not yet consumed
    { shared.lock().unwrap().apply_answer(&q1.id, answer()); }
    assert!(shared.lock().unwrap().sessions.contains_key("cleanup"), "session still active");

    // Answer q2 — all consumed, session auto-removed
    { shared.lock().unwrap().apply_answer(&q2.id, answer()); }
    let st = shared.lock().unwrap();
    assert!(!st.sessions.contains_key("cleanup"), "session should be auto-removed");
    assert!(st.questions.contains_key(&q1.id), "q1 must be preserved");
    assert!(st.questions.contains_key(&q2.id), "q2 must be preserved");
}

// ------------------------------------------------------------
// remove_session_with_questions: pending questions become Dismissed,
// session is removed, but questions are kept in state for in-flight waits.
// ------------------------------------------------------------
#[test]
fn test_remove_session_with_questions_dismisses_pending() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("rm", ask_item("Q?")) };
    { shared.lock().unwrap().remove_session_with_questions("rm"); }
    let st = shared.lock().unwrap();
    assert!(!st.sessions.contains_key("rm"), "session must be removed");
    // Questions are kept (not deleted) so in-flight wait callers can still read results
    assert!(st.questions.contains_key(&q.id), "question must be preserved for in-flight waits");
    assert_eq!(st.questions[&q.id].status, QuestionStatus::Dismissed, "pending question must be dismissed");
    assert_eq!(
        st.questions[&q.id].dismiss_reason.as_deref(),
        Some("session removed"),
        "dismiss reason must identify session removal"
    );
}

#[test]
fn test_multi_session_isolation() {
    let shared = make_state();
    let mut st = shared.lock().unwrap();
    st.add_question_to_session("a", ask_item("A1"));
    st.add_question_to_session("b", ask_item("B1"));
    st.add_question_to_session("b", ask_item("B2"));
    assert_eq!(st.get_session_question_ids("a").len(), 1);
    assert_eq!(st.get_session_question_ids("b").len(), 2);
    assert_eq!(st.get_session_question_ids("c").len(), 0); // non-existent
}

// ============================================================
// Answer / Dismiss
// ============================================================

#[test]
fn test_apply_answer_sets_status() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };
    let ok = shared.lock().unwrap().apply_answer(&q.id, answer());
    assert!(ok);
    assert_eq!(shared.lock().unwrap().questions[&q.id].status, QuestionStatus::Answered);
}

#[test]
fn test_apply_answer_rejects_non_pending() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };
    shared.lock().unwrap().apply_answer(&q.id, answer());
    // Second apply on already-answered question should fail
    let ok = shared.lock().unwrap().apply_answer(&q.id, answer());
    assert!(!ok);
}

#[test]
fn test_dismiss_questions_sets_dismissed() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };
    let dismissed = shared.lock().unwrap().dismiss_questions(&[q.id.clone()], Some("test"));
    assert_eq!(dismissed, vec![q.id.clone()]);
    assert_eq!(shared.lock().unwrap().questions[&q.id].status, QuestionStatus::Dismissed);
}

#[test]
fn test_apply_denied_sets_denied() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };
    let denied = shared.lock().unwrap().apply_denied(&[q.id.clone()], "denied by user");
    assert_eq!(denied, vec![q.id.clone()]);
    assert_eq!(shared.lock().unwrap().questions[&q.id].status, QuestionStatus::Denied);
}

// ============================================================
// get_answers buckets
// ============================================================

#[test]
fn test_get_answers_all_buckets() {
    let shared = make_state();
    let (q1, q2, q3) = {
        let mut st = shared.lock().unwrap();
        let q1 = st.add_question_to_session("s", ask_item("Q1"));
        let q2 = st.add_question_to_session("s", ask_item("Q2"));
        let q3 = st.add_question_to_session("s", ask_item("Q3"));
        (q1, q2, q3)
    };
    shared.lock().unwrap().apply_answer(&q1.id, answer());
    shared.lock().unwrap().apply_denied(&[q2.id.clone()], "denied");
    // q3 stays pending

    let st = shared.lock().unwrap();
    let result = st.get_answers(&[q1.id.clone(), q2.id.clone(), q3.id.clone()]);
    assert_eq!(result.answered.len(), 1);
    assert_eq!(result.denied.len(), 1);
    assert_eq!(result.pending, vec![q3.id.clone()]);
    assert!(!result.timed_out);
}

#[test]
fn test_get_answers_dismissed_maps_to_denied() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };
    shared.lock().unwrap().dismiss_questions(&[q.id.clone()], None);
    let st = shared.lock().unwrap();
    let result = st.get_answers(&[q.id.clone()]);
    // Dismissed must appear in denied bucket
    assert_eq!(result.denied.len(), 1);
    assert!(result.answered.is_empty());
}

// ============================================================
// wait_for_answers_sync
// ============================================================

#[test]
fn test_wait_resolves_on_answer() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };

    let shared2 = Arc::clone(&shared);
    let qid = q.id.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        shared2.lock().unwrap().apply_answer(&qid, answer());
    });

    let result = wait_for_answers_sync(&shared, &[q.id.clone()], false, Some(5));
    assert_eq!(result.answered.len(), 1);
    assert!(!result.timed_out);
}

#[test]
fn test_wait_resolves_on_dismiss() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };

    let shared2 = Arc::clone(&shared);
    let qid = q.id.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        shared2.lock().unwrap().dismiss_questions(&[qid], None);
    });

    let result = wait_for_answers_sync(&shared, &[q.id.clone()], false, Some(5));
    assert_eq!(result.denied.len(), 1);
    assert!(!result.timed_out);
}

#[test]
fn test_wait_timeout() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };

    let result = wait_for_answers_sync(&shared, &[q.id.clone()], false, Some(1));
    assert!(result.timed_out);
    assert!(result.answered.is_empty());
}

#[test]
fn test_wait_require_all_waits_for_every_question() {
    let shared = make_state();
    let (q1, q2) = {
        let mut st = shared.lock().unwrap();
        (
            st.add_question_to_session("s", ask_item("Q1")),
            st.add_question_to_session("s", ask_item("Q2")),
        )
    };

    let shared2 = Arc::clone(&shared);
    let (id1, id2) = (q1.id.clone(), q2.id.clone());
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        shared2.lock().unwrap().apply_answer(&id1, answer());
        thread::sleep(Duration::from_millis(50));
        shared2.lock().unwrap().apply_answer(&id2, answer());
    });

    let result = wait_for_answers_sync(&shared, &[q1.id.clone(), q2.id.clone()], true, Some(5));
    assert_eq!(result.answered.len(), 2);
    assert!(result.pending.is_empty());
}

#[test]
fn test_wait_already_answered_returns_immediately() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };
    shared.lock().unwrap().apply_answer(&q.id, answer());

    let result = wait_for_answers_sync(&shared, &[q.id.clone()], false, Some(1));
    assert_eq!(result.answered.len(), 1);
    assert!(!result.timed_out);
}

// ============================================================
// instant question — resolves wait early even with require_all=true
// ============================================================

#[test]
fn test_instant_answer_resolves_require_all_early() {
    let shared = make_state();
    let (q_instant, q_normal) = {
        let mut st = shared.lock().unwrap();
        let mut item = ask_item("Instant Q?");
        item.instant = true;
        let qi = st.add_question_to_session("s", item);
        let qn = st.add_question_to_session("s", ask_item("Normal Q?"));
        (qi, qn)
    };

    let shared2 = Arc::clone(&shared);
    let instant_id = q_instant.id.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        shared2.lock().unwrap().apply_answer(&instant_id, answer());
        // q_normal intentionally left pending
    });

    // require_all=true: normally waits for every question, but instant answer overrides that
    let result = wait_for_answers_sync(&shared, &[q_instant.id.clone(), q_normal.id.clone()], true, Some(5));
    assert!(!result.timed_out, "must not time out");
    assert_eq!(result.answered.len(), 1, "instant question is answered");
    assert_eq!(result.pending, vec![q_normal.id.clone()], "normal question still pending");
}

// ============================================================
// shutdown — begin_shutdown unblocks wait with shutdown=true
// ============================================================

#[test]
fn test_shutdown_unblocks_wait() {
    let shared = make_state();
    let q = { let mut st = shared.lock().unwrap(); st.add_question_to_session("s", ask_item("Q?")) };

    let shared2 = Arc::clone(&shared);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        shared2.lock().unwrap().begin_shutdown();
    });

    let result = wait_for_answers_sync(&shared, &[q.id.clone()], false, Some(5));
    assert!(result.shutdown, "shutdown flag must be set");
    assert!(!result.timed_out, "must not time out");
    assert_eq!(result.denied.len(), 1, "dismissed question appears in denied");
    assert_eq!(result.denied[0].reason, "app shutdown");
}

// ============================================================
// derive_display_name (tested indirectly via ensure_session)
// ============================================================

#[test]
fn test_display_name_short_id() {
    let shared = make_state();
    let mut st = shared.lock().unwrap();
    let name = st.ensure_session("short");
    assert_eq!(name, "short"); // <= 12 chars: returned as-is
}

#[test]
fn test_display_name_long_ascii() {
    let shared = make_state();
    let mut st = shared.lock().unwrap();
    let name = st.ensure_session("abcdef-1234-5678-90ab"); // 21 chars
    assert_eq!(name.len(), 8); // last 8 bytes (ASCII)
    assert_eq!(name, "678-90ab");
}

#[test]
fn test_display_name_multibyte_no_panic() {
    let shared = make_state();
    let mut st = shared.lock().unwrap();
    // 13 Korean chars (> 12 threshold) — each is 3 bytes; byte-based slicing would panic
    let name = st.ensure_session("세션안녕하세요테스트하나셋");
    assert_eq!(name.chars().count(), 8, "must return last 8 characters, not bytes");
}
