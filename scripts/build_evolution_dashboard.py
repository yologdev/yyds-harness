#!/usr/bin/env python3
"""Build the static harness evolution dashboard from audit-log summaries."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path
from typing import Any


REPO_URL = "https://github.com/yologdev/yyds-harness"


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def session_sort_key(path: Path) -> str:
    return path.name


def is_real_blocker(blocker: dict[str, Any]) -> bool:
    reason = str(blocker.get("reason") or "").lower()
    if reason.startswith("allowed "):
        return False
    if " via session_always" in reason or " via repo_always" in reason:
        return False
    return True


def load_sessions(audit_sessions: Path) -> list[dict[str, Any]]:
    sessions: list[dict[str, Any]] = []
    if not audit_sessions.is_dir():
        return sessions

    for session_dir in sorted(audit_sessions.iterdir(), key=session_sort_key):
        if not session_dir.is_dir():
            continue
        outcome = load_json(session_dir / "outcome.json")
        summary = load_json(session_dir / "state" / "summary.json")
        latest_eval = summary.get("latest_eval") if isinstance(summary.get("latest_eval"), dict) else {}
        latest_decision = (
            summary.get("latest_decision") if isinstance(summary.get("latest_decision"), dict) else {}
        )
        blockers = [
            blocker
            for blocker in (summary.get("blockers", []) if isinstance(summary.get("blockers"), list) else [])
            if isinstance(blocker, dict) and is_real_blocker(blocker)
        ]
        sessions.append(
            {
                "id": session_dir.name,
                "day": outcome.get("day"),
                "ts": outcome.get("ts") or summary.get("generated_at"),
                "session_time": outcome.get("session_time"),
                "build_ok": outcome.get("build_ok"),
                "test_ok": outcome.get("test_ok"),
                "tasks_attempted": outcome.get("tasks_attempted"),
                "tasks_succeeded": outcome.get("tasks_succeeded"),
                "reverted": outcome.get("reverted"),
                "event_count": summary.get("event_count", 0),
                "event_counts": summary.get("event_counts", {}),
                "latest_gnomes": summary.get("latest_gnomes", {}),
                "gnome_keys": summary.get("gnome_keys", []),
                "evals": summary.get("evals", []),
                "latest_eval": latest_eval,
                "latest_decision": latest_decision,
                "patches": summary.get("patches", []),
                "decisions": summary.get("decisions", []),
                "blockers": blockers,
                "code_refs": summary.get("code_refs", []),
                "audit_url": f"{REPO_URL}/tree/audit-log/sessions/{session_dir.name}",
            }
        )
    return sessions


def run_health(session: dict[str, Any]) -> str:
    attempted = session.get("tasks_attempted") or 0
    succeeded = session.get("tasks_succeeded") or 0
    if session.get("reverted"):
        return "reverted"
    if session.get("build_ok") is True and session.get("test_ok") is True and attempted == succeeded:
        return "passed"
    if succeeded:
        return "partial"
    return "attention"


def aggregate(sessions: list[dict[str, Any]]) -> dict[str, Any]:
    promoted = 0
    rejected = 0
    blockers = 0
    evals = 0
    events = 0
    tasks_attempted = 0
    tasks_succeeded = 0
    latest_gnomes: dict[str, Any] = {}
    gnome_keys: list[str] = []
    health = {"passed": 0, "partial": 0, "attention": 0, "reverted": 0}
    event_counts: dict[str, int] = {}

    for session in sessions:
        evals += 1 if session.get("latest_eval") else 0
        blockers += len(session.get("blockers") or [])
        events += int(session.get("event_count") or 0)
        tasks_attempted += int(session.get("tasks_attempted") or 0)
        tasks_succeeded += int(session.get("tasks_succeeded") or 0)
        health[run_health(session)] += 1
        latest_gnomes.update(session.get("latest_gnomes") or {})
        for key in session.get("gnome_keys") or []:
            if isinstance(key, str) and key not in gnome_keys:
                gnome_keys.append(key)
        for kind, count in (session.get("event_counts") or {}).items():
            if isinstance(count, int):
                event_counts[str(kind)] = event_counts.get(str(kind), 0) + count
        for decision in session.get("decisions") or []:
            decision_text = str(decision.get("decision") or "").lower()
            if decision.get("eligible") is True or "promote" in decision_text:
                promoted += 1
            if decision.get("eligible") is False or "reject" in decision_text:
                rejected += 1

    return {
        "session_count": len(sessions),
        "eval_count": evals,
        "promoted_decisions": promoted,
        "rejected_decisions": rejected,
        "blocker_count": blockers,
        "event_count": events,
        "tasks_attempted": tasks_attempted,
        "tasks_succeeded": tasks_succeeded,
        "task_success_rate": (tasks_succeeded / tasks_attempted) if tasks_attempted else None,
        "health": health,
        "event_counts": event_counts,
        "latest_gnomes": latest_gnomes,
        "gnome_keys": gnome_keys,
        "latest_ts": sessions[-1].get("ts") if sessions else None,
    }


HTML = r"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Yoyo DeepSeek Harness Evolution</title>
  <style>
    :root {
      color-scheme: light;
      --paper: #f2f5f1;
      --ink: #15140f;
      --muted: #59625d;
      --line: #cbd5cf;
      --panel: #fffdfa;
      --panel-strong: #e6eee8;
      --green: #1b7a58;
      --red: #b23a32;
      --blue: #285c92;
      --gold: #9b7018;
      --violet: #6d4aa2;
      --shadow: 0 18px 44px rgba(22, 36, 29, 0.12);
    }

    * { box-sizing: border-box; }
    body {
      margin: 0;
      background:
        linear-gradient(90deg, rgba(21, 20, 15, 0.045) 1px, transparent 1px),
        linear-gradient(rgba(21, 20, 15, 0.035) 1px, transparent 1px),
        var(--paper);
      background-size: 28px 28px;
      color: var(--ink);
      font: 15px/1.45 ui-monospace, "SFMono-Regular", "Cascadia Mono", "Liberation Mono", monospace;
    }

    header {
      padding: 28px clamp(18px, 4vw, 48px) 18px;
      border-bottom: 1px solid var(--line);
      background: rgba(242, 245, 241, 0.92);
      position: sticky;
      top: 0;
      z-index: 5;
      backdrop-filter: blur(10px);
    }

    h1 {
      margin: 0;
      font-size: clamp(28px, 5vw, 56px);
      line-height: 0.95;
      letter-spacing: 0;
      font-weight: 900;
      max-width: 980px;
    }

    .subhead {
      margin: 12px 0 0;
      max-width: 940px;
      color: var(--muted);
    }

    .note {
      margin-top: 12px;
      display: inline-flex;
      flex-wrap: wrap;
      gap: 8px;
      color: var(--muted);
      font-size: 13px;
    }

    main {
      padding: 24px clamp(18px, 4vw, 48px) 48px;
      display: grid;
      gap: 18px;
    }

    .toolbar {
      display: grid;
      grid-template-columns: minmax(180px, 1fr) auto auto;
      gap: 10px;
      align-items: center;
    }

    input, select, button {
      border: 1px solid var(--line);
      background: var(--panel);
      color: var(--ink);
      min-height: 42px;
      padding: 0 12px;
      border-radius: 6px;
      font: inherit;
    }

    button {
      cursor: pointer;
      font-weight: 800;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(6, minmax(130px, 1fr));
      gap: 12px;
    }

    .metric, .panel {
      border: 1px solid var(--line);
      background: rgba(255, 253, 250, 0.94);
      border-radius: 8px;
      box-shadow: var(--shadow);
    }

    .metric {
      min-height: 118px;
      padding: 14px;
      display: grid;
      align-content: space-between;
    }

    .metric small {
      color: var(--muted);
      display: block;
      margin-top: 8px;
      overflow-wrap: anywhere;
    }

    .label {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
      font-weight: 800;
    }

    .value {
      font-size: clamp(24px, 4vw, 42px);
      font-weight: 900;
      line-height: 1;
      overflow-wrap: anywhere;
    }

    .split {
      display: grid;
      grid-template-columns: minmax(320px, 1.2fr) minmax(280px, 0.8fr);
      gap: 18px;
      align-items: start;
    }

    .chart-grid {
      display: grid;
      grid-template-columns: minmax(320px, 1fr) minmax(320px, 1fr);
      gap: 18px;
      align-items: start;
    }

    .panel h2 {
      margin: 0;
      padding: 14px 16px;
      border-bottom: 1px solid var(--line);
      font-size: 15px;
      letter-spacing: 0;
      text-transform: uppercase;
    }

    .panel-body {
      padding: 14px 16px 16px;
      display: grid;
      gap: 14px;
    }

    .explain {
      color: var(--muted);
      margin: 0;
      max-width: 900px;
    }

    .bar-row {
      display: grid;
      gap: 7px;
    }

    .bar-meta {
      display: flex;
      justify-content: space-between;
      gap: 16px;
      color: var(--muted);
      font-size: 13px;
    }

    .bar-track {
      height: 14px;
      border: 1px solid var(--line);
      border-radius: 999px;
      overflow: hidden;
      background: #edf1ec;
      display: flex;
    }

    .bar-fill {
      min-width: 2px;
      height: 100%;
      background: var(--blue);
    }

    .bar-fill.good { background: var(--green); }
    .bar-fill.warn { background: var(--gold); }
    .bar-fill.bad { background: var(--red); }
    .bar-fill.info { background: var(--blue); }
    .bar-fill.violet { background: var(--violet); }

    .legend {
      display: flex;
      flex-wrap: wrap;
      gap: 8px 14px;
      color: var(--muted);
      font-size: 13px;
    }

    .legend span::before {
      content: "";
      display: inline-block;
      width: 10px;
      height: 10px;
      margin-right: 6px;
      border-radius: 2px;
      background: var(--blue);
    }

    .legend .passed::before { background: var(--green); }
    .legend .partial::before { background: var(--gold); }
    .legend .attention::before { background: var(--red); }
    .legend .reverted::before { background: var(--violet); }

    .detail-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }

    .table-wrap { overflow-x: auto; }
    table {
      width: 100%;
      border-collapse: collapse;
      min-width: 760px;
    }

    th, td {
      padding: 11px 12px;
      border-bottom: 1px solid var(--line);
      text-align: left;
      vertical-align: top;
    }

    th {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
    }

    tr:hover td { background: rgba(230, 238, 232, 0.58); }
    .pill {
      display: inline-flex;
      align-items: center;
      min-height: 24px;
      padding: 0 8px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: var(--panel-strong);
      font-size: 12px;
      font-weight: 900;
      white-space: nowrap;
    }

    .pill.soft {
      background: transparent;
      color: var(--muted);
    }

    .good { color: var(--green); }
    .bad { color: var(--red); }
    .info { color: var(--blue); }
    .warn { color: var(--gold); }
    .violet { color: var(--violet); }
    .stack {
      display: grid;
      gap: 10px;
      padding: 14px;
    }

    .item {
      border: 1px solid var(--line);
      border-radius: 6px;
      background: #fffdf7;
      padding: 11px;
    }

    .item strong {
      display: block;
      margin-bottom: 4px;
      overflow-wrap: anywhere;
    }

    .item p {
      margin: 6px 0 0;
    }

    a {
      color: var(--blue);
      text-decoration-thickness: 1px;
      text-underline-offset: 3px;
    }

    .muted { color: var(--muted); }
    .empty {
      padding: 28px;
      color: var(--muted);
      text-align: center;
    }

    @media (max-width: 980px) {
      .grid { grid-template-columns: repeat(2, minmax(130px, 1fr)); }
      .chart-grid { grid-template-columns: 1fr; }
      .split { grid-template-columns: 1fr; }
      .toolbar { grid-template-columns: 1fr; }
    }

    @media (max-width: 520px) {
      .grid { grid-template-columns: 1fr; }
      .detail-grid { grid-template-columns: 1fr; }
      header { position: static; }
    }
  </style>
</head>
<body>
  <header>
    <h1>DeepSeek harness evolution</h1>
    <p class="subhead">A human-readable view of yyds's self-improvement loop: what ran, whether it shipped, which state signals were captured, and where the audit evidence lives.</p>
    <div class="note">
      <span>Source: audit-log branch</span>
      <span>Only sessions with pushed audit evidence appear here.</span>
    </div>
  </header>
  <main>
    <section class="toolbar" aria-label="Dashboard filters">
      <input id="search" placeholder="Filter sessions, decisions, event types, evidence">
      <select id="status">
        <option value="all">All sessions</option>
        <option value="passed">Passed runs</option>
        <option value="attention">Needs attention</option>
        <option value="blocked">Has blockers</option>
        <option value="promoted">Promoted or eligible</option>
        <option value="rejected">Rejected or ineligible</option>
      </select>
      <button id="reset" type="button">Reset</button>
    </section>
    <section class="grid" id="summary"></section>
    <section class="chart-grid">
      <section class="panel">
        <h2>Run Health</h2>
        <div class="panel-body">
          <p class="explain">This chart answers the first human question: did the autonomous session complete useful work and keep the harness green?</p>
          <div id="healthChart"></div>
          <div class="legend">
            <span class="passed">passed</span>
            <span class="partial">partial</span>
            <span class="attention">needs attention</span>
            <span class="reverted">reverted</span>
          </div>
        </div>
      </section>
      <section class="panel">
        <h2>State Signals</h2>
        <div class="panel-body">
          <p class="explain">Top recorded event types from yoagent-state. These show what the harness actually observed, not what the journal claims.</p>
          <div id="eventChart"></div>
        </div>
      </section>
    </section>
    <section class="chart-grid">
      <section class="panel">
        <h2>Task Throughput</h2>
        <div class="panel-body">
          <p class="explain">Successful tasks divided by attempted tasks across the visible audit window.</p>
          <div id="taskChart"></div>
        </div>
      </section>
      <section class="panel">
        <h2>Latest Gnomes</h2>
        <div class="panel-body">
          <p class="explain">Gnome metrics are compact health signals from state summaries: cost, latency, cache, failures, workflow quality, and feedback-loop quality when available.</p>
          <div id="gnomes"></div>
        </div>
      </section>
    </section>
    <section class="split">
      <section class="panel">
        <h2>Session Timeline</h2>
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Session</th>
                <th>Outcome</th>
                <th>Decision</th>
                <th>State</th>
                <th>Evidence</th>
              </tr>
            </thead>
            <tbody id="sessions"></tbody>
          </table>
        </div>
      </section>
      <section class="panel">
        <h2>Evidence Queue</h2>
        <div class="stack" id="evidence"></div>
      </section>
    </section>
  </main>
  <script>
    const fmt = new Intl.NumberFormat(undefined, { maximumFractionDigits: 3 });
    const state = { data: null, query: "", status: "all" };
    const gnomeLabels = {
      cost_usd: "Estimated cost",
      cost_per_successful_task_usd: "Cost per successful task",
      latency_ms: "Latency",
      cache_hit_ratio: "Cache hit ratio",
      tool_call_malformed_rate: "Malformed tool calls",
      json_parse_failure_rate: "JSON parse failures",
      context_miss_rate: "Context misses",
      repair_loop_count: "Repair loops",
      state_failure_count: "State failures",
      coding_log_score: "Coding log score",
      coding_log_confidence: "Coding log confidence",
      workflow_success_rate: "Workflow success",
      session_success_rate: "Session success",
      task_success_rate: "Task success",
      recurring_failure_count: "Recurring failures",
      state_capture_coverage: "State capture",
      audit_capture_coverage: "Audit capture",
      closed_loop_fix_rate: "Closed-loop fix rate"
    };

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, char => ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;"
      }[char]));
    }

    function text(value) {
      if (value === null || value === undefined || value === "") return "-";
      if (typeof value === "number") return escapeHtml(fmt.format(value));
      return escapeHtml(value);
    }

    function metricChip(name, value) {
      return `<span class="pill">${text(name)}: ${text(value)}</span>`;
    }

    function percent(value) {
      if (value === null || value === undefined || Number.isNaN(Number(value))) return "-";
      return `${fmt.format(Number(value) * 100)}%`;
    }

    function healthOf(session) {
      const attempted = Number(session.tasks_attempted || 0);
      const succeeded = Number(session.tasks_succeeded || 0);
      if (session.reverted) return "reverted";
      if (session.build_ok === true && session.test_ok === true && attempted === succeeded) return "passed";
      if (succeeded > 0) return "partial";
      return "attention";
    }

    function healthClass(health) {
      if (health === "passed") return "good";
      if (health === "partial") return "warn";
      if (health === "reverted") return "violet";
      return "bad";
    }

    function decisionClass(decision) {
      const d = String(decision?.decision || "").toLowerCase();
      if (decision?.eligible === true || d.includes("promote")) return "good";
      if (decision?.eligible === false || d.includes("reject")) return "bad";
      return "warn";
    }

    function aggregateSessions(sessions, fallback = {}) {
      const health = { passed: 0, partial: 0, attention: 0, reverted: 0 };
      const eventCounts = {};
      const latestGnomes = {};
      const gnomeKeys = [];
      let eventCount = 0;
      let tasksAttempted = 0;
      let tasksSucceeded = 0;
      let evalCount = 0;
      let blockers = 0;
      let promoted = 0;
      let rejected = 0;

      sessions.forEach(session => {
        const healthKey = healthOf(session);
        health[healthKey] = (health[healthKey] || 0) + 1;
        eventCount += Number(session.event_count || 0);
        tasksAttempted += Number(session.tasks_attempted || 0);
        tasksSucceeded += Number(session.tasks_succeeded || 0);
        blockers += (session.blockers || []).length;
        if (session.latest_eval && Object.keys(session.latest_eval).length) evalCount += 1;
        Object.entries(session.event_counts || {}).forEach(([kind, count]) => {
          eventCounts[kind] = (eventCounts[kind] || 0) + Number(count || 0);
        });
        Object.assign(latestGnomes, session.latest_gnomes || {});
        (session.gnome_keys || []).forEach(key => {
          if (!gnomeKeys.includes(key)) gnomeKeys.push(key);
        });
        (session.decisions || []).forEach(decision => {
          const text = String(decision.decision || "").toLowerCase();
          if (decision.eligible === true || text.includes("promote")) promoted += 1;
          if (decision.eligible === false || text.includes("reject")) rejected += 1;
        });
      });

      return {
        ...fallback,
        session_count: sessions.length,
        event_count: eventCount,
        event_counts: eventCounts,
        tasks_attempted: tasksAttempted,
        tasks_succeeded: tasksSucceeded,
        task_success_rate: tasksAttempted ? tasksSucceeded / tasksAttempted : null,
        eval_count: evalCount,
        blocker_count: blockers,
        promoted_decisions: promoted,
        rejected_decisions: rejected,
        health,
        latest_gnomes: latestGnomes,
        gnome_keys: gnomeKeys
      };
    }

    function matches(session) {
      const haystack = JSON.stringify(session).toLowerCase();
      if (state.query && !haystack.includes(state.query.toLowerCase())) return false;
      const decisions = session.decisions || [];
      const health = healthOf(session);
      if (state.status === "passed") return health === "passed";
      if (state.status === "attention") return health !== "passed";
      if (state.status === "blocked") return (session.blockers || []).length > 0;
      if (state.status === "promoted") return decisions.some(d => d.eligible === true || String(d.decision || "").toLowerCase().includes("promote"));
      if (state.status === "rejected") return decisions.some(d => d.eligible === false || String(d.decision || "").toLowerCase().includes("reject"));
      return true;
    }

    function barRow(label, value, max, className = "info", detail = "") {
      const safeMax = Math.max(Number(max) || 0, 1);
      const width = Math.max(0, Math.min(100, (Number(value) || 0) / safeMax * 100));
      return `<div class="bar-row">
        <div class="bar-meta"><strong>${text(label)}</strong><span>${text(value)}${detail ? ` ${text(detail)}` : ""}</span></div>
        <div class="bar-track"><div class="bar-fill ${className}" style="width:${width}%"></div></div>
      </div>`;
    }

    function stackedHealth(health) {
      const total = Object.values(health || {}).reduce((sum, value) => sum + Number(value || 0), 0) || 1;
      return `<div class="bar-track" title="Run health">
        ${["passed", "partial", "attention", "reverted"].map(key => {
          const width = Math.max(0, Number(health?.[key] || 0) / total * 100);
          return `<div class="bar-fill ${healthClass(key)}" style="width:${width}%"></div>`;
        }).join("")}
      </div>
      <div class="detail-grid">
        ${["passed", "partial", "attention", "reverted"].map(key => `
          <div class="item"><span class="pill ${healthClass(key)}">${key}</span><strong>${text(health?.[key] || 0)}</strong></div>
        `).join("")}
      </div>`;
    }

    function renderSummary(agg) {
      const rate = agg.task_success_rate;
      const cards = [
        ["Sessions", agg.session_count || 0, "audit-backed runs"],
        ["Task success", rate === null || rate === undefined ? "-" : percent(rate), `${text(agg.tasks_succeeded || 0)}/${text(agg.tasks_attempted || 0)} tasks`],
        ["Green runs", agg.health?.passed || 0, "build + tests passed"],
        ["Events", agg.event_count || 0, "state records"],
        ["Evaluations", agg.eval_count || 0, "patch eval records"],
        ["Blockers", agg.blocker_count || 0, "real blocking signals"],
      ];
      document.getElementById("summary").innerHTML = cards.map(([label, value, hint]) => `
        <article class="metric">
          <div class="label">${label}</div>
          <div class="value">${text(value)}</div>
          <small>${text(hint)}</small>
        </article>
      `).join("");
    }

    function renderCharts(agg) {
      document.getElementById("healthChart").innerHTML = stackedHealth(agg.health || {});

      const attempted = Number(agg.tasks_attempted || 0);
      const succeeded = Number(agg.tasks_succeeded || 0);
      document.getElementById("taskChart").innerHTML = attempted
        ? barRow("Successful tasks", succeeded, attempted, succeeded === attempted ? "good" : "warn", `of ${attempted}`)
        : `<div class="empty">No task outcome data yet.</div>`;

      const eventRows = Object.entries(agg.event_counts || {})
        .sort((a, b) => Number(b[1]) - Number(a[1]))
        .slice(0, 8);
      const eventMax = Math.max(...eventRows.map(([, value]) => Number(value || 0)), 1);
      document.getElementById("eventChart").innerHTML = eventRows.length
        ? eventRows.map(([kind, count]) => barRow(kind, count, eventMax, "info")).join("")
        : `<div class="empty">No state events captured yet.</div>`;

      const gnomeRows = Object.entries(agg.latest_gnomes || {}).slice(0, 12);
      document.getElementById("gnomes").innerHTML = gnomeRows.length
        ? `<div class="detail-grid">${gnomeRows.map(([key, value]) => `
          <article class="item">
            <span class="pill soft">${text(key)}</span>
            <strong>${text(gnomeLabels[key] || key)}</strong>
            <p>${text(value)}</p>
          </article>
        `).join("")}</div>`
        : (agg.gnome_keys || []).length
          ? `<div class="stack">${(agg.gnome_keys || []).slice(0, 16).map(key => `<span class="pill soft">${text(gnomeLabels[key] || key)}</span>`).join("")}</div><p class="explain">These signals are configured, but this audit window has not emitted numeric KPI values yet.</p>`
          : `<div class="empty">No gnome KPI values captured yet. This is expected until eval or log-feedback events emit metrics.</div>`;
    }

    function renderSessions(sessions) {
      const body = document.getElementById("sessions");
      if (!sessions.length) {
        body.innerHTML = `<tr><td colspan="5" class="empty">No sessions match the current filter.</td></tr>`;
        return;
      }
      body.innerHTML = sessions.slice().reverse().map(session => {
        const evalData = session.latest_eval || {};
        const decision = session.latest_decision || {};
        const health = healthOf(session);
        const events = session.event_count || 0;
        return `<tr>
          <td><strong>${text(session.id)}</strong><div class="muted">Day ${text(session.day)} at ${text(session.session_time)}<br>${text(session.ts)}</div></td>
          <td><span class="pill ${healthClass(health)}">${text(health)}</span><div class="muted">build ${text(session.build_ok)} / test ${text(session.test_ok)}<br>tasks ${text(session.tasks_succeeded)}/${text(session.tasks_attempted)}</div></td>
          <td><span class="${decisionClass(decision)}">${text(decision.criterion || decision.decision || decision.decision_type)}</span><div class="muted">${text(decision.reason)}</div></td>
          <td><span class="pill soft">${text(events)} events</span><div class="muted">eval ${text(evalData.status)} ${evalData.score === undefined ? "" : `score ${text(evalData.score)}`}</div></td>
          <td><a href="${text(session.audit_url)}">audit files</a><div class="muted">${text((session.blockers || []).length)} blockers / ${text((session.evals || []).length)} evals / ${text((session.patches || []).length)} patches / ${text((session.code_refs || []).length)} refs</div></td>
        </tr>`;
      }).join("");
    }

    function renderEvidence(sessions) {
      const items = [];
      sessions.slice().reverse().forEach(session => {
        (session.blockers || []).forEach(blocker => {
          items.push({ kind: "Blocker", className: "bad", session: session.id, title: blocker.reason, detail: blocker.patch_id || blocker.event_id });
        });
        (session.code_refs || []).forEach(ref => {
          items.push({ kind: "Code ref", className: "info", session: session.id, title: ref.commit || ref.patch_id || ref.artifact_path, detail: ref.event_type });
        });
        (session.evals || []).slice(-2).forEach(evalData => {
          items.push({ kind: "Eval", className: evalData.status === "passed" ? "good" : "warn", session: session.id, title: evalData.eval_id || evalData.suite || "evaluation", detail: `${evalData.suite || "-"} ${evalData.status || "-"} score ${evalData.score === undefined ? "-" : evalData.score}` });
        });
        (session.patches || []).slice(-2).forEach(patch => {
          items.push({ kind: "Patch", className: "warn", session: session.id, title: patch.patch_id || patch.intent, detail: `${patch.kind || "-"} risk ${patch.risk_level || "-"}` });
        });
      });
      const panel = document.getElementById("evidence");
      if (!items.length) {
        panel.innerHTML = `<div class="empty">No blockers, evals, patches, or code references yet.</div>`;
        return;
      }
      panel.innerHTML = items.slice(0, 24).map(item => `
        <article class="item">
          <span class="pill ${item.className}">${item.kind}</span>
          <strong>${text(item.title)}</strong>
          <div class="muted">${text(item.session)} / ${text(item.detail)}</div>
        </article>
      `).join("");
    }

    function render() {
      const data = state.data || { sessions: [], aggregate: {} };
      const filtered = (data.sessions || []).filter(matches);
      const visibleAgg = aggregateSessions(filtered, data.aggregate || {});
      renderSummary(visibleAgg);
      renderCharts(visibleAgg);
      renderSessions(filtered);
      renderEvidence(filtered);
    }

    fetch("data.json")
      .then(response => response.ok ? response.json() : Promise.reject(new Error("missing data.json")))
      .then(data => { state.data = data; render(); })
      .catch(error => {
        state.data = { sessions: [], aggregate: {}, error: String(error) };
        render();
      });

    document.getElementById("search").addEventListener("input", event => {
      state.query = event.target.value;
      render();
    });
    document.getElementById("status").addEventListener("change", event => {
      state.status = event.target.value;
      render();
    });
    document.getElementById("reset").addEventListener("click", () => {
      state.query = "";
      state.status = "all";
      document.getElementById("search").value = "";
      document.getElementById("status").value = "all";
      render();
    });
  </script>
</body>
</html>
"""


def build(audit_sessions: Path, output_dir: Path) -> dict[str, Any]:
    sessions = load_sessions(audit_sessions)
    data = {
        "schema_version": 1,
        "source": str(audit_sessions),
        "aggregate": aggregate(sessions),
        "sessions": sessions,
    }
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "data.json").write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (output_dir / "index.html").write_text(HTML, encoding="utf-8")
    return data


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--audit-sessions", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--copy-to", type=Path, help="Optional second output directory.")
    args = parser.parse_args()

    data = build(args.audit_sessions, args.output_dir)
    if args.copy_to:
        if args.copy_to.exists():
            shutil.rmtree(args.copy_to)
        shutil.copytree(args.output_dir, args.copy_to)
    print(
        f"Evolution dashboard built: {args.output_dir / 'index.html'} "
        f"({len(data['sessions'])} sessions)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
