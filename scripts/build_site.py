#!/usr/bin/env python3
"""Build the Yoyo DeepSeek Harness website from markdown sources."""

import html
import re
from itertools import groupby
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "site"


def read_file(name):
    try:
        return (ROOT / name).read_text()
    except FileNotFoundError:
        print(f"WARNING: {name} not found — section will be empty")
        return ""


def md_inline(text):
    """Convert inline markdown (bold, code, links) to HTML."""
    text = html.escape(text)
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"`(.+?)`", r"<code>\1</code>", text)
    text = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', text)
    return text


# ── Parsers ──


def parse_journal(content):
    entries = []
    chunks = re.split(r"^## ", content, flags=re.MULTILINE)
    for chunk in chunks:
        chunk = chunk.strip()
        if not chunk:
            continue
        lines = chunk.split("\n")
        m = re.match(r"Day\s+(\d+)\s*[—–\-]+\s*(.+)", lines[0])
        if not m:
            continue
        day = int(m.group(1))
        title = m.group(2).strip()
        body = "\n".join(lines[1:]).strip()
        entries.append({"day": day, "title": title, "body": body})
    return entries



def parse_identity(content):
    intro_lines = []
    rules = []
    sections = re.split(r"^## ", content, flags=re.MULTILINE)
    for section in sections:
        section = section.strip()
        if not section:
            continue
        lines = section.split("\n")
        header = lines[0].strip()
        # Intro: everything before the first ## (starts with # title)
        if header.startswith("# ") or header.startswith("Who "):
            for line in lines[1:] if header.startswith("# ") else lines:
                if line.strip():
                    intro_lines.append(line.strip())
        elif "rule" in header.lower():
            for line in lines[1:]:
                m = re.match(r"^\d+\.\s+\*\*(.+?)\*\*(.*)$", line)
                if m:
                    rules.append(
                        f"<strong>{html.escape(m.group(1))}</strong>"
                        f"{md_inline(m.group(2))}"
                    )
                elif re.match(r"^\d+\.", line):
                    text = line.split(".", 1)[1].strip()
                    rules.append(md_inline(text))
    return {"intro": intro_lines, "rules": rules}


# ── Renderers ──


def render_entry_body(body):
    """Render a journal entry body to HTML.

    Splits on blank lines into blocks. A block starting with `### ` becomes
    an <h4>; anything else becomes a <p>. Single newlines within a block
    become <br>. Inline markdown (bold, code, links) is handled by md_inline.
    """
    blocks = re.split(r"\n\s*\n", body.strip())
    out = []
    for block in blocks:
        block = block.strip()
        if not block:
            continue
        if block.startswith("### "):
            # Subheading line (possibly followed by body lines in same block).
            lines = block.split("\n", 1)
            heading = lines[0][4:].strip()
            out.append(f'<h4 class="entry-subheading">{md_inline(heading)}</h4>')
            if len(lines) > 1 and lines[1].strip():
                rest = md_inline(lines[1]).replace("\n", "<br>")
                out.append(f'<p class="entry-body-para">{rest}</p>')
        else:
            rendered = md_inline(block).replace("\n", "<br>")
            out.append(f'<p class="entry-body-para">{rendered}</p>')
    return "\n          ".join(out)


def render_journal(entries):
    if not entries:
        return (
            '<div class="timeline-empty">'
            "No evolution journal entries yet."
            "</div>"
        )
    parts = []
    # Group consecutive entries by day so multi-session days share one header.
    # Works automatically for future entries since it operates on parsed data.
    for day, day_entries in groupby(entries, key=lambda e: e["day"]):
        parts.append(f'      <div class="day-group">')
        parts.append(f'        <div class="day-separator">Day {day}</div>')
        for entry in day_entries:
            body_html = render_entry_body(entry["body"]) if entry["body"] else ""
            parts.append(
                f'        <article class="entry">\n'
                f'          <div class="entry-marker"></div>\n'
                f'          <div class="entry-content">\n'
                f'            <h3 class="entry-title">{md_inline(entry["title"])}</h3>\n'
                f'            <div class="entry-body">\n            {body_html}\n            </div>\n'
                f"          </div>\n"
                f"        </article>"
            )
        parts.append(f'      </div>')
    return "\n".join(parts)


def render_harness_overview(day_count):
    cards = [
        {
            "title": "Interactive evolution dashboard",
            "body": "Review harness gnomes, eval decisions, patch lifecycle evidence, hotspots, and audit-branch summaries.",
            "href": "evolution/",
        },
        {
            "title": "DeepSeek harness documentation",
            "body": "Read the operator docs for the DeepSeek-native profile, state boundary, eval gates, and fork setup.",
            "href": "book/",
        },
        {
            "title": f"Day {day_count} harness state",
            "body": "Follow the current harness evolution without exposing yoagent-state internals to end users of yoyo-ds.",
            "href": "https://github.com/yologdev/yyds-harness/actions/workflows/evolve.yml",
        },
    ]
    parts = []
    for card in cards:
        parts.append(
            f'        <article class="entry">\n'
            f'          <div class="entry-marker"></div>\n'
            f'          <div class="entry-content">\n'
            f'            <h3 class="entry-title"><a href="{card["href"]}">{md_inline(card["title"])}</a></h3>\n'
            f'            <div class="entry-body">\n'
            f'              <p class="entry-body-para">{md_inline(card["body"])}</p>\n'
            f"            </div>\n"
            f"          </div>\n"
            f"        </article>"
        )
    return "\n".join(parts)



def render_identity(identity):
    parts = []
    if identity["intro"]:
        # First paragraph as mission statement
        mission = md_inline(identity["intro"][0])
        parts.append(f'      <p class="mission">{mission}</p>')
        # Remaining paragraphs
        for line in identity["intro"][1:]:
            parts.append(f'      <p class="identity-text">{md_inline(line)}</p>')
    if identity["rules"]:
        parts.append('      <ol class="rules">')
        for rule in identity["rules"]:
            parts.append(f"        <li>{rule}</li>")
        parts.append("      </ol>")
    return "\n".join(parts)


def render_harness_identity():
    return """\
      <p class="mission">Yoyo DeepSeek Harness is the internal evolution layer for making yoyo work reliably with DeepSeek. It tracks harness gnomes, eval evidence, failures, decisions, patches, and dashboard artifacts without exposing that state machinery to end users of the CLI.</p>
      <p class="identity-text">The harness uses <code>yoagent-state</code> as its durable evidence substrate. Git remains the source of concrete code changes; state records why those changes exist, what they improve, and which DeepSeek-specific risks or KPIs they affect.</p>
      <ol class="rules">
        <li><strong>DeepSeek first.</strong> Improve protocol coverage, cache behavior, tool-call reliability, prompt layout, context policy, and eval gates from recorded evidence.</li>
        <li><strong>Harness boundary.</strong> Keep state and evolution analytics in the harness layer, not in the user-facing yoyo/yoyo-ds CLI experience.</li>
        <li><strong>Trusted intake.</strong> Evolution reads trusted-owner feedback and state-derived work items before deciding what to improve.</li>
        <li><strong>Reviewable evidence.</strong> Dashboards and audit artifacts should explain gnome/KPI movement, not bury it in raw code changes.</li>
      </ol>
"""


# ── Templates ──


HTML_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Yoyo DeepSeek Harness \u2014 Day {day_count}</title>
  <meta name="description" content="A DeepSeek-native coding agent harness that evolves from state evidence. Currently on Day {day_count}.">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:ital,wght@0,300;0,400;0,500;0,700;1,400&display=swap" rel="stylesheet">
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <nav>
    <a href="#" class="nav-name">Yoyo DS Harness</a>
    <div class="nav-links">
      <a href="book/">docs</a>
      <a href="evolution/">dashboard</a>
      <a href="#journal">evidence</a>
      <a href="https://github.com/yologdev/yyds-harness" target="_blank" rel="noopener">github \u2197</a>
    </div>
  </nav>

  <main>
    <header class="hero">
      <div class="hero-prompt">
        <span class="hero-prompt-sigil">$</span>
        <span class="hero-cmd">yoyo-ds --deepseek-native --status</span>
      </div>
      <h1>Yoyo DeepSeek Harness<span class="cursor">_</span></h1>
      <p class="hero-status">day {day_count}<span class="sep">·</span><span class="status-tag">state-backed harness evolution</span></p>
    </header>

    <section id="journal">
      <h2 class="section-label">// evidence surfaces</h2>
      <div class="timeline">
{journal_html}
      </div>
    </section>

    <section id="identity">
      <h2 class="section-label">// harness boundary</h2>
{identity_html}
    </section>
  </main>

  <footer>
    <p>DeepSeek harness evolution powered by yoagent-state</p>
    <a href="https://github.com/yologdev/yyds-harness">github.com/yologdev/yyds-harness</a>
  </footer>
</body>
</html>
"""

CSS = """\
/* Yoyo DeepSeek Harness — terminal chronicle */

:root {
  --bg: #0a0c10;
  --bg-raised: #12161c;
  --border: #1e2330;
  --text: #9ca3af;
  --text-bright: #d1d5db;
  --text-dim: #4a5568;
  --cyan: #22d3ee;
  --green: #34d399;
  --amber: #f59e0b;
  --red: #ef4444;
  --font: "JetBrains Mono", "Fira Code", "Cascadia Code", "Source Code Pro", monospace;

  /* type scale */
  --fs-micro: 0.72rem;
  --fs-small: 0.82rem;
  --fs-body:  0.9rem;
  --fs-lead:  1rem;
  --fs-title: 1.1rem;
  --fs-hero:  2.65rem;

  /* layout */
  --col:      720px;
}

*, *::before, *::after {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html {
  scroll-behavior: smooth;
  scroll-padding-top: 4rem;
}

body {
  background: var(--bg);
  color: var(--text);
  font-family: var(--font);
  font-size: 14.5px;
  line-height: 1.65;
  -webkit-font-smoothing: antialiased;
}

a {
  color: var(--cyan);
  text-decoration: none;
}

a:hover {
  text-decoration: underline;
}

strong {
  color: var(--text-bright);
  font-weight: 500;
}

code {
  background: var(--bg-raised);
  padding: 0.15em 0.4em;
  font-size: 0.9em;
  border: 1px solid var(--border);
}


/* ── nav ── */

nav {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: space-between;
  max-width: var(--col);
  width: 90%;
  margin: 0 auto;
  padding: 1rem 0;
  border-bottom: 1px solid var(--border);
  background: var(--bg);
}

.nav-name {
  font-weight: 700;
  font-size: var(--fs-small);
  color: var(--cyan);
  letter-spacing: 0.05em;
}

.nav-name:hover {
  text-decoration: none;
  opacity: 0.8;
}

.nav-links {
  display: flex;
  gap: 1.5rem;
}

.nav-links a {
  color: var(--text-dim);
  font-size: var(--fs-micro);
  letter-spacing: 0.08em;
}

.nav-links a:hover {
  color: var(--text);
  text-decoration: none;
}


/* ── main ── */

main {
  max-width: var(--col);
  width: 90%;
  margin: 0 auto;
}


/* ── hero ── */

.hero {
  padding: 5rem 0 4rem;
}

.hero-prompt {
  font-size: var(--fs-small);
  color: var(--text-dim);
  letter-spacing: 0.04em;
  margin-bottom: 1.25rem;
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
}

.hero-prompt-sigil {
  color: var(--green);
  font-weight: 700;
}

.hero-cmd {
  color: var(--text);
}

.hero h1 {
  font-size: var(--fs-hero);
  font-weight: 700;
  color: var(--cyan);
  line-height: 1;
  letter-spacing: -0.02em;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.cursor {
  animation: blink 1.2s step-end infinite;
  color: var(--cyan);
  font-weight: 300;
}

.hero-status {
  margin-top: 1rem;
  font-size: var(--fs-body);
  color: var(--green);
  font-weight: 500;
  letter-spacing: 0.01em;
}

.hero-status .sep {
  color: var(--text-dim);
  margin: 0 0.5rem;
  font-weight: 400;
}

.hero-status .status-tag {
  color: var(--text-dim);
  font-style: italic;
  font-weight: 400;
}


/* ── sections ── */

section {
  padding: 3.5rem 0 0;
}

.section-label {
  font-size: var(--fs-micro);
  font-weight: 400;
  color: var(--text-dim);
  letter-spacing: 0.12em;
  margin-bottom: 2rem;
}


/* ── journal timeline ── */

.timeline {
  position: relative;
  padding-left: 28px;
}

.timeline::before {
  content: '';
  position: absolute;
  left: 3px;
  top: 6px;
  bottom: 0;
  width: 1px;
  background: var(--border);
}

.timeline-empty {
  color: var(--text-dim);
  font-style: italic;
  padding-left: 28px;
}

.day-group {
  margin-bottom: 3rem;
}

.day-group:last-child {
  margin-bottom: 0;
}

.day-separator {
  position: relative;
  font-size: var(--fs-micro);
  font-weight: 700;
  color: var(--green);
  letter-spacing: 0.12em;
  text-transform: uppercase;
  margin-bottom: 1.75rem;
  padding-left: 0.25rem;
}

.day-separator::before {
  content: '';
  position: absolute;
  left: -28px;
  top: 50%;
  width: 13px;
  height: 1px;
  background: var(--green);
  opacity: 0.6;
}

.entry {
  position: relative;
  border-top: 1px solid var(--border);
  padding-top: 1.75rem;
  margin-top: 1.75rem;
}

.entry:first-of-type {
  border-top: none;
  padding-top: 0;
  margin-top: 0;
}

.entry-marker {
  position: absolute;
  left: -28px;
  top: 8px;
  width: 7px;
  height: 7px;
  background: var(--green);
}

.entry:first-of-type .entry-marker {
  top: 6px;
}

.entry-title {
  font-size: var(--fs-title);
  font-weight: 500;
  color: var(--text-bright);
  margin: 0 0 0.6rem;
  line-height: 1.4;
  letter-spacing: -0.005em;
}

.entry-body {
  color: var(--text);
  font-size: var(--fs-body);
  line-height: 1.72;
}

.entry-body-para {
  margin: 0 0 0.9rem;
}

.entry-body-para:last-child {
  margin-bottom: 0;
}

.entry-subheading {
  font-size: var(--fs-small);
  font-weight: 600;
  color: var(--cyan);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  margin: 1.6rem 0 0.6rem;
  padding-bottom: 0.35rem;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: baseline;
  gap: 0.55rem;
}

.entry-subheading::before {
  content: "▸";
  color: var(--cyan);
  font-size: var(--fs-micro);
  opacity: 0.85;
}

.entry-subheading:first-child {
  margin-top: 0.2rem;
}


/* ── identity ── */

.mission {
  font-size: var(--fs-lead);
  color: var(--text-bright);
  line-height: 1.75;
  margin-bottom: 1.5rem;
  padding-left: 1rem;
  border-left: 2px solid var(--cyan);
}

.identity-text {
  font-size: var(--fs-body);
  line-height: 1.7;
  margin-bottom: 1rem;
}

.rules {
  list-style: none;
  counter-reset: rules;
  padding: 0;
  margin-top: 2rem;
}

.rules li {
  counter-increment: rules;
  position: relative;
  padding-left: 2.5rem;
  margin-bottom: 0.75rem;
  font-size: var(--fs-body);
  line-height: 1.7;
}

.rules li::before {
  content: counter(rules, decimal-leading-zero);
  position: absolute;
  left: 0;
  color: var(--text-dim);
  font-size: var(--fs-micro);
  font-weight: 300;
  top: 0.15rem;
}


/* ── footer ── */

footer {
  max-width: var(--col);
  width: 90%;
  margin: 4rem auto 0;
  padding: 2rem 0 4rem;
  border-top: 1px solid var(--border);
}

footer p {
  font-size: var(--fs-micro);
  color: var(--text-dim);
  margin-bottom: 0.25rem;
}

footer a {
  font-size: var(--fs-micro);
  color: var(--text-dim);
}

footer a:hover {
  color: var(--cyan);
}


/* ── responsive ── */

@media (max-width: 480px) {
  :root {
    --fs-hero: 2.1rem;
  }

  nav {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .nav-links {
    gap: 1rem;
  }
}
"""


# ── Build ──


def build():
    day_count = 0
    try:
        day_count = int(read_file("DAY_COUNT").strip())
    except (ValueError, AttributeError):
        pass

    journal_html = render_harness_overview(day_count)
    identity_html = render_harness_identity()

    page = HTML_TEMPLATE.format(
        day_count=day_count,
        journal_html=journal_html,
        identity_html=identity_html,
    )

    DOCS.mkdir(exist_ok=True)
    (DOCS / "index.html").write_text(page)
    (DOCS / "style.css").write_text(CSS)
    (DOCS / ".nojekyll").touch()

    print(f"Site built: site/index.html (Day {day_count})")


if __name__ == "__main__":
    build()
