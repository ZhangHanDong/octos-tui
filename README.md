# octos-tui

Standalone terminal UI client for the Octos AppUi/UI Protocol.

This repo is intentionally separate from `octos-cli`. The Octos repo owns the
server/runtime and shared `octos-core` protocol types; this repo owns the TUI
binary and connects over the UI Protocol WebSocket.

## Local Development

```bash
cargo run -- --mode mock
cargo run -- --mode protocol --endpoint ws://127.0.0.1:50080/api/ui-protocol/ws --session local:demo
```

The local dependency on `../octos/crates/octos-core` is the development contract
boundary. For release, pin `octos-core` to the matching git tag or published
crate version.

## M9.17 Pane Data Status

`octos-tui` consumes AppUi/UI Protocol fields instead of inventing local wire
extensions. Live task updates hydrate the Tasks pane, and `task/output/read`
metadata such as `output_files` and limitations is retained in the task-output
viewer even when the read window omits those lines.

Artifact, workspace, and git pane hydration is governed by accepted
`UPCR-2026-002`. In protocol mode the client requests `pane.snapshots.v1` and
hydrates those panes from optional `session/open.panes` payloads when the server
supports them. When the field is absent, the TUI keeps the explicit fallback
data from snapshot sessions, task tails, launch target, and status.
