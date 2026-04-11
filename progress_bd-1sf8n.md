# bd-1sf8n progress

Current slice: Phase 9 compliance harness hardening after the time-travel gate landed, specifically making sure the shared Phase 7/8/9 proptest corpus can no longer forget a newly-added Phase 9 marker.

Implemented in this increment:
- Added the missing `Time-Travel MVCC Snapshot Verification` entry to the `REQUIRED_TOKENS` mutation corpus in `crates/fsqlite-harness/tests/bd_331_4_phase_7_8_9_verification_gates_compliance.rs`, so the property-based compliance check now mutates that Phase 9 requirement instead of silently skipping it.
- Added a dedicated `test_required_tokens_cover_all_compliance_requirements` guard in the same harness test to assert that every declared unit id, phase-gate id, E2E id, phase marker, log level, and logging-standard reference is represented in `REQUIRED_TOKENS`.
- Kept the previously-landed `bd_1sf8n_phase9_time_travel_gate` and shared Phase 9 marker wiring intact; this increment hardens the self-checks around that gate rather than changing the runtime behavior.

Notes:
- `bd-1mt2x` is still blocked by `bd-3mgq5`, so this commit remains an epic-level verification increment rather than a claim/closure of the child bead.
- Guardrails preserved: `concurrent_mode_default` remains on by default, no `unsafe`, no Tokio, manual edits only.
- The repo already had unrelated local edits in other files; this slice stays confined to the harness compliance test and this progress note.

Verification target for this increment:
- `cargo test -p fsqlite-harness --test bd_331_4_phase_7_8_9_verification_gates_compliance -- --nocapture --test-threads=1`
- `cargo test -p fsqlite-harness test_required_tokens_cover_all_compliance_requirements -- --nocapture`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all --check`
