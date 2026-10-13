# MCP UI Designer · Vector Work Edition (preview)

October 11 update: the active `codex_editor.rs` module and streaming bridge now implement host-approved/trusted canvas operations, progress, cancellation, image inspection and bounded telemetry. See [current implementation and verification](../docs/verification/EMBEDDED-CODEX-NATURAL-BRUSHES-2026-10-11.md). The original read-only preview description below is retained as history; it no longer describes the active source module. Live inference and packaged acceptance are separate checks in the current report.

Research integrations: [StripMaker classifiers and Gurobi-free native solving](stripmaker-lab/README.md). The isolated SCIP/OSQP CPU backend passes 34 adapter checks and two native synthetic sketch fixtures. Editor cleanup controls and acceptance remain pending; this does not change this panel's read-only status.

The **MCP Designer** button in the app bar opens a direct, read-only model panel. It uses
the supplied `Codex_App_Bridge_Kit.zip` through `integrations/codex_bridge.py`.
No OpenAI API key is used; sign in with the official Codex CLI (`codex login`).

For this local source build, the default kit path is
`D:\0\Noir Fiction Engine\documentation\toolkits\Codex_App_Bridge_Kit.zip`.
Set `RAINEE_CODEX_KIT` to another ZIP path or `RAINEE_CODEX_PYTHON` to a
Python 3.11+ executable if needed. The panel currently requires the source
checkout because it launches the script from `integrations/`.

The active document inspection snapshot is sent only after **Ask Codex** is
clicked. The model gets one read-only `read_active_document` tool. It cannot
execute VectorCraft commands through this panel, and its replies are advice,
not evidence of canvas changes. The existing live MCP server remains separate:
run the app with `--control 7979`, then connect `vectorcraft-cli mcp --connect
127.0.0.1:7979`. That control port is opt-in and should only be used with
trusted local MCP clients; it is not automatically exposed by the panel.

Verified locally: `cargo check --locked -p vectorcraft` and one signed-in
Codex smoke prompt returned `bridge ready` using model `gpt-6.1-sol`;
a second prompt used the registered read-only document tool and returned the
test document's title.
The app launched, and its new panel opened through a real UI click and was
captured in a screenshot. Not yet verified: typing and receiving a response
through the mounted panel, cancellation, command approval, or a distributable
Python/kit bundle.
