# AGENTS.md — happy-wakey/happy-wakey-desktop-app.rs

## Parent / root agent contract

This file is **this repository's** agent contract. The fleet-wide parent lives at:

- GitHub: https://github.com/oresoftware/my-ai/AGENTS.md
- Disk: `~/codes/oresoftware/my-ai/AGENTS.md`
- Installed by `~/codes/oresoftware/my-ai/setup-final.sh` (not `.md`) as a symlink onto `~/codes/AGENTS.md`

When this file and the parent disagree: follow **this file** for Qt/cxx-qt tools,
desktop destinations, BLE protocol, and URL safety; follow the parent for org-wide
git/Linear/GitHub/k8s/shared-auth/opto-sync/ores-otel/zed-pkg conventions.

The mapping is 1:1:1:1 — GitHub org : Linear project : GitHub org project
(usually `https://github.com/orgs/<org>/projects/1`) : Slack channel in
`oresoftware-workspace.slack.com`. Linear workspace: https://linear.app/denman
Primary GitHub user: `ORESoftware`. Secondary: `the1mills`.

## This repository

- GitHub org: [`happy-wakey`](https://github.com/happy-wakey)
- Repository: [`happy-wakey/happy-wakey-desktop-app.rs`](https://github.com/happy-wakey/happy-wakey-desktop-app.rs)
- Local checkout: `~/codes/happy-wakey/happy-wakey-desktop-app.rs`
- Linear project: https://linear.app/denman/project/githubcomhappy-wakey-f3b3dba8b195
- GitHub org project: https://github.com/orgs/happy-wakey/projects/1
- Sibling test org: `github.com/happy-wakey-test`
- Kind: canonical Rust/Qt `*-desktop-app.rs`. Do not develop in the legacy
  `happy-wakey.rs` duplicate.
- Flutter parity peer: `happy-wakey/happy-wakey-flutter`
- e2e contract: `happy-wakey/happy-wakey-e2e` (`contracts/desktop-parity.json`)
  plus `happy-wakey-test/desktop-feature-parity-e2e`.

Shared backends: `shared-auth`, `opto-sync`, `ores-otel`, `zed-pkg`,
`oresoftware/k8s-cluster`.

## Safety

- No default `HAPPY_WAKEY_PLATFORM_URL`. Cloud reminders fail closed until a
  hostname HTTPS URL (or loopback HTTP) is provided.
- Reject numeric IP hosts for platform/shared-auth/gateway except loopback.
- Bookmarks persist HTTPS (or loopback HTTP) only.
- BLE preview commands match Flutter: schema
  `happy-wakey.ble.preview-command.v1`, 512-byte bound, no tokens.
- Destinations: Home, Calendar, Weather, Markets, News, Planner, Focus,
  Devices, Browser, Settings.
- stdout is not an MCP wire here; still never log secret values.
- Git: merge, never rebase/stash/reset unless a human explicitly authorizes.

## Code style and coding patterns

remember to modularize the rust, typescript and dart - not everything belongs in main.rs, main.ts and main.dart; also follow functional coding principles - fewer side-effects (use pure functions more), more immutability (immutable variables); but for stateful apps like the client or stateful servers like websockets or tcp connections, sometimes classes and oop make more sense than functional programming perse, but we can still adhere to functional programming more than usual. Favor exhaustive pattern matching and use formal methods checking too. Favor composability and re-use , so basically create more utility functions and routines for shared use. You can follow a medium level of D.R.Y. (don't repeat yourself) - in other words you can repeat yourself at medium amount (not too much not too little). Some chaining is totally fine, so either method-chaining (immutable sometimes although with classes can be mutable too for performance), and chaining via the pipe operator is ok in languages like gleamlang.

Functional programming is mostly the following:

+ explicit inputs
+ explicit outputs
+ immutable values
+ pure transformations
+ typed errors
+ explicit state transitions
+ composition
+ effects pushed outward
+ illegal states excluded by types

## Required validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

## Repository-local Git worktrees

- Create or use a Git worktree only when the human operator explicitly authorizes it for the current task. Concurrency or a dirty checkout is not permission by itself.
- Put every authorized worktree at `<repository-root>/tmp/worktrees/<name>`; from the repository root, use `./tmp/worktrees/<name>`. Never place worktrees beside repositories or organization directories.
- Keep `tmp`, `temp`, `tmp/worktrees`, and `temp/worktrees` ignored in the repository-root `.gitignore`. Do not commit files from those directories.
- Relocate or remove a worktree only when the operator explicitly requests it. Before removal, preserve and publish intended changes, verify its commit is represented on the target branch, and confirm there are no tracked, untracked, ignored-sensitive, or in-use files that must survive. Remove it with `git worktree remove <path>` without `--force`; never delete a worktree directory with `rm`.
