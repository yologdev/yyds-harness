#!/usr/bin/env python3
"""Deterministic synthesis of learnings.jsonl → active_learnings.md.

Reads the full learning archive and regenerates the active context file using
time-weighted compression tiers: recent (full detail, capped), medium (condensed),
old (themed summaries).

Output target: ≤ 250 lines, preserving the most actionable and unique insights.

Usage:
  python3 scripts/synthesize_learnings.py            # regenerate active_learnings.md
  python3 scripts/synthesize_learnings.py --check    # exit 0 if fresh, 1 if stale
  python3 scripts/synthesize_learnings.py --help     # show this help
"""

import json
import os
import re
import sys
from collections import defaultdict
from datetime import datetime

# Hard caps to keep output within ~200-250 lines
MAX_RECENT_ENTRIES = 15  # full-detail entries in recent section
MAX_MEDIUM_ENTRIES = 15  # condensed entries in medium section
MAX_OLD_THEMES = 7  # theme groups in old section
MAX_TAKEAWAY_CHARS = 400  # truncate long takeaways in recent section


def load_learnings(path):
    """Read learnings.jsonl, return list of entries sorted by day desc."""
    if not os.path.exists(path):
        return []
    entries = []
    with open(path, "r") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
                if entry.get("type") == "lesson":
                    entries.append(entry)
            except json.JSONDecodeError:
                continue
    entries.sort(key=lambda e: (-e.get("day", 0), e.get("ts", "")))
    return entries


def parse_date(ts):
    """Parse an ISO timestamp like '2026-06-26T03:50:00Z' to '2026-06-26'."""
    if not ts:
        return "unknown"
    try:
        ts_clean = str(ts).replace("Z", "+00:00")
        dt = datetime.fromisoformat(ts_clean)
        return dt.strftime("%Y-%m-%d")
    except (ValueError, TypeError):
        if "T" in str(ts):
            return str(ts).split("T")[0]
        return str(ts)[:10]


def first_sentences(text, n=2):
    """Extract the first n sentences from text."""
    if not text:
        return ""
    sentences = re.split(r"(?<=[.!?])\s+", text.strip())
    result = " ".join(sentences[:n])
    if len(result) < len(text.strip()) * 0.25 and len(sentences) <= 1:
        # Few sentence boundaries; take first ~150 chars at word boundary
        if len(text) <= 150:
            return text.strip()
        truncated = text[:150]
        last_space = truncated.rfind(" ")
        if last_space > 80:
            return truncated[:last_space]
        return truncated
    return result


def truncate_takeaway(text, max_chars=MAX_TAKEAWAY_CHARS):
    """Truncate a takeaway to max_chars at a sentence or word boundary."""
    if len(text) <= max_chars:
        return text
    truncated = text[:max_chars]
    # Try to break at sentence end
    last_period = max(truncated.rfind(". "), truncated.rfind("! "), truncated.rfind("? "))
    if last_period > max_chars * 0.6:
        return truncated[: last_period + 1]
    # Try word boundary
    last_space = truncated.rfind(" ")
    if last_space > max_chars * 0.6:
        return truncated[:last_space] + "..."
    return truncated + "..."


def score_entry(entry):
    """Score an entry for selection priority (higher = more important to include)."""
    score = 0.0
    # Prefer entries with pattern_keys (they indicate structured thinking)
    if entry.get("pattern_key"):
        score += 2.0
    # Prefer entries with substantive takeaways
    takeaway_len = len(entry.get("takeaway", ""))
    if takeaway_len > 500:
        score += 1.5
    elif takeaway_len > 200:
        score += 1.0
    # Prefer entries with context
    if len(entry.get("context", "")) > 100:
        score += 0.5
    return score


def classify_tiers(entries, latest_day):
    """Split entries into recent, medium, old based on latest_day."""
    recent = []
    medium = []
    old = []
    for e in entries:
        day = e.get("day", 0)
        if day >= latest_day - 14:
            recent.append(e)
        elif day >= latest_day - 56:
            medium.append(e)
        else:
            old.append(e)
    return recent, medium, old


def render_recent(entries):
    """Full markdown for recent entries (capped)."""
    lines = []
    lines.append("## Recent Insights (Last 2 Weeks)")
    lines.append("")
    for e in entries:
        title = e.get("title", "Untitled")
        day = e.get("day", "?")
        date = parse_date(e.get("ts", ""))
        source = e.get("source", "unknown")
        context = e.get("context", "")
        takeaway = truncate_takeaway(e.get("takeaway", ""))

        lines.append(f"### Lesson: {title}")
        lines.append(f"**Day:** {day} | **Date:** {date} | **Source:** {source}")
        lines.append("")
        if context:
            # Truncate context too if very long
            ctx = context if len(context) <= 500 else context[:497] + "..."
            lines.append(f"**Context:** {ctx}")
            lines.append("")
        if takeaway:
            lines.append(takeaway)
        lines.append("")
    return lines


def render_medium(entries):
    """Condensed markdown for medium entries (capped)."""
    lines = []
    lines.append("## Medium History (2-8 Weeks Old)")
    lines.append("")
    for e in entries:
        title = e.get("title", "Untitled")
        day = e.get("day", "?")
        takeaway = e.get("takeaway", "")
        condensed = first_sentences(takeaway, 2)
        # Remove trailing period to avoid double punctuation
        condensed = re.sub(r"\.$", "", condensed.strip())
        lines.append(f"**{title}** (Day {day}): {condensed}.")
        lines.append("")
    return lines


# Map pattern_key prefixes to human-readable theme names
THEME_PREFIX_MAP = {
    "planning": "Planning & Throughput",
    "assessment": "Assessment & Self-Evaluation",
    "diagnosis": "Diagnosis & Debugging Patterns",
    "diagnostics": "Diagnosis & Debugging Patterns",
    "growth": "Growth & Extensibility",
    "design": "Design & Architecture Decisions",
    "skill-evolve": "Skill Evolution & Meta-Learning",
    "dedup": "Quality & Maintenance",
    "ux": "Context & Perception Blindness",
    "maintenance": "Quality & Maintenance",
    "tests": "Quality & Maintenance",
    "recovery": "Recovery & Resilience",
    "detection": "Diagnosis & Debugging Patterns",
    "sweep": "Quality & Maintenance",
    "integration": "Design & Architecture Decisions",
    "learning": "Skill Evolution & Meta-Learning",
    "features": "Design & Architecture Decisions",
    "work": "Avoidance & Execution Patterns",
    "audit": "Assessment & Self-Evaluation",
    "bugs": "Diagnosis & Debugging Patterns",
    "attention": "Context & Perception Blindness",
    "avoidance": "Avoidance & Execution Patterns",
    "bias": "Context & Perception Blindness",
    "blindness": "Context & Perception Blindness",
    "capability": "Growth & Extensibility",
    "compete": "Growth & Extensibility",
    "completeness": "Growth & Extensibility",
    "config": "Design & Architecture Decisions",
    "context": "Context & Perception Blindness",
    "correction": "Diagnosis & Debugging Patterns",
    "decisions": "Planning & Throughput",
    "defaults": "Design & Architecture Decisions",
    "architecture": "Design & Architecture Decisions",
    "surface": "Context & Perception Blindness",
    "signal": "Diagnosis & Debugging Patterns",
    "tasks": "Planning & Throughput",
    "state": "Assessment & Self-Evaluation",
    "tool": "Design & Architecture Decisions",
    "tools": "Design & Architecture Decisions",
}


def theme_key(entry):
    """Derive theme from pattern_key prefix, or title keywords."""
    pk = entry.get("pattern_key", "")
    if pk:
        prefix = pk.split(".")[0] if "." in pk else pk
        if prefix in THEME_PREFIX_MAP:
            return THEME_PREFIX_MAP[prefix]

    # Fallback: keyword matching from title
    title = entry.get("title", "").lower()
    if any(
        w in title
        for w in [
            "avoid",
            "procrastin",
            "stall",
            "guilt",
            "ritual",
            "stuck",
            "orbit",
            "sidestep",
        ]
    ):
        return "Avoidance & Execution Patterns"
    if any(
        w in title
        for w in [
            "build",
            "clean",
            "cycle",
            "phase",
            "rhythm",
            "arc",
            "momentum",
            "sprint",
            "transform",
        ]
    ):
        return "Build-Clean-Build Rhythms"
    if any(
        w in title
        for w in [
            "blind",
            "perception",
            "see",
            "visible",
            "invisible",
            "notice",
            "attention",
            "surface",
        ]
    ):
        return "Context & Perception Blindness"
    if any(
        w in title
        for w in [
            "finish",
            "release",
            "ship",
            "publish",
            "milestone",
            "completion",
            "last mile",
            "final",
        ]
    ):
        return "Finishing & Release Dynamics"
    if any(
        w in title
        for w in [
            "test",
            "quality",
            "bug",
            "maintenance",
            "refactor",
            "split",
            "dedup",
            "duplicat",
        ]
    ):
        return "Quality & Maintenance"
    if any(
        w in title
        for w in ["plan", "priority", "backlog", "session", "throughput", "capacity", "task"]
    ):
        return "Planning & Throughput"
    if any(
        w in title
        for w in ["grow", "extend", "progress", "measure", "competitive", "compete"]
    ):
        return "Growth & Extensibility"
    if any(
        w in title
        for w in [
            "learn",
            "reflect",
            "insight",
            "wisdom",
            "self-",
            "pattern",
            "meta",
        ]
    ):
        return "Skill Evolution & Meta-Learning"
    if any(
        w in title
        for w in [
            "design",
            "architect",
            "boundary",
            "interface",
            "abstraction",
            "layer",
        ]
    ):
        return "Design & Architecture Decisions"
    if any(
        w in title
        for w in ["user", "community", "issue", "feedback", "social", "other", "stranger"]
    ):
        return "Community & External Feedback"

    return "Early Learning Patterns"


def render_old(entries):
    """Grouped theme summaries for old entries."""
    groups = defaultdict(list)
    for e in entries:
        theme = theme_key(e)
        groups[theme].append(e)

    # Sort themes by entry count (largest first), but keep catch-all last
    def sort_key(item):
        theme, ents = item
        if theme == "Early Learning Patterns":
            return (1, -len(ents))
        return (0, -len(ents))

    sorted_groups = sorted(groups.items(), key=sort_key)

    # Limit to MAX_OLD_THEMES themes; merge remainder into catch-all
    if len(sorted_groups) > MAX_OLD_THEMES:
        main_groups = sorted_groups[:MAX_OLD_THEMES]
        catch_all = []
        for _, ents in sorted_groups[MAX_OLD_THEMES:]:
            catch_all.extend(ents)
        if catch_all:
            main_groups.append(("Additional Patterns", catch_all))
        sorted_groups = main_groups

    lines = []
    lines.append("## Wisdom Themes (8+ Weeks Old)")
    lines.append("")

    for theme, ents in sorted_groups:
        lines.append(f"### **{theme}**")
        # Collect distinct key insights from the top entries in this theme
        best = sorted(
            ents, key=lambda e: len(e.get("takeaway", "")), reverse=True
        )[:5]

        insights = []
        for e in best:
            core = first_sentences(e.get("takeaway", ""), 1)
            if core and len(core) > 20 and core not in insights:
                insights.append(core)

        # Write up to 3 summary sentences
        for s in insights[:3]:
            lines.append(s)
        lines.append("")

    return lines


def count_learnings(path):
    """Count entries in learnings.jsonl."""
    if not os.path.exists(path):
        return 0
    count = 0
    with open(path, "r") as f:
        for line in f:
            if line.strip():
                count += 1
    return count


def synthesize(learnings_path, output_path):
    """Main synthesis pipeline."""
    entries = load_learnings(learnings_path)

    if not entries:
        print("No entries found in learnings.jsonl — writing fallback note.")
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        with open(output_path, "w") as f:
            f.write("# Active Learnings\n\n")
            f.write(
                "Self-reflection — what I've learned about how I work, "
                "what I value, and how I'm growing.\n\n"
            )
            f.write(
                "*(No learnings archive entries available yet. "
                "Synthesis will run when learnings.jsonl has content.)*\n"
            )
        return 0

    latest_day = max(e.get("day", 0) for e in entries)
    recent, medium, old = classify_tiers(entries, latest_day)

    # Select best entries for each tier (capped)
    # Recent: take the most recent MAX_RECENT_ENTRIES
    recent = recent[:MAX_RECENT_ENTRIES]

    # Medium: take highest-scored MAX_MEDIUM_ENTRIES
    medium = sorted(medium, key=score_entry, reverse=True)[:MAX_MEDIUM_ENTRIES]
    # Re-sort by day desc for display
    medium.sort(key=lambda e: -e.get("day", 0))

    # Old: keep all (they're grouped and summarized)

    lines = []
    lines.append("# Active Learnings")
    lines.append("")
    lines.append(
        "Self-reflection — what I've learned about how I work, "
        "what I value, and how I'm growing."
    )
    lines.append("")

    lines.extend(render_recent(recent))
    lines.extend(render_medium(medium))
    lines.extend(render_old(old))

    # Trim trailing whitespace/blank lines
    while lines and lines[-1] == "":
        lines.pop()
    lines.append("")  # single trailing newline

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w") as f:
        f.write("\n".join(lines))

    return len(lines)


def is_fresh(learnings_path, output_path):
    """Check if output exists and is newer than learnings.jsonl."""
    if not os.path.exists(output_path):
        return False
    if not os.path.exists(learnings_path):
        return True
    return os.path.getmtime(output_path) >= os.path.getmtime(learnings_path)


def main():
    if "--help" in sys.argv or "-h" in sys.argv:
        print(__doc__)
        sys.exit(0)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(script_dir)

    learnings_path = os.path.join(repo_root, "memory", "learnings.jsonl")
    output_path = os.path.join(repo_root, "memory", "active_learnings.md")

    if "--check" in sys.argv:
        if is_fresh(learnings_path, output_path):
            print("active_learnings.md is fresh (up to date with learnings.jsonl)")
            sys.exit(0)
        else:
            print("active_learnings.md is stale or missing — needs regeneration")
            sys.exit(1)

    n_entries = count_learnings(learnings_path)
    n_lines = synthesize(learnings_path, output_path)
    print(
        f"Synthesized {n_lines} lines from {n_entries} archive entries → {output_path}"
    )


if __name__ == "__main__":
    main()
