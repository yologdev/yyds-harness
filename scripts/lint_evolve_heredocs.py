#!/usr/bin/env python3
"""Lint scripts/evolve.sh for recurring shell-expansion bugs in prompt text.

Bash inside ${VAR:+WORD} and ${VAR:-WORD} interprets single quotes. Any
unescaped apostrophe in the WORD opens a quoted string that scrambles
parsing until a literal } produces "bad substitution: no closing }",
killing evolve.sh before it can run the journal/learnings/issue agents.

Unquoted heredocs also perform command substitution. Markdown code spans in
agent prompts such as `list_files` or `--json` therefore run as shell commands
before the agent starts unless their backticks are escaped.

This bug has bitten three times — see commits cb9d9b0, 25f4e90, 9847db2 —
because each fix kept chasing the symptom (the journal commit instruction
printed right before the crash) instead of the cause. This lint enforces
the rules directly:
- no apostrophes inside ${VAR:+...} or ${VAR:-...} blocks
- no unescaped backticks inside unquoted heredocs

Exit codes:
  0  clean
  1  one or more shell-expansion hazards found
"""
import sys
import re
from pathlib import Path

TARGET = Path(__file__).resolve().parent.parent / "scripts" / "evolve.sh"


def find_param_expansion_blocks(src):
    """Yield (start_line, block_text) for each ${VAR:+...} or ${VAR:-...}.

    Walks the source character by character to handle nested {} correctly.
    """
    i, n = 0, len(src)
    while i < n:
        j = src.find("${", i)
        if j < 0:
            return
        # find the colon that opens :+ or :-
        k = j + 2
        while k < n and src[k] not in ":}":
            k += 1
        if k >= n or src[k] != ":" or k + 1 >= n or src[k + 1] not in "+-":
            i = j + 2
            continue
        # find the balanced closing }
        depth = 1
        m = k + 2
        while m < n and depth > 0:
            if src[m] == "{":
                depth += 1
            elif src[m] == "}":
                depth -= 1
            m += 1
        block = src[j:m]
        line = src[:j].count("\n") + 1
        yield line, block
        i = m


HEREDOC_RE = re.compile(r"<<-?\s*(['\"]?)([A-Za-z_][A-Za-z0-9_]*)\1")


def heredoc_blocks(src):
    """Yield (start_line, delimiter, quoted, body_lines) for heredocs."""
    lines = src.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        matches = list(HEREDOC_RE.finditer(line))
        if not matches:
            i += 1
            continue
        # evolve.sh uses one heredoc per command line. If that changes, linting
        # the first one is still better than silently missing prompt hazards.
        match = matches[0]
        quote = match.group(1)
        delimiter = match.group(2)
        start_line = i + 1
        body = []
        i += 1
        while i < len(lines):
            if lines[i] == delimiter:
                break
            body.append((i + 1, lines[i]))
            i += 1
        yield start_line, delimiter, bool(quote), body
        i += 1


def unescaped_backtick_positions(line):
    positions = []
    for idx, char in enumerate(line):
        if char == "`" and (idx == 0 or line[idx - 1] != "\\"):
            positions.append(idx + 1)
    return positions


def main():
    src = TARGET.read_text()
    bad_param = []
    for line, block in find_param_expansion_blocks(src):
        if "'" in block:
            bad_param.append((line, block))

    bad_heredocs = []
    for start_line, delimiter, quoted, body in heredoc_blocks(src):
        if quoted:
            continue
        bad_lines = []
        for line_no, line in body:
            if unescaped_backtick_positions(line):
                bad_lines.append((line_no, line))
        if bad_lines:
            bad_heredocs.append((start_line, delimiter, bad_lines))

    if not bad_param and not bad_heredocs:
        return 0

    if bad_param:
        print(
            "ERROR: scripts/evolve.sh contains apostrophes inside ${VAR:+...} "
            "or ${VAR:-...} blocks.\n"
            "Bash interprets single quotes inside parameter expansion WORDs, so "
            "an apostrophe opens a quoted string that scrambles parsing until a "
            'literal } produces "bad substitution: no closing }".\n'
            "Fix: rephrase to avoid the apostrophe (Do not, Here is, etc).\n"
        )
        for line, block in bad_param:
            print(f"--- block starting at scripts/evolve.sh:{line} ---")
            for offset, ln in enumerate(block.splitlines()):
                if "'" in ln:
                    print(f"  line {line + offset}: {ln.rstrip()}")
            print()

    if bad_heredocs:
        print(
            "ERROR: scripts/evolve.sh contains unescaped backticks inside "
            "unquoted heredocs.\n"
            "Bash performs command substitution in unquoted heredocs, so "
            "Markdown code spans like `list_files` run before yyds starts.\n"
            "Fix: escape Markdown backticks as \\`...\\` or use a quoted "
            "heredoc with explicit variable substitution.\n"
        )
        for start_line, delimiter, bad_lines in bad_heredocs:
            print(f"--- heredoc {delimiter} starting at scripts/evolve.sh:{start_line} ---")
            for line_no, line in bad_lines:
                print(f"  line {line_no}: {line.rstrip()}")
            print()
    return 1


if __name__ == "__main__":
    sys.exit(main())
