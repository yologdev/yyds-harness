# Grow Your Own Agent

Fork Yoyo DeepSeek Harness, edit the project identity and lineage files, and run your own DeepSeek-focused self-evolving coding agent on GitHub Actions.

## What You Get

A coding agent that:
- Runs on GitHub Actions every ~8 hours
- Reads its own source code, picks improvements, implements them
- Writes a journal of its evolution
- Responds to trusted-owner issues in its own voice
- Gets smarter over time through a persistent memory system

## Quick Start

### 1. Fork the repo

Fork [yologdev/yyds-harness](https://github.com/yologdev/yyds-harness) on GitHub.

### 2. Edit your agent's identity

**`IDENTITY.md`** — your agent's constitution: name, mission, goals, and rules.

**`LINEAGE.md`** — your agent's family record: parent, generation, branch point, and role.

**`PERSONALITY.md`** — your agent's voice: how it writes, speaks, and expresses itself.

These are the only files you *need* to edit. Everything else auto-detects.

If you register in the yoyo family Address Book, also update **`LINEAGE.md`** so your agent has prompt-visible generation, parent, and branch-point context.

### 3. Choose your provider

Yoyo DS Harness supports 13+ providers out of the box. Pick the one that fits your budget and preferences:

| Provider | Env Var | Default Model | Notes |
|----------|---------|---------------|-------|
| `deepseek` | `DEEPSEEK_API_KEY` | `deepseek-v4-pro` | Default for this harness. |
| `anthropic` | `ANTHROPIC_API_KEY` | `claude-opus-4-6` | Strong general-purpose coding models. |
| `openai` | `OPENAI_API_KEY` | `gpt-4o` | GPT-4o and o-series models |
| `google` | `GOOGLE_API_KEY` | `gemini-2.0-flash` | Gemini models |
| `openrouter` | `OPENROUTER_API_KEY` | `anthropic/claude-sonnet-4-20250514` | Multi-provider gateway |
| `groq` | `GROQ_API_KEY` | `llama-3.3-70b-versatile` | Fast inference |
| `mistral` | `MISTRAL_API_KEY` | `mistral-large-latest` | Mistral and Codestral models |
| `xai` | `XAI_API_KEY` | `grok-3` | Grok models |
| `ollama` | *(none — local)* | `llama3.2` | Free, runs on your hardware |

For the full list of providers and models, see [Models & Providers](../configuration/models.md).

> **Tip:** Yoyo DS Harness is optimized for DeepSeek. If you are maintaining this fork as a DeepSeek-native harness, start with `DEEPSEEK_API_KEY` and override generic provider defaults only when you have a specific reason.

### 4. Create a GitHub App

Your agent needs a GitHub App to commit code and interact with issues.

1. Go to **Settings > Developer settings > GitHub Apps > New GitHub App**
2. Give it your agent's name
3. Set permissions:
   - **Repository > Contents**: Read and write
   - **Repository > Issues**: Read and write
   - **Repository > Discussions**: Read and write (optional, for social features)
4. Install it on your forked repo
5. Note the **App ID**, **Private Key** (generate one), and **Installation ID**
   - Installation ID: visit `https://github.com/settings/installations` and click your app — the ID is in the URL

### 5. Set repo secrets

In your fork, go to **Settings > Secrets and variables > Actions** and add:

| Secret | Description |
|--------|-------------|
| *Provider API key* | API key for your chosen provider (see table in step 3) |
| `APP_ID` | GitHub App ID |
| `APP_PRIVATE_KEY` | GitHub App private key (PEM) |
| `APP_INSTALLATION_ID` | GitHub App installation ID |

For the default DeepSeek-native harness, set `DEEPSEEK_API_KEY`. If you intentionally change providers, set the API key secret matching your chosen provider.

### 6. Enable the Evolution workflow

Go to **Actions** in your fork and enable the **Evolution** workflow. Your agent will start evolving on its next scheduled run, or trigger it manually with **Run workflow**.

## What Each File Does

| File | Purpose |
|------|---------|
| `IDENTITY.md` | Agent's constitution — name, mission, goals, rules |
| `LINEAGE.md` | Agent's family record — parent, generation, branch point, role |
| `PERSONALITY.md` | Agent's voice — writing style, personality traits |
| `ECONOMICS.md` | What money and resources mean to the agent |
| `journals/JOURNAL.md` | Chronological log of evolution sessions (auto-maintained) |
| `DAY_COUNT` | Tracks the agent's current evolution day |
| `memory/` | Persistent learning system (auto-maintained) |

## Costs

Costs vary by provider and model:

- **Anthropic Claude Opus** — ~$3-8 per session (~$10-25/day at 3 sessions/day)
- **Anthropic Claude Sonnet** — ~$1-3 per session, good balance of quality and cost
- **DeepSeek** — significantly cheaper, strong coding performance
- **Groq** — fast and affordable for smaller models
- **Ollama** — free (runs locally), but requires capable hardware

The default schedule runs ~3 sessions per day (8-hour gap between runs). To reduce costs, switch to a cheaper provider/model or reduce session frequency.

## Customization

### Change the provider and model

Set `PROVIDER` and `MODEL` environment variables in `.github/workflows/evolve.yml`:

```yaml
env:
  PROVIDER: openai
  MODEL: gpt-4o
```

Or set just `MODEL` to use a different DeepSeek model:

```yaml
env:
  MODEL: deepseek-chat
```

You can also edit the default directly in `scripts/evolve.sh`.

### Change session frequency

Edit the cron schedule in `.github/workflows/evolve.yml`. The default `0 * * * *` (every hour) is gated by an 8-hour gap in the script, so the agent runs ~3 times/day.

### Limit issue intake

Evolution only reads `agent-input` issues authored by trusted accounts. By default,
`TRUSTED_ISSUE_AUTHORS` falls back to the repository owner. To allow multiple
creator accounts, set a comma-separated repository secret:

```bash
gh secret set TRUSTED_ISSUE_AUTHORS --body 'your-login,teammate-login'
```

### Add custom skills

Create markdown files with YAML frontmatter in the `skills/` directory. The agent loads them automatically via `--skills ./skills`.

## The `/update` Command

The yyds binary's `/update` command checks for releases from `yologdev/yyds-harness`, not your fork. This is expected behavior. As a fork maintainer, rebuild from source after pulling changes:

```bash
cargo build --release
```

In the future, an evolve portal will provide guided setup including custom update targets.

## Optional: Dashboard Notifications

If you have a dashboard repo that accepts repository dispatch events, set a repo variable:

```bash
gh variable set DASHBOARD_REPO --body "your-user/your-dashboard" --repo your-user/your-fork
```

And add the `DASHBOARD_TOKEN` secret with a token that can dispatch to that repo.
