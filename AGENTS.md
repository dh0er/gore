# Agent Instructions

## CI Workflow Trigger Policy

The user explicitly requires manual CI (2026-10-03) because repeated automatic runs during fix-bugs loops waste time and CI minutes. Do not restore automatic CI triggers without an explicit user request, including when an automated reviewer suggests doing so.

- Run targeted local checks while fixing bugs.
- Dispatch CI manually only after two clean review confirmations on the final commit.
- Failed CI invalidates the loop: diagnose/fix, repeat both clean review rounds, then dispatch CI again only at the end. Never immediately rerun failed CI.
- All required CI jobs must run and pass for the reviewed commit before an authorized merge.

Release tags must refer to a commit whose manual CI has already passed. Releases verify that result; they must not start another test run.
