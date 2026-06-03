#!/usr/bin/env python3
"""Summarize harness gnome state from yoagent-state JSONL events.

The output is intentionally compact: it keeps KPI values, patch/eval/decision
references, and blockers, while leaving code diffs and transcripts as external
audit-log artifacts.
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from typing import Any, Optional


GNOME_KEYS = [
    "cost_usd",
    "cost_per_successful_task_usd",
    "latency_ms",
    "latency_per_successful_task_ms",
    "input_tokens",
    "output_tokens",
    "cache_hit_ratio",
    "tool_call_malformed_rate",
    "json_parse_failure_rate",
    "context_miss_rate",
    "repair_loop_count",
    "state_failure_count",
    "fixture_agent_attempts",
    "fixture_agent_mutation_scope_failure_rate",
    "fixture_agent_unexpected_changed_file_count",
    "fim_compile_success_rate",
    "fim_rollback_rate",
    "fim_token_savings",
    "deepseek_streaming_protocol_checks",
    "deepseek_prefix_cache_checks",
    "deepseek_thinking_protocol_checks",
]


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    if not path.is_file():
        return events
    with path.open(encoding="utf-8", errors="replace") as handle:
        for line_no, line in enumerate(handle, 1):
            text = line.strip()
            if not text:
                continue
            try:
                value = json.loads(text)
            except json.JSONDecodeError as exc:
                events.append(
                    {
                        "event_id": f"malformed-line-{line_no}",
                        "event_type": "FailureObserved",
                        "payload": {
                            "reason": "malformed_state_jsonl",
                            "line": line_no,
                            "error": str(exc),
                        },
                    }
                )
                continue
            if isinstance(value, dict):
                events.append(value)
    return events


def payload(event: dict[str, Any]) -> dict[str, Any]:
    value = event.get("payload")
    if not isinstance(value, dict):
        return {}
    wrapped = value.get("value")
    if set(value.keys()).issubset({"_yoyo", "value"}) and isinstance(wrapped, dict):
        return wrapped
    return value


def event_id(event: dict[str, Any]) -> Any:
    return event.get("event_id") or event.get("id")


def event_type(event: dict[str, Any]) -> str:
    value = event.get("event_type")
    if isinstance(value, str):
        return value
    raw_payload = event.get("payload")
    if isinstance(raw_payload, dict):
        meta = raw_payload.get("_yoyo")
        if isinstance(meta, dict):
            value = meta.get("event_type")
            if isinstance(value, str):
                return value
    value = event.get("kind")
    return value if isinstance(value, str) else ""


def select_gnomes(metrics: Any) -> dict[str, Any]:
    if not isinstance(metrics, dict):
        return {}
    return {key: metrics[key] for key in GNOME_KEYS if key in metrics}


def state_metrics(value: dict[str, Any]) -> dict[str, Any]:
    metrics = value.get("metrics")
    if isinstance(metrics, dict):
        nested = metrics.get("state_metrics")
        if isinstance(nested, dict):
            return nested
    nested = value.get("state_metrics")
    if isinstance(nested, dict):
        return nested
    return {}


def summarize_patch(event: dict[str, Any]) -> dict[str, Any]:
    data = payload(event)
    return {
        "event_id": event_id(event),
        "patch_id": data.get("patch_id"),
        "kind": data.get("kind"),
        "status": data.get("status"),
        "risk_level": data.get("risk_level"),
        "base_git_commit": data.get("base_git_commit"),
        "base_harness_version": data.get("base_harness_version"),
        "intent": data.get("intent"),
    }


def summarize_eval(event: dict[str, Any]) -> dict[str, Any]:
    data = payload(event)
    metrics = state_metrics(data)
    return {
        "event_id": event_id(event),
        "eval_id": data.get("eval_id"),
        "patch_id": data.get("patch_id"),
        "suite": data.get("suite"),
        "status": data.get("status"),
        "score": data.get("score"),
        "passed": data.get("passed"),
        "failed": data.get("failed"),
        "artifact_path": data.get("artifact_path"),
        "gnomes": select_gnomes(metrics),
    }


def summarize_decision(event: dict[str, Any]) -> dict[str, Any]:
    data = payload(event)
    decision = data.get("promotion_decision")
    if not isinstance(decision, dict):
        return {
            "event_id": event_id(event),
            "decision_type": data.get("decision_type"),
            "decision": data.get("decision"),
            "patch_id": data.get("patch_id"),
            "reason": data.get("reason"),
        }

    evidence = decision.get("metric_evidence")
    metric_rows: list[dict[str, Any]] = []
    if isinstance(evidence, dict):
        rows = evidence.get("metrics")
        if isinstance(rows, list):
            for row in rows:
                if isinstance(row, dict):
                    metric_rows.append(
                        {
                            "metric": row.get("metric"),
                            "baseline": row.get("baseline"),
                            "candidate": row.get("candidate"),
                            "delta": row.get("delta"),
                            "direction": row.get("direction"),
                        }
                    )

    return {
        "event_id": event_id(event),
        "patch_id": data.get("patch_id") or decision.get("patch_id"),
        "decision_type": data.get("decision_type"),
        "decision": decision.get("decision") or data.get("decision"),
        "eligible": decision.get("eligible"),
        "criterion": decision.get("criterion"),
        "reason": decision.get("reason") or data.get("reason"),
        "baseline_eval_id": decision.get("baseline_eval_id"),
        "candidate_eval_id": decision.get("candidate_eval_id"),
        "metrics": metric_rows,
    }


def summarize_blocker(event: dict[str, Any]) -> Optional[dict[str, Any]]:
    data = payload(event)
    reason = data.get("reason") or data.get("failure_reason") or data.get("blocker")
    if not reason:
        decision = data.get("promotion_decision")
        if isinstance(decision, dict) and decision.get("eligible") is False:
            reason = decision.get("reason")
    if not reason:
        return None
    return {
        "event_id": event_id(event),
        "event_type": event_type(event),
        "patch_id": data.get("patch_id"),
        "reason": reason,
    }


def summarize(events: list[dict[str, Any]], source: Path) -> dict[str, Any]:
    counts: dict[str, int] = {}
    patches: list[dict[str, Any]] = []
    evals: list[dict[str, Any]] = []
    decisions: list[dict[str, Any]] = []
    blockers: list[dict[str, Any]] = []
    code_refs: list[dict[str, Any]] = []
    latest_gnomes: dict[str, Any] = {}

    for event in events:
        kind = event_type(event)
        counts[kind] = counts.get(kind, 0) + 1
        data = payload(event)

        if kind in {"PatchProposed", "PatchApplied", "PatchPromoted", "PatchRejected"}:
            patches.append(summarize_patch(event))
        if kind == "PatchEvaluated":
            eval_summary = summarize_eval(event)
            evals.append(eval_summary)
            latest_gnomes.update(eval_summary["gnomes"])
        if kind == "DecisionRecorded":
            decisions.append(summarize_decision(event))
        if kind in {"FailureObserved", "PatchRejected", "DecisionRecorded"}:
            blocker = summarize_blocker(event)
            if blocker:
                blockers.append(blocker)
        if kind in {"PatchProposed", "PatchApplied", "CommitCreated", "RevertPerformed"}:
            ref = {
                "event_id": event_id(event),
                "event_type": kind,
                "patch_id": data.get("patch_id"),
                "commit": data.get("commit") or data.get("commit_sha") or data.get("base_git_commit"),
                "artifact_path": data.get("artifact_path"),
                "pr": data.get("pr") or data.get("pull_request"),
            }
            if any(value for key, value in ref.items() if key not in {"event_id", "event_type"}):
                code_refs.append(ref)

    latest_eval = evals[-1] if evals else None
    latest_decision = decisions[-1] if decisions else None
    return {
        "schema_version": 1,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "source": str(source),
        "event_count": len(events),
        "event_counts": counts,
        "gnome_keys": GNOME_KEYS,
        "latest_gnomes": latest_gnomes,
        "patches": patches[-20:],
        "evals": evals[-20:],
        "decisions": decisions[-20:],
        "blockers": blockers[-20:],
        "code_refs": code_refs[-20:],
        "latest_eval": latest_eval,
        "latest_decision": latest_decision,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    events = load_jsonl(args.events)
    summary = summarize(events, args.events)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2, sort_keys=True)
        handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
