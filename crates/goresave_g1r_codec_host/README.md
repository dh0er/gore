# G1R Binary Codec Host

This crate provides the out-of-process helper used by `goresave` for private
save payload operations. It communicates over newline-delimited JSON on stdio
and is designed to fail closed when the configured runtime cannot be verified.

## Scope

- Runs separately from the Flutter app and Rust core.
- Does not patch files, inject into a running game, or require the game process.
- Maps the configured game executable in an isolated worker process.
- Applies timeouts and process cleanup around worker calls.
- Keeps private payload writes disabled unless verification succeeds.

## Commands

- `probe`: check whether a configured game executable can be used.
- `self_test`: verify runtime behavior and optional sample roundtrips.
- `decompress`: decode one payload chunk and return `outputBase64`.
- `compress`: encode one payload chunk and return `outputBase64`.
- `export_derived_profile`: export a cached profile by `exeSha256` or `exePath`.

## Cache

Resolved profiles are cached by executable SHA-256 with file size, PE timestamp,
image base, resolved RVAs, confidence, matched anchors, and self-test status.

Default cache path:

```text
%LOCALAPPDATA%\goresave\g1r_codec_host_derived_profiles.json
```

IPC commands that resolve an executable accept `derivedProfileCachePath` to
override the cache location for tests.
