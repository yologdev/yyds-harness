Title: Create x-research skill for reading X/Twitter via xurl
Files: skills/x-research/SKILL.md (new)
Issue: #355

Create a new skill file `skills/x-research/SKILL.md` that wraps xurl for read-only X/Twitter access.

## Frontmatter

```yaml
---
name: x-research
description: Read X (Twitter) via xurl — search posts, fetch threads, read profiles, and read long-form articles
tools: [bash, read_file]
origin: yoyo
status: active
score: 0.0
uses: 0
wins: 0
last_used: null
last_evolved: null
parent_pattern_key: null
keywords: ["xurl", "twitter", "x.com", "tweet", "thread", "x-research"]
---
```

## Skill body structure

Follow the pattern from `research/SKILL.md` and `social/SKILL.md` but adapted for X access:

### 1. When to use / When NOT to use
- Use: when researching what people are saying on X about a topic, reading a specific thread, checking a profile's recent posts, reading X Articles
- Don't use for: general web research (use `research` skill), posting/liking/following (read-only), bulk historical scraping, real-time monitoring

### 2. Prerequisites section
- `xurl` must be on PATH — check with `xurl --version`
- Auth must be configured — check for `~/.xurl` directory
- If either fails: print clear setup instructions and exit, don't retry

### 3. Four primitives

**search** — `xurl GET "/2/tweets/search/recent?query=QUERY&max_results=10&tweet.fields=created_at,author_id,public_metrics,text"`
- URL-encode the query
- Show: text, author, date, engagement metrics
- Cost: 1 request per call

**thread** — Given a tweet ID, fetch the tweet, find conversation_id, then search for replies in that conversation
- `xurl GET "/2/tweets/TWEET_ID?tweet.fields=conversation_id,author_id,created_at,text"`
- Then search: `xurl GET "/2/tweets/search/recent?query=conversation_id:CONV_ID&max_results=50&tweet.fields=created_at,author_id,text"`
- Reconstruct chronological order
- Cost: 2 requests per call

**profile** — Given a username, fetch recent posts
- `xurl GET "/2/users/by/username/USERNAME?user.fields=description,public_metrics"`
- `xurl GET "/2/users/USER_ID/tweets?max_results=10&tweet.fields=created_at,public_metrics,text"`
- Show: bio, follower count, recent posts
- Cost: 2 requests per call

**article** — Given an article URL or tweet ID, investigate and fetch long-form X Article content
- Research which API path works for X Articles (note in the skill that this needs investigation at first use)
- Try `/2/tweets/{id}?tweet.fields=note_tweet` or similar expanded fields
- If API doesn't expose article text, fall back to fetching the HTML page and extracting content
- Cost: 1-2 requests per call

### 4. Caching strategy
- Cache dir: `.yoyo/x-research-cache/` (gitignored)
- TTL: 15 minutes for search, 1 hour for threads/profiles/articles
- Cache key: SHA256 of the full API URL
- Before every API call: check cache first
- `--no-cache` flag for when freshness matters
- Implementation: save raw JSON response to cache file, check mtime for TTL

### 5. Cost awareness section
- Every primitive notes its rough cost (requests per call)
- Explicit instruction: "consider cache before every call"
- Note that pay-per-use billing means unnecessary requests cost real money

### 6. Failure modes
- `xurl` not in PATH → setup instructions, exit
- Auth expired/missing → tell user to run `xurl auth oauth2`, exit
- Rate limited (429) → back off 60s, retry once, then give up
- Empty results → report "no results found", don't retry with broader query
- Network timeout → single retry with 2s delay, then give up

### 7. Rules
- Read-only — never post, like, follow, DM, or modify anything
- Don't hide costs — every invocation should be visible
- Don't build knowledge-base ingestion — that's Layer 2
- Don't try to authenticate autonomously — human does that

## Acceptance criteria
- Valid YAML frontmatter with all required fields
- All four primitives documented with exact xurl commands
- Caching strategy with TTL and cache-key scheme
- Failure modes documented
- Cost awareness section present
- "When NOT to use" section present
- No attempts to post/write to X
