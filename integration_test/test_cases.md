# goresave Test Cases

## P0

- **TC-CORE-0001**: GSAV container parsing reports public payload, compressed stream, trailer, and SHA-1.
- **TC-CORE-0002**: GSAV rebuild is byte-identical when the compressed private stream is preserved.
- **TC-CORE-0003**: Corrupt/truncated saves are rejected with a parse error.
- **TC-CORE-0004**: Safe write creates a backup and validates the output before replacing a save.
- **TC-UI-0001**: Editor shell renders save list, overview, metadata editor, advanced inspector, backups, and settings.
- **TC-UI-0002**: Missing native core is surfaced as a user-visible status.
- **TC-CODEC-0001**: Missing codec reports private edits unavailable.

## Conditional

- **TC-CODEC-1001**: If a legal compressor is configured, private edit fixtures roundtrip through compress, rebuild, decompress, parse.
