# AngelScript decompiler — completeness and known gaps

**Status: nearly complete, but not lossless.** The current emitter reconstructs almost every
ordinary function body in the shipped cache. When it cannot prove that a body is correct, it
keeps the declaration and emits a clearly marked, signature-preserving stub instead of silently
inventing logic.

## Current measured baseline

Measured on 2026-07-11 against the current hotfix
`PrecompiledScript_Shipping.Cache` (SHA-256
`1018F1CFE6B99A650EECB33AFB96752D691D2088EAD27808971B812F04ECB4C2`), with the matching
`Binds.Cache` loaded and **without** `GORE_AS_STUBLIST`:

| Metric | Value |
|--------|-------|
| Emitted modules | 7,305 |
| Raw cache function records | 156,251 |
| Emitted body-bearing functions | 55,403 |
| Bodies emitted without a fallback stub | 55,402 (**99.99820%**) |
| Signature-preserving stubs | 1 (**0.00180%**) |
| Modules containing at least one stub | 1 |

The counts describe functions that `emit-all` emits. A non-stub body is one the structurer could
render; this percentage is **not** a semantic byte-faithfulness score. Deep argument/dataflow
mistakes can exist in otherwise complete-looking source and are measured separately by the
semantic `bytediff` oracle and game compiler. Compiler-generated special functions such
as `__InitDefaults` are intentionally omitted today, so these numbers must not be read as proof
that every piece of class-default data round-trips. A fresh whole-tree game-compiler run reached
the real generator and the diagnostics callback hook captured concrete file/line/column errors
before the compiler exited without publishing a development cache. Those diagnostics exposed
three generic emitter residues, which are fixed in the current tree. A final controlled compile of
the corrected 7,305-module tree then completed successfully with the hardened helper and produced
a structurally complete 91,321,157-byte development cache. A separate intentional unknown-symbol
compile proved normal `file:line:column: error` output and correctly accepted no cache. Both
transactions restored every installed source, JIT artifact, proxy, and shipping cache byte-for-byte.
The percentages still measure decompiler body coverage, not semantic byte identity.

Reproduce the measurement with:

```text
GORE_AS_BINDS=.../Binds.Cache gore as emit-all <cache> <out>
rg -o 'stub \[[^]]+\]' <out>
```

`emit-all` now distinguishes raw cache function records from functions for which it actually
writes an editable body. It also prints exact stubbed module/function totals, so no filename-based
estimate is needed.

## Remaining stubs

| Reason | Count | Current cause |
|--------|-------|---------------|
| `opcode-uncovered` | 1 | `UCBT_CompleteSequence::Tick` combines a compound loop header, switch, and backward `continue`; that ownership shape is not yet reconstructed conservatively. |

These are proactive stubs from the structured emitter. The old force-stub workflow and its
thousands of name-keyed compile-failure stubs are no longer part of this baseline.

The generalized `Thiscall1` stack-frame fix removed 17 fallback bodies in the current hotfix. It uses
the opcode's physical stack arity independently from rendered argument arity, preserving deferred
outer `FName`/object arguments while consuming compiler-injected inner defaults. This cleared all
repeated `SetupTransitions` delegate cases and also corrected deep arguments in already
non-stubbed bodies; no function-name-specific rewrite is involved. The latter is why stub counts
alone must never be treated as a semantic-completeness proof.

All formerly residual operand/type stubs are now recovered generically. Owner-safe native bind
arity inference recovered the integer/static-name cases; a strictly typed PSF copy-constructor
proof recovered the remaining copy patterns; and native struct-field enum metadata now wins over
the enclosing handle-owner fallback for `LoadRObjR`/`LoadVObjR -> PshRPtr`. Positive real-cache
tests and negative bytecode mutations cover each proof. There are no function-name-specific
exceptions.

## What a stub means

Only the body is replaced. The class/function declaration, parameters, return type and relevant
annotations remain available, while the original bytecode can still be inspected with
`gore as disasm`:

```angelscript
bool DoesEntryApplyToCurrentSituation_Implementation()
{
    // body not fully recovered — stub [argmismatch:argtype]
    return false;
}
```

A stub is therefore safe and visible, but it is not a faithful implementation. Editing one
requires reconstructing its body manually or first extending the decompiler.

## Root causes and next work

1. **One compound `JMPP`/loop shape.** The switch recognizer handles normal tables, but
   `UCBT_CompleteSequence::Tick` still takes the conservative `opcode-uncovered` exit because its
   loop header, switch and backward `continue` ownership are not yet jointly proven.
2. **Generated defaults.** `__InitDefaults` and related generated methods contain important NPC,
   quest and class-default data and need a separate faithful representation before full asset
   authoring can be claimed.
3. **Whole-tree compiler gate -- passed for the current 1.0.3 hotfix.** The shipping build suppresses AngelScript diagnostics from
   stdout and UE file logs, so `gore as compile` now uses a hotfix-safe signature scan plus a sparse
   callback-body fingerprint to attach to the per-error `asSMessageInfo` callback. It prints normal
   file/line/column diagnostics only when the raw signature is unique and all five message-field
   offsets verify, and automatically falls back to the unhooked compiler when either gate is absent
   or injection cannot be confirmed. Controlled runs with the predecessor capture helper exposed
   concrete generic emitter residues, compiled the corrected whole tree successfully, and surfaced
   the expected normalized error for an intentional failure. The exact shipped
   structure-hardened helper (`17E0AD3033C31ADD311E3C25BA63615E481C83DCF8E96E83D9B3AC088E55C01C`)
   has now repeated both gates on installed 1.0.3: the corrected whole tree compiled to a
   structurally complete 91,321,157-byte cache, while an intentional unknown-symbol compile
   returned the normal `file:line:column: error` diagnostic and accepted no output. Both runs
   preserved the complete loose-source and JIT trees byte-for-byte. Archived 1.0.0 through 1.0.3
   executables pass the same offline structural check; runtime injection remains proven only on
   the installed 1.0.3 executable.

The mixed-RVO switch in `MakeNewCrimeRegisterData` is now recovered with a per-exit proof: each
early bare-RET edge must contain exactly one resolved RVO store, and removing that store in the
negative regression atomically restores the stub. The remaining `UCBT_CompleteSequence::Tick`
case needs compound-loop and backward-`continue` structuring. The corrected final emission now
passes the whole-tree game compiler; use the remaining stub/generated-default limitations and the
semantic `bytediff` oracle, rather than compileability alone, when deciding whether a broad edit is
faithful.
