#!/usr/bin/env bash
# ============================================================
# AsQu Integration Test Suite
# ============================================================
# Runs all CLI commands and verifies expected output.
# IDs are captured dynamically — no hardcoded counter values.
# Usage: bash test.sh [path/to/asqu.exe]
# ============================================================

set -euo pipefail

ASQU="${1:-target/debug/asqu.exe}"
PASS=0
FAIL=0

# ------------------------------------------------------------
# Helpers
# ------------------------------------------------------------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

pass() { echo -e "${GREEN}[PASS]${NC} $1"; PASS=$((PASS + 1)); }
fail() { echo -e "${RED}[FAIL]${NC} $1"; FAIL=$((FAIL + 1)); }
info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

run() {
  # run <SESSION_ID> <args...>
  local sid="$1"; shift
  CLAUDE_SESSION_ID="$sid" "$ASQU" "$@" 2>/dev/null
}

contains() {
  # contains <string> <substring>
  [[ "$1" == *"$2"* ]]
}

assert_contains() {
  local label="$1" actual="$2" expected="$3"
  if contains "$actual" "$expected"; then
    pass "$label"
  else
    fail "$label — expected '$expected' in: $actual"
  fi
}

assert_not_contains() {
  local label="$1" actual="$2" unexpected="$3"
  if ! contains "$actual" "$unexpected"; then
    pass "$label"
  else
    fail "$label — did NOT expect '$unexpected' in: $actual"
  fi
}

# Extract first ID from '"ids":["N"]'
extract_id() {
  echo "$1" | grep -o '"ids":\["[^"]*"\]' | grep -o '"[0-9][0-9]*"' | head -1 | tr -d '"'
}

# ------------------------------------------------------------
# Kill ALL stale processes, then start a fresh server
# ------------------------------------------------------------
info "Killing stale asqu processes..."
powershell.exe -Command "Get-Process asqu -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id \$_.Id -Force }" 2>/dev/null || true
sleep 1

info "Starting fresh asqu server..."
"$ASQU" &
SERVER_PID=$!

# Wait until the IPC server is ready (up to 20 seconds)
info "Waiting for server to be ready..."
READY=0
for i in $(seq 1 40); do
  R=$("$ASQU" open 2>/dev/null || true)
  if contains "$R" '"result":"ui_ok"'; then
    READY=1
    break
  fi
  sleep 0.5
done

if [[ "$READY" -ne 1 ]]; then
  echo -e "${RED}[ERROR]${NC} asqu server did not start within 20 seconds"
  exit 1
fi
info "Server ready."

echo ""
echo "============================================================"
echo " AsQu Integration Tests"
echo "============================================================"

# ============================================================
# TEST 1: ask — single question
# ============================================================
echo ""
info "--- Test 1: ask (single) ---"
R=$(run t1 ask '[{"text":"Single question","choices":["A","B","C"]}]')
assert_contains "ask returns ask_ok"       "$R" '"result":"ask_ok"'
assert_contains "ask returns ids array"    "$R" '"ids":['
assert_contains "ask returns pending"      "$R" '"pending":'
T1_ID=$(extract_id "$R")
info "   t1 question id=$T1_ID"

# ============================================================
# TEST 2: ask — multiple questions in same session
# ============================================================
echo ""
info "--- Test 2: ask (multiple, same session) ---"
R2=$(run t2 ask '[{"text":"Multi Q1","choices":["X","Y"]}]')
assert_contains "ask multi Q1 ok"     "$R2" '"result":"ask_ok"'
assert_contains "ask multi Q1 ids"    "$R2" '"ids":['
T2_ID1=$(extract_id "$R2")

R3=$(run t2 ask '[{"text":"Multi Q2","choices":["1","2"]}]')
assert_contains "ask multi Q2 ok"     "$R3" '"result":"ask_ok"'
assert_contains "ask multi Q2 ids"    "$R3" '"ids":['
T2_ID2=$(extract_id "$R3")
info "   t2 question ids=$T2_ID1,$T2_ID2"

# IDs in same session must be different
if [[ "$T2_ID1" != "$T2_ID2" ]]; then
  pass "ask multi: different ids for two questions"
else
  fail "ask multi: same id returned for two questions"
fi

# IDs must be monotonically increasing
if [[ "$T2_ID2" -gt "$T2_ID1" ]]; then
  pass "ask multi: ids are increasing"
else
  fail "ask multi: ids not increasing ($T2_ID1 -> $T2_ID2)"
fi

# ============================================================
# TEST 3: get — non-blocking snapshot
# ============================================================
echo ""
info "--- Test 3: get (non-blocking) ---"
R=$(run t2 get)
assert_contains "get returns answers_ok"           "$R" '"result":"answers_ok"'
assert_contains "get t2 shows pending T2_ID1"      "$R" "\"$T2_ID1\""
assert_contains "get t2 shows pending T2_ID2"      "$R" "\"$T2_ID2\""

R=$(run t1 get)
assert_contains "get t1 returns answers_ok"        "$R" '"result":"answers_ok"'
assert_contains "get t1 shows pending T1_ID"       "$R" "\"$T1_ID\""

R=$(run t2 get "$T2_ID1")
assert_contains "get by id returns pending"        "$R" '"pending":['
assert_contains "get by id shows T2_ID1"           "$R" "\"$T2_ID1\""

# ============================================================
# TEST 4: dismiss — cancel pending questions
# ============================================================
echo ""
info "--- Test 4: dismiss ---"
R=$(run t2 dismiss "$T2_ID1")
assert_contains "dismiss single ok"               "$R" '"result":"dismiss_ok"'
assert_contains "dismiss returns T2_ID1"          "$R" "\"$T2_ID1\""

R=$(run t2 get "$T2_ID1")
assert_contains "get after dismiss: denied"       "$R" '"denied"'
assert_not_contains "get after dismiss: no pending" "$R" '"pending"'

R=$(run t2 dismiss)
assert_contains "dismiss all t2 ok"               "$R" '"result":"dismiss_ok"'
assert_contains "dismiss all returns T2_ID2"      "$R" "\"$T2_ID2\""

# ============================================================
# TEST 5: get after full dismiss
# ============================================================
echo ""
info "--- Test 5: get after full dismiss ---"
R=$(run t2 get)
assert_not_contains "t2 fully consumed: no pending" "$R" '"pending"'

# ============================================================
# TEST 6: wait — simulate resolution via dismiss
# ============================================================
echo ""
info "--- Test 6: wait (auto) ---"
info "   Dismissing t1 question to simulate wait resolution..."
R=$(run t1 dismiss "$T1_ID")
assert_contains "dismiss t1 ok"                   "$R" '"result":"dismiss_ok"'
assert_contains "dismiss t1 returns T1_ID"        "$R" "\"$T1_ID\""

# wait should return immediately: question already dismissed (denied)
R=$(run t1 wait "$T1_ID" --timeout 5)
assert_contains "wait returns answers_ok"         "$R" '"result":"answers_ok"'
assert_contains "wait shows denied"               "$R" '"denied"'
assert_not_contains "wait: no pending"            "$R" '"pending"'

# ============================================================
# TEST 7: wait — session cleanup fallback
# ============================================================
echo ""
info "--- Test 7: wait fallback after session auto-cleanup ---"
R4=$(run t3 ask '[{"text":"Fallback test","choices":["Yes","No"]}]')
assert_contains "ask t3 ok"   "$R4" '"result":"ask_ok"'
T3_ID=$(extract_id "$R4")
info "   Registered question id=$T3_ID"

# Dismiss triggers session auto-cleanup (all questions dismissed)
R=$(run t3 dismiss "$T3_ID")
assert_contains "dismiss t3 ok" "$R" '"dismissed"'

# wait without explicit IDs — session gone, fallback searches questions map by session_id
R=$(run t3 wait --timeout 2)
assert_contains "wait t3 fallback: answers_ok" "$R" '"result":"answers_ok"'
assert_contains "wait t3 fallback: denied"     "$R" '"denied"'

# ============================================================
# TEST 8: wait --require-all
# ============================================================
echo ""
info "--- Test 8: wait --require-all ---"
RA=$(run t4 ask '[{"text":"RA Q1","choices":["A","B"]}]')
RB=$(run t4 ask '[{"text":"RA Q2","choices":["C","D"]}]')
T4_ID1=$(extract_id "$RA")
T4_ID2=$(extract_id "$RB")
info "   t4 question ids=$T4_ID1,$T4_ID2"

R=$(run t4 get)
assert_contains "t4 get shows both pending" "$R" '"pending":['

# Dismiss all t4 questions
run t4 dismiss > /dev/null

R=$(run t4 wait --require-all --timeout 2)
assert_contains "wait require-all: answers_ok" "$R" '"result":"answers_ok"'
assert_contains "wait require-all: denied"     "$R" '"denied"'

# ============================================================
# TEST 9: wait timeout
# ============================================================
echo ""
info "--- Test 9: wait timeout ---"
run t5 ask '[{"text":"Timeout test","choices":["A","B"]}]' > /dev/null

R=$(run t5 wait --timeout 2)
assert_contains "wait timeout returns answers_ok" "$R" '"result":"answers_ok"'
assert_contains "wait timeout has timed_out"      "$R" '"timedOut":true'

# Cleanup
run t5 dismiss > /dev/null

# ============================================================
# TEST 10: open
# ============================================================
echo ""
info "--- Test 10: open ---"
R=$("$ASQU" open 2>/dev/null)
assert_contains "open returns ui_ok" "$R" '"result":"ui_ok"'

# ============================================================
# TEST 11: process count — no duplicate GUI spawning
# ============================================================
echo ""
info "--- Test 11: single process ---"
COUNT=$(powershell.exe -Command "Get-Process asqu -ErrorAction SilentlyContinue | Measure-Object | Select-Object -ExpandProperty Count" 2>/dev/null | tr -d '[:space:]')
if [[ "$COUNT" == "1" ]]; then
  pass "Only 1 asqu process running (no duplicate GUI spawning)"
else
  fail "Expected 1 process, found $COUNT"
fi

# ============================================================
# TEST 12: choices with description field
# ============================================================
echo ""
info "--- Test 12: choices with description ---"

R=$(run t_desc ask '[
  {"text":"Description test","choices":[
    {"label":"Alpha","description":"First option with a longer explanation"},
    {"label":"Beta","description":"Second option — also has detail"},
    {"label":"Gamma"}
  ],"category":"Test"}
]')
assert_contains "ask with description: ask_ok"    "$R" '"result":"ask_ok"'
assert_contains "ask with description: ids"       "$R" '"ids":['
T_DESC_ID=$(extract_id "$R")

# Dismiss immediately — we just need ask to succeed
R=$(run t_desc dismiss "$T_DESC_ID")
assert_contains "dismiss description question ok" "$R" '"result":"dismiss_ok"'

# ======================================================================
# TEST 13: live UI answer — full end-to-end path (with description)
# ======================================================================
echo ""
info "--- Test 13: live answer with description (requires UI interaction) ---"

# Open the window so it's visible
"$ASQU" open 2>/dev/null > /dev/null

# Register the question with descriptions so user can verify rendering
R12=$(run t6 ask '[{"text":"UI 테스트: Alpha를 클릭해 주세요","choices":[
  {"label":"Alpha","description":"이 선택지를 클릭하세요 — description 렌더링 확인용"},
  {"label":"Beta","description":"클릭하지 마세요"},
  {"label":"Gamma"}
]}]')
assert_contains "ask t6 ok" "$R12" '"result":"ask_ok"'
T6_ID=$(extract_id "$R12")

echo ""
echo -e "  ${YELLOW}┌──────────────────────────────────────────────────────┐${NC}"
echo -e "  ${YELLOW}│  AsQu 창에서 'Alpha' 를 클릭하세요 (60초 제한)      │${NC}"
echo -e "  ${YELLOW}│  선택지 아래 description 텍스트가 보이는지 확인하세요 │${NC}"
echo -e "  ${YELLOW}│  question id = $T6_ID                                  │${NC}"
echo -e "  ${YELLOW}└──────────────────────────────────────────────────────┘${NC}"
echo ""

R=$(run t6 wait "$T6_ID" --timeout 60)
assert_contains "live answer: answers_ok"       "$R" '"result":"answers_ok"'
assert_contains "live answer: has answered"     "$R" '"answered"'
assert_not_contains "live answer: not denied"   "$R" '"denied"'
assert_not_contains "live answer: not timedout" "$R" '"timedOut"'

# ======================================================================
# TEST 14: wait unblocks on session removal (background wait + dismiss)
# ======================================================================
echo ""
info "--- Test 14: wait unblocks when session is removed ---"

R=$(run t_sess ask '[{"text":"Session removal test","choices":["Yes","No"]}]')
assert_contains "ask t_sess ok" "$R" '"result":"ask_ok"'
T_SESS_ID=$(extract_id "$R")
info "   t_sess question id=$T_SESS_ID"

# Start blocking wait in background
T13_TMP=$(mktemp)
CLAUDE_SESSION_ID="t_sess" "$ASQU" wait "$T_SESS_ID" --timeout 30 2>/dev/null >"$T13_TMP" &
T13_PID=$!
sleep 0.3

# Dismiss with "session removed" reason — same operation as remove_session_with_questions
R=$(run t_sess dismiss "$T_SESS_ID" --reason "session removed")
assert_contains "dismiss t_sess ok" "$R" '"result":"dismiss_ok"'

# Background wait must have unblocked
wait "$T13_PID"
T13_OUT=$(cat "$T13_TMP"); rm -f "$T13_TMP"
assert_contains     "session removal: answers_ok"        "$T13_OUT" '"result":"answers_ok"'
assert_contains     "session removal: denied"             "$T13_OUT" '"denied"'
assert_contains     "session removal: reason preserved"  "$T13_OUT" '"session removed"'
assert_not_contains "session removal: no timedOut"       "$T13_OUT" '"timedOut"'
assert_not_contains "session removal: no shutdown"       "$T13_OUT" '"shutdown"'

# ================================================================================
# TEST 15: wait returns shutdown=true on graceful shutdown (asqu shutdown)
# ================================================================================
echo ""
info "--- Test 15: wait returns shutdown=true on graceful shutdown ---"

R=$(run t_quit ask '[{"text":"Shutdown test","choices":["Yes","No"]}]')
assert_contains "ask t_quit ok" "$R" '"result":"ask_ok"'
T_QUIT_ID=$(extract_id "$R")
info "   t_quit question id=$T_QUIT_ID"

# Start blocking wait in background
T14_TMP=$(mktemp)
CLAUDE_SESSION_ID="t_quit" "$ASQU" wait "$T_QUIT_ID" --timeout 60 2>/dev/null >"$T14_TMP" &
T14_PID=$!
sleep 0.3

# Trigger graceful shutdown via IPC (begin_shutdown → notify condvar → app.exit)
"$ASQU" shutdown 2>/dev/null || true

# Background wait should have unblocked with shutdown=true (200ms drain window)
wait "$T14_PID" 2>/dev/null || true
T14_OUT=$(cat "$T14_TMP"); rm -f "$T14_TMP"
assert_contains     "shutdown: answers_ok"         "$T14_OUT" '"result":"answers_ok"'
assert_contains     "shutdown: shutdown=true"      "$T14_OUT" '"shutdown":true'
assert_not_contains "shutdown: no timedOut"        "$T14_OUT" '"timedOut"'

# ============================================================
# Summary
# ============================================================
echo ""
echo "============================================================"
TOTAL=$((PASS + FAIL))
echo -e " Results: ${GREEN}$PASS passed${NC} / ${RED}$FAIL failed${NC} / $TOTAL total"
echo "============================================================"

[[ $FAIL -eq 0 ]] && exit 0 || exit 1
