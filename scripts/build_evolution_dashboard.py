#!/usr/bin/env python3
"""Build the static harness evolution dashboard from audit-log summaries."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def session_sort_key(path: Path) -> str:
    return path.name


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
                "latest_gnomes": summary.get("latest_gnomes", {}),
                "latest_eval": latest_eval,
                "latest_decision": latest_decision,
                "patches": summary.get("patches", []),
                "decisions": summary.get("decisions", []),
                "blockers": summary.get("blockers", []),
                "code_refs": summary.get("code_refs", []),
            }
        )
    return sessions


def aggregate(sessions: list[dict[str, Any]]) -> dict[str, Any]:
    promoted = 0
    rejected = 0
    blockers = 0
    evals = 0
    latest_gnomes: dict[str, Any] = {}

    for session in sessions:
        evals += 1 if session.get("latest_eval") else 0
        blockers += len(session.get("blockers") or [])
        latest_gnomes.update(session.get("latest_gnomes") or {})
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
        "latest_gnomes": latest_gnomes,
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
      grid-template-columns: repeat(5, minmax(130px, 1fr));
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

    .panel h2 {
      margin: 0;
      padding: 14px 16px;
      border-bottom: 1px solid var(--line);
      font-size: 15px;
      letter-spacing: 0;
      text-transform: uppercase;
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

    .good { color: var(--green); }
    .bad { color: var(--red); }
    .info { color: var(--blue); }
    .warn { color: var(--gold); }
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

    .muted { color: var(--muted); }
    .empty {
      padding: 28px;
      color: var(--muted);
      text-align: center;
    }

    @media (max-width: 980px) {
      .grid { grid-template-columns: repeat(2, minmax(130px, 1fr)); }
      .split { grid-template-columns: 1fr; }
      .toolbar { grid-template-columns: 1fr; }
    }

    @media (max-width: 520px) {
      .grid { grid-template-columns: 1fr; }
      header { position: static; }
    }
  </style>
</head>
<body>
  <header>
    <h1>DeepSeek harness evolution</h1>
    <p class="subhead">Audit-log state summaries for gnome KPIs, patch decisions, eval evidence, blockers, and code references. Product users do not see this layer.</p>
  </header>
  <main>
    <section class="toolbar" aria-label="Dashboard filters">
      <input id="search" placeholder="Filter sessions, patches, decisions, blockers">
      <select id="status">
        <option value="all">All sessions</option>
        <option value="blocked">Has blockers</option>
        <option value="promoted">Promoted or eligible</option>
        <option value="rejected">Rejected or ineligible</option>
      </select>
      <button id="reset" type="button">Reset</button>
    </section>
    <section class="grid" id="summary"></section>
    <section class="split">
      <section class="panel">
        <h2>Session Timeline</h2>
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Session</th>
                <th>Eval</th>
                <th>Decision</th>
                <th>Gnomes</th>
                <th>Run</th>
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

    function decisionClass(decision) {
      const d = String(decision?.decision || "").toLowerCase();
      if (decision?.eligible === true || d.includes("promote")) return "good";
      if (decision?.eligible === false || d.includes("reject")) return "bad";
      return "warn";
    }

    function matches(session) {
      const haystack = JSON.stringify(session).toLowerCase();
      if (state.query && !haystack.includes(state.query.toLowerCase())) return false;
      const decisions = session.decisions || [];
      if (state.status === "blocked") return (session.blockers || []).length > 0;
      if (state.status === "promoted") return decisions.some(d => d.eligible === true || String(d.decision || "").toLowerCase().includes("promote"));
      if (state.status === "rejected") return decisions.some(d => d.eligible === false || String(d.decision || "").toLowerCase().includes("reject"));
      return true;
    }

    function renderSummary(data, filtered) {
      const agg = data.aggregate || {};
      const cards = [
        ["Sessions", filtered.length],
        ["Evaluations", agg.eval_count || 0],
        ["Promoted", agg.promoted_decisions || 0],
        ["Rejected", agg.rejected_decisions || 0],
        ["Blockers", agg.blocker_count || 0],
      ];
      document.getElementById("summary").innerHTML = cards.map(([label, value]) => `
        <article class="metric">
          <div class="label">${label}</div>
          <div class="value">${text(value)}</div>
        </article>
      `).join("");
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
        const gnomes = Object.entries(session.latest_gnomes || {}).slice(0, 4);
        const runClass = session.reverted ? "bad" : (session.build_ok && session.test_ok ? "good" : "warn");
        return `<tr>
          <td><strong>${text(session.id)}</strong><div class="muted">${text(session.ts)}</div></td>
          <td><span class="pill ${evalData.status === "Passed" ? "good" : "warn"}">${text(evalData.status)}</span><div class="muted">${text(evalData.suite)} score ${text(evalData.score)}</div></td>
          <td><span class="${decisionClass(decision)}">${text(decision.criterion || decision.decision)}</span><div class="muted">${text(decision.reason)}</div></td>
          <td>${gnomes.length ? gnomes.map(([k, v]) => metricChip(k, v)).join(" ") : `<span class="muted">No gnomes captured</span>`}</td>
          <td><span class="${runClass}">build ${text(session.build_ok)} / test ${text(session.test_ok)}</span><div class="muted">tasks ${text(session.tasks_succeeded)}/${text(session.tasks_attempted)}</div></td>
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
        (session.patches || []).slice(-2).forEach(patch => {
          items.push({ kind: "Patch", className: "warn", session: session.id, title: patch.patch_id || patch.intent, detail: `${patch.kind || "-"} risk ${patch.risk_level || "-"}` });
        });
      });
      const panel = document.getElementById("evidence");
      if (!items.length) {
        panel.innerHTML = `<div class="empty">No blockers or code references yet.</div>`;
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
      renderSummary(data, filtered);
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
