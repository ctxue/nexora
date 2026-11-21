# Browser Search Design

## Goal

Add local browser bookmark, history, and open-tab search to NEXORA without
uploading browsing data.

## Architecture

NEXORA reads Chromium bookmark JSON files and history SQLite databases from
Edge and Chrome profiles. Open tabs are supplied by an optional NEXORA
Companion extension through native messaging. The extension and desktop app
exchange local snapshots and tab-activation commands only.

## Privacy

- Each source has an independent, opt-in setting.
- Browser data stays local and is excluded from GitHub configuration sync.
- Only HTTP(S) entries are indexed.
- History reads are capped to recent entries and failed/locked profiles are
  skipped without breaking normal search.

## Search

Bookmarks, history, and open tabs appear in separate result groups and reuse
NEXORA fuzzy matching for titles, URLs, pinyin, and initials. Open-tab results
activate the exact tab through the bridge; bookmark/history results open in
their source browser.

## Failure Handling

- Missing or malformed browser files produce no results.
- Locked history databases are copied to a temporary snapshot before a
  read-only retry.
- The extension is optional. Without it, bookmarks and history still work.
