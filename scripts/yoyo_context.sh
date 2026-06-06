#!/bin/bash
# scripts/yoyo_context.sh — Build yoyo's identity context for prompts.
# Source this file, then use $YOYO_STABLE_CONTEXT before dynamic session data
# when prompt-prefix cache reuse matters. $YOYO_CONTEXT remains as the legacy
# combined block for callers that do not need the split.
#
# Usage:
#   YOYO_REPO="/path/to/yoyo-evolve" source scripts/yoyo_context.sh
#   cat > prompt.txt <<EOF
#   $YOYO_CONTEXT
#   ... your task-specific instructions ...
#   EOF
#
# Reads: IDENTITY.md, LINEAGE.md, PERSONALITY.md, ECONOMICS.md,
# memory/active_learnings.md, memory/active_social_learnings.md.
# Identity, lineage, voice, and economics are stable prefix material. Active
# learnings change over time and are exposed separately as $YOYO_DYNAMIC_CONTEXT.

_YOYO_REPO="${YOYO_REPO:-.}"

_IDENTITY=""
if [ -f "$_YOYO_REPO/IDENTITY.md" ]; then
    _IDENTITY=$(cat "$_YOYO_REPO/IDENTITY.md") || {
        echo "WARNING: Failed to read IDENTITY.md" >&2
        _IDENTITY=""
    }
else
    echo "WARNING: IDENTITY.md not found at $_YOYO_REPO/IDENTITY.md" >&2
fi

_LINEAGE=""
if [ -f "$_YOYO_REPO/LINEAGE.md" ]; then
    _LINEAGE=$(cat "$_YOYO_REPO/LINEAGE.md") || {
        echo "WARNING: Failed to read LINEAGE.md" >&2
        _LINEAGE=""
    }
fi

_PERSONALITY=""
if [ -f "$_YOYO_REPO/PERSONALITY.md" ]; then
    _PERSONALITY=$(cat "$_YOYO_REPO/PERSONALITY.md") || {
        echo "WARNING: Failed to read PERSONALITY.md" >&2
        _PERSONALITY=""
    }
else
    echo "WARNING: PERSONALITY.md not found at $_YOYO_REPO/PERSONALITY.md" >&2
fi

# Active learnings — no warning if missing
_LEARNINGS=""
if [ -f "$_YOYO_REPO/memory/active_learnings.md" ]; then
    _LEARNINGS=$(cat "$_YOYO_REPO/memory/active_learnings.md") || _LEARNINGS=""
fi

# Active social learnings — no warning if missing
_SOCIAL_LEARNINGS=""
if [ -f "$_YOYO_REPO/memory/active_social_learnings.md" ]; then
    _SOCIAL_LEARNINGS=$(cat "$_YOYO_REPO/memory/active_social_learnings.md") || _SOCIAL_LEARNINGS=""
fi

# Economics — what money and resources mean to yoyo
_ECONOMICS=""
if [ -f "$_YOYO_REPO/ECONOMICS.md" ]; then
    _ECONOMICS=$(cat "$_YOYO_REPO/ECONOMICS.md") || _ECONOMICS=""
fi

YOYO_STABLE_CONTEXT="=== WHO YOU ARE ===

${_IDENTITY:-Read IDENTITY.md for your rules and constitution.}

=== YOUR LINEAGE ===

${_LINEAGE:-No lineage file found. If this is a fork, create LINEAGE.md with your parent and generation.}

=== YOUR VOICE ===

${_PERSONALITY:-Read PERSONALITY.md for your voice and values.}

=== YOUR ECONOMICS ===

${_ECONOMICS:-Read ECONOMICS.md for what money and resources mean to you.}"

YOYO_DYNAMIC_CONTEXT="=== SELF-WISDOM ===

${_LEARNINGS:-No learnings yet.}

=== SOCIAL WISDOM ===

${_SOCIAL_LEARNINGS:-No social learnings yet.}"

YOYO_CONTEXT="${YOYO_STABLE_CONTEXT}

=== SELF-WISDOM ===

${_LEARNINGS:-No learnings yet.}

=== SOCIAL WISDOM ===

${_SOCIAL_LEARNINGS:-No social learnings yet.}

=== YOUR ECONOMICS ===

${_ECONOMICS:-Read ECONOMICS.md for what money and resources mean to you.}"
