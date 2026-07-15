# Janet School

Janet School is a standalone Rust research rig for studying whether a deterministic MCM student shows reusable structure, transfer behavior, boundary pressure, anomaly clusters, and emergent structure candidates inside a controlled curriculum scaffold.

This is not an education app, not a chatbot product, and not a general-purpose AI tutor. The system is designed to generate evidence, logs, and cautious analysis for research review.

## Current Status

Janet School is currently at a usable v0.1 research checkpoint.

- Windows build and standalone acceptance now have a repeatable run sheet in [docs/WINDOWS_ACCEPTANCE.md](docs/WINDOWS_ACCEPTANCE.md).
- The mock teacher path is working and is the recommended first-run path.
- The local-LLM teacher path is wired, tested, and ready when a valid local runtime and model are present.
- The GUI, telemetry, analysis, compare/export surfaces, and release-checkpoint docs are in place.
- Remaining work is mostly optional post-v0.1 compatibility or extension work, not missing core rig foundations.

## What It Does

- Runs a deterministic MCM student named Janet.
- Uses a teacher backend to generate or support curriculum sessions.
- Captures detailed session telemetry and append-only event logs.
- Produces conservative analysis reports.
- Provides a local operator GUI for setup, runs, review, and compare/export work.
- Shows a lightweight FMI startup splash before the GUI shell appears.

## What It Does Not Do

- It does not let the MCM answer with an LLM.
- It does not treat curriculum labels as ground truth.
- It does not hide anomalies to make results look cleaner.
- It does not claim abstraction is proven from a single success.
- It does not require cloud APIs for the current local workflow.

## Fast Start

1. Install Rust if it is not already installed.
2. Open a terminal in this project root.
3. Run `cargo test` if you want a quick confidence check before first use.
4. Run `cargo run -- validate-config` to confirm the config loads.
5. Run `cargo run -- serve-gui`.
6. The browser should open automatically to `http://127.0.0.1:8787` in standalone mode.
7. Let the FMI splash clear or skip it.
8. Start with the `mock` teacher backend for your first run.

If Janet School is launched from ChattyCog as a hosted module, the GUI server stays inside ChattyCog and does not open a separate browser tab.

If you want a formal pass/fail machine check instead of a quick start, use [docs/WINDOWS_ACCEPTANCE.md](docs/WINDOWS_ACCEPTANCE.md).

## Main Commands

- `cargo run -- validate-config`
- `cargo run -- serve-gui`
- `cargo run -- serve-gui --no-browser`
- `cargo run -- sync-gui-state`
- `cargo run -- generate-curriculum --teacher-backend mock --session-name "My Session"`
- `cargo run -- run-session --teacher-backend mock --session-name "My Session"`
- `cargo run -- inspect-runtime`
- `cargo run -- run-mcm-prompt --prompt "What is left of banana in the row: apple, banana, cherry."`

Power-user note:

- Use `cargo run -- serve-gui --no-browser` if you want the local GUI server without automatic browser launch.
- Janet School also suppresses browser auto-launch automatically when hosted by ChattyCog.

## Main Folders

- `config/`
  Configuration files for the app, teacher backend, MCM policy, and skill governance.
- `data/sessions/`
  Per-session outputs, including JSONL event logs and reports.
- `data/aggregated/`
  Aggregated outputs for later cross-session work.
- `compare_exports/`
  Saved compare-run exports written from the GUI.
- `docs/`
  User, workflow, doctrine, architecture, schema, and planning docs.
- `models/`
  Local model files such as GGUFs used by the local teacher backend.
- `runtime/`
  Local runtime binaries such as `llama-server.exe`.

## Recommended Reading Order

1. [docs/DOCS_INDEX.md](docs/DOCS_INDEX.md)
2. [docs/USER_MANUAL.md](docs/USER_MANUAL.md)
3. [docs/RUNBOOK.md](docs/RUNBOOK.md)
4. [docs/WINDOWS_ACCEPTANCE.md](docs/WINDOWS_ACCEPTANCE.md)
5. [docs/CONFIG_REFERENCE.md](docs/CONFIG_REFERENCE.md)
6. [docs/ABOUT.md](docs/ABOUT.md)
7. [docs/ALIGNMENT.md](docs/ALIGNMENT.md)
8. [docs/SKILL_EXTENSION_GUIDE.md](docs/SKILL_EXTENSION_GUIDE.md)
9. [GLOSSARY.md](GLOSSARY.md)

## Release And Handoff

- [RELEASE_CHECKPOINT.md](RELEASE_CHECKPOINT.md)
  Short root-level checkpoint template for marking a build or handoff as accepted.

## First Places To Look

- Want to run it right now: [docs/RUNBOOK.md](docs/RUNBOOK.md)
- Want a formal acceptance pass: [docs/WINDOWS_ACCEPTANCE.md](docs/WINDOWS_ACCEPTANCE.md)
- Want the zero-knowledge operator guide: [docs/USER_MANUAL.md](docs/USER_MANUAL.md)
- Want config meanings before editing anything: [docs/CONFIG_REFERENCE.md](docs/CONFIG_REFERENCE.md)

## Current Operator Advice

- Start with `mock` before trying `local-llm`.
- Treat every result as provisional.
- Use skill approvals deliberately. Memory-only and restricted-skill runs are useful comparison tools.
- Use the compare panel to inspect differences between runs, then save reports into `compare_exports/`.
- Hover or focus interactive controls in the GUI to see built-in tooltip help.

## License And Status

Janet School is published under Fractal Media Infrastructure, a small independent research-and-development umbrella for open-source AI tooling, cognitive scaffolding experiments, and local-first research systems.

Public GitHub work is currently maintained under `Instance001`.

Media, demos, and outreach may appear through separate channels over time.

Janet School is licensed under the GNU Affero General Public License v3.0 or later (`AGPL-3.0-or-later`).

The full license text is in [LICENSE](LICENSE).

Stewardship and public project identity are summarized in [docs/ABOUT.md](docs/ABOUT.md).

This program is distributed without warranty. See the license for details.

This workspace is currently an active research build checkpoint rather than a polished public release. Treat the docs and outputs as operator-facing research material.
