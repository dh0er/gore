# G1R Build 24539464 compiler reverse map

This is the static, read-only reverse map for the embedded AngelScript compiler
that the standalone profile must reproduce. It records only evidence established
against the exact executable below. Runtime-dependent data is listed separately
and must not be guessed.

## Executable identity

- Steam app/build: `1297900` / `24539464`
- file size: `171784704`
- Steam SHA-1: `cbee276566da22293fa05638e4cdec36c5c7928d`
- SHA-256: `c71c04dd86e11e3e94483ea02c26c612b6243c147f6d83973233b3c8ddc5de25`
- preferred image base: `0x140000000`
- PDB: `G1R-Win64-Shipping.pdb`
- RSDS GUID/age: `CF0B83BD-E023-061B-2100-0F0FCCF871D2` / `1`

All addresses below are RVAs. At runtime use `loaded module base + RVA`; never
assume the preferred image base under ASLR.

## Pipeline anchors

```text
Manager construction                  0x4684800
  asSetGlobalMemoryFunctions          0x47af380
  asCreateScriptEngine(23300)         0x47af1e0
Initialize_AnyThread                  0x4685160..0x4685c17
  SetEngineProperty                   0x47a50f0
  SetMessageCallback                  0x47a5b70
    message collector                 0x4685ff0
  SetContextCallbacks                 0x47a4840
  BindDatabase.Load                   0x4685d10
  sorted indirect bind loop           0x46856e0..0x4685708
    call bind callback                0x46856fb
    final post-bind boundary           0x468570a
  optional PrecompiledData.Load       0x48dcce0
  InitialCompile                      0x4684210
  generate: InitFromActiveScript      0x48dbd50
  generate: Save                      0x48e4d90
Top-level PrecompiledData serializer  0x48bec00..0x48bece1
GetCurrentBuildIdentifier             0x48d3230
```

The message collector at `0x4685ff0` consumes the known `asSMessageInfo`
layout and is not engine creation. Its only static code xref is the callback
setup at `0x46853ec`.

## Engine properties

The setter at `0x47a50f0` has the fork's switch over property IDs 1 through 34.
The current executable performs these assignments, in this order:

| Property | Value/source | Call RVA |
| --- | --- | --- |
| `ALLOW_UNSAFE_REFERENCES` | `1` | `0x4685257` |
| `USE_CHARACTER_LITERALS` | `1` | `0x468526b` |
| `ALLOW_MULTILINE_STRINGS` | `1` | `0x468527b` |
| `SCRIPT_SCANNER` | `1` | `0x468528b` |
| `OPTIMIZE_BYTECODE` | `1` | `0x468529b` |
| `AUTO_GARBAGE_COLLECT` | `0` | `0x46852aa` |
| `ALTER_SYNTAX_NAMED_ARGS` | `1` | `0x46852ba` |
| `DISALLOW_VALUE_ASSIGN_FOR_REF_TYPE` | `1` | `0x46852ca` |
| `ALLOW_IMPLICIT_HANDLE_TYPES` | `1` | `0x46852da` |
| `REQUIRE_ENUM_SCOPE` | `1` | `0x46852ea` |
| `ALWAYS_IMPL_DEFAULT_CONSTRUCT` | `1` | `0x46852fa` |
| `PROPERTY_ACCESSOR_MODE` | `3` | `0x468530a` |
| `TYPECHECK_SWITCH_ENUMS` | `1` | `0x468531a` |
| `FLOAT_IS_FLOAT64` | `Config+0x6c != 0` | `0x4685338` |
| `ALLOW_DOUBLE_TYPE` | `Config+0x6e == 0` | `0x4685356` |
| `WARN_ON_FLOAT_CONSTANTS_FOR_DOUBLES` | `Config+0x6f != 0` | `0x4685374` |
| `WARN_INTEGER_DIVISION` | `Config+0x70 != 0` | `0x4685392` |
| `COMPILER_WARNINGS` | `1` or `2` from `Config+0x6d` | `0x46853b3` |
| `AUTOMATIC_IMPORTS` | `1` if global `0x149d6b362` | `0x46853cc` |
| `BUILD_WITHOUT_LINE_CUES` | `1` in generate mode | `0x46855dd` |

The branch-derived values are not qualified until observed in the controlled
runtime capture. Property 19 is configured by the shipping executable even
though the corresponding checked-in UNREANGEL manager block differs.

## Manager switches and data

- `as-simulate-cooked`: xref `0x468518c`, global `0x149d6b340`
- Shipping `UseEditorScripts`: global `0x149d6b341`, statically set to false
- `as-test-errors`: xref `0x46851a6`, global `0x149d6b342`
- `as-force-preprocess-editor-code`: xref `0x46851c0`, global `0x149d6b344`
- `as-generate-precompiled-data`: global `0x149d6b345`
- `as-development-mode`: global `0x149d6b361`
- manager engine pointer: `+0x28`
- manager `PrecompiledData*`: `+0x460`
- manager `StaticJIT*`: `+0x468`
- manager precompiled-use decision: `+0x470`
- manager settings object: `+0x4d0`
- `as-precompiledscript-output=` xref: `0x4685afc`

## Bind database and registration boundary

`BindDatabase.Load` reads Structs and Classes. Only generation mode appends
`.Headers`, reads the header pairs and resolves their objects. It does not load
bind modules.

The actual bind callback array is runtime-populated in zero-filled data at VA
`0x149d6b550`. Records have stride `0x50`; the manager sorts them and invokes
the callback at record `+0x10`. The callback list, Early/Normal/Late order and
post-bind mutations therefore cannot be reconstructed completely from a static
array dump or `Binds.Cache`.

## Preprocessor and class generator anchors

- `InitialCompile`: `0x4684210`, disk preprocessor construction `0x4684358`
- `ParseIntoChunks` proven code regions: `0x489a43c..0x489d691`
- `ParsePreProc`: `0x489d930..0x489dac7`
- class analysis proven region: `0x485d7b3..0x48621c0`
- delegate analysis proven region: `0x4862237..0x4863024`

Optimizer splitting means a proven code-region start is not automatically an
original C++ function entry. Hooks must use the exact instruction boundaries
validated for the current executable, not these broad region labels.

## Top-level cache serialization

The current serializer writes:

1. `DataGuid` (object `+0x28`)
2. `BuildIdentifier` (object `+0x278`)
3. `Modules` (`+0x38`)
4. `TypeReferences` (`+0x88`)
5. `TypeIdReferenceToPointer` (`+0xd8`)
6. `FunctionReferences` (`+0x128`)
7. `FunctionIdReferenceToPointer` (`+0x178`)
8. `GlobalReferences` (`+0x1c8`)
9. `StaticNames` (`+0x268`)
10. `PropertyReferences` (`+0x218`)

This matches the fork's top-level field order, including `StaticNames` before
`PropertyReferences`. It does not by itself prove nested record or relocation
parity.

For the pristine current cache:

- DataGuid bytes: `be78fe0a46ac6643968597e85c7e5b3f`
- `BuildIdentifier` at file offset `0x10`: `0x9e377abe`
- module count at `0x14`: `7308`

`0x9e377abe` is a build identifier, not a file magic. Qualification must also
capture the current return value of `GetCurrentBuildIdentifier()`.

## Mandatory runtime capture

A later explicitly authorized run must collect, without generating into the
live installation:

1. the complete property timeline at `0x47a50f0`;
2. before/after engine snapshots around every bind callback at
   `0x46856fb/0x46856fd`, plus the final snapshot at `0x468570a`;
3. all public registrations plus fork-internal flags and compile-only metadata,
   neutralizing process addresses into stable descriptors;
4. source roots, module order/import edges, preprocessor chunks and class/
   delegate generator results at the InitialCompile boundary;
5. controlled positive and negative oracle probes with bytecode/diagnostics;
6. runtime BuildIdentifier and StaticJIT state.

The generate mode may write cache files. It must not be used against the live
installation. Writer captures require a verified installation copy or a proven
redirected output path and separate authority.
