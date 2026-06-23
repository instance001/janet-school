# Janet School Unfinished Checklist

This checklist tracks the meaningful remaining work from the original build outline in [build-plan-fox-codex.md](C:/Users/User/Desktop/github_portal/janet-school/build-plan-fox-codex.md).

It is intentionally focused on unfinished or partially finished items, not work that is already substantially complete.

## Now

## Soon

## Later

- [ ] Add a ChattyCog handoff/export summary surface later if still desired.
  This remains intentionally out of scope for v0.1 and should stay that way unless we explicitly choose to begin post-v0.1 compatibility work.

## Notes

- Phase 0 through Phase 6 are substantially implemented in foundation form.
- Full curriculum generation now reaches the original target shape in [src/curriculum/mod.rs](C:/Users/User/Desktop/github_portal/janet-school/src/curriculum/mod.rs):
  - 12 domains
  - 5 concepts per domain
  - 5 teaching items per concept
  - 3 probes per concept
  - 480 total items
  - 180 total probes
- The GUI now supports editable setup controls, direct session-folder opening, and workspace-owned JSON bundle export.
- Cross-session analysis now uses prior completed sessions instead of a placeholder summary field.
- Analyzer acceptance coverage now explicitly exercises:
  - repeated anomaly clustering
  - category mismatch clustering
  - provisional emergent candidate wording
  - required report section completeness
- Local-LLM acceptance coverage now explicitly exercises:
  - curriculum generation from a ready local endpoint
  - clean failure when the endpoint is unavailable and runtime launch is disabled
  - session artifact logging for a local-teacher-backed run
- The Windows standalone acceptance pass now has a dedicated run sheet in [WINDOWS_ACCEPTANCE.md](C:/Users/User/Desktop/github_portal/janet-school/docs/WINDOWS_ACCEPTANCE.md).
- The remaining work is mostly about:
  - optional later compatibility/export work beyond v0.1
