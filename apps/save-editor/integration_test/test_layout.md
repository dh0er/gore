# goresave Test Layout

Use `python test.py` from the repo root.

| Suite | Location | Purpose |
| --- | --- | --- |
| Rust | `crates/goresave_core` | Parser, writer, codec diagnostics, FFI contract |
| Flutter | `apps/goresave/test` | Models, state, widget shell |
| Integration | `apps/goresave/integration_test` | Future temp-save workflow tests |

Real local savegames are not fixtures. Commit only synthetic saves or maintainer-approved public-safe samples.
