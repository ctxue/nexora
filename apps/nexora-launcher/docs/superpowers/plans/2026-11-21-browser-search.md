# Browser Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add local bookmark, history, and optional open-tab search.

**Architecture:** Rust reads Chromium profile files and bridge snapshots; React
adds privacy settings and local fuzzy result groups.

**Tech Stack:** Tauri 2, Rust, rusqlite, React 19, Vitest, Manifest V3.

**Spec:** `docs/superpowers/specs/2026-11-21-browser-search-design.md`

## Global Constraints

- Browser sources are opt-in and local-only.
- Browser data must not be included in GitHub configuration sync.
- Edge and Chrome are supported first.
- Existing launcher search and packaging workflows must keep working.

### Task 1: Browser Result Search

- Write failing Vitest cases for browser fuzzy matching, source filters, and
  URL opening metadata.
- Implement `BrowserEntry` types, browser result groups, and tests.
- Run `pnpm test src/core/browserSearch.test.ts`.

### Task 2: Settings and Persistence

- Write failing tests for the three browser source defaults and normalization.
- Add source settings, UI toggles, transfer-field compatibility, and tests.

### Task 3: Native Browser Data

- Add Rusqlite and Rust browser-data tests for bookmark JSON and SQLite rows.
- Implement Edge/Chrome profile discovery, bookmark parsing, history snapshots,
  and Tauri commands.

### Task 4: Companion Extension Bridge

- Add native-messaging protocol tests and bridge installation commands.
- Add the Manifest V3 extension, settings connection controls, and snapshot
  reader.

### Task 5: Integration and Release

- Connect browser results to launcher execution and refresh events.
- Run all JavaScript and Rust tests, production build, package, and verify
  artifacts.
