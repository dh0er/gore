# AngelScript decompiler — completeness and known gaps

**Status: complete body coverage for the current hotfix, but not lossless.** The current emitter
reconstructs every ordinary function body it writes from the shipped cache. When it cannot prove
that a body is correct, it still keeps the declaration and emits a clearly marked,
signature-preserving stub instead of silently inventing logic; the measured current cache has no
such fallback body.

## Current measured baseline

Measured on 2026-07-12 against the current hotfix
`PrecompiledScript_Shipping.Cache` (SHA-256
`1018F1CFE6B99A650EECB33AFB96752D691D2088EAD27808971B812F04ECB4C2`), with the matching
`Binds.Cache` loaded and **without** `GORE_AS_STUBLIST`:

| Metric | Value |
|--------|-------|
| Emitted modules | 7,305 |
| Raw cache function records | 156,251 |
| Emitted body-bearing functions | 55,403 |
| Bodies emitted without a fallback stub | 55,403 (**100%**) |
| Signature-preserving stubs | 0 (**0%**) |
| Modules containing at least one stub | 0 |

The counts describe functions that `emit-all` emits. A non-stub body is one the structurer could
render; this percentage is **not** a semantic byte-faithfulness score. Deep argument/dataflow
mistakes can exist in otherwise complete-looking source and are measured separately by the
semantic `bytediff` oracle and game compiler. Compiler-generated special functions such
as `__InitDefaults` are intentionally omitted from editable source. Existing-module edits can now
carry the proven base records, compiler wrappers, behavior functions, and full method tables
byte-for-byte through the strict base-keyspace remap path, but authored changes to those defaults
are deliberately refused. The numbers therefore must not be read as proof that arbitrary
class-default data can be reconstructed or edited. A fresh whole-tree game-compiler run reached
the real generator and the diagnostics callback hook captured concrete file/line/column errors
before the compiler exited without publishing a development cache. Those diagnostics exposed
three generic emitter residues, which are fixed in the current tree. A final controlled compile of
the 7,305-module, zero-stub tree completed successfully with the hardened helper and produced a
structurally complete 91,181,145-byte development cache (SHA-256
`FD868A0B46E71E93552F774435940FB9146216156C9E2160DE80C9FBCBED0EC1`). A separate intentional unknown-symbol
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

None in the measured current-hotfix corpus. This is a corpus result, not a promise that arbitrary
future bytecode shapes will recover: every unsupported or unproved construct still takes the
visible signature-preserving stub path.

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

1. **Generated defaults are safe to retain, not yet editable.** `compile-module --op edit` carries
   existing `__InitDefaults` plus every emitter-omitted executable record only after exact
   header/tail/reference, declaration/layout, method-table, and cache-wide collision proofs. An
   authored CDO `default` token, new-symbol remap, unsupported `__*` shape, or any metadata drift
   fails closed before publishing a mini-cache. A separate faithful source representation is still
   required before NPC, quest, or arbitrary class-default authoring can be claimed; new modules may
   continue to use explicit defaults through `--op add`.
2. **Whole-tree compiler gate -- passed for the current 1.0.3 hotfix.** The shipping build suppresses AngelScript diagnostics from
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

The mixed-RVO switch in `MakeNewCrimeRegisterData` is recovered with a per-exit proof: each early
bare-RET edge must contain exactly one resolved RVO store, and removing that store in the negative
regression atomically restores the stub. `UCBT_CompleteSequence::Tick` is now recovered by a
bounded symbolic header-DAG proof plus single-entry, dominance, unique-backedge and fully
structured-body gates. Its switch accepts only the exact backward loop-continue target as an early
exit, stops at the proven loop exit, and leaves the physically later default and outer return tail
under their correct owners. Elided header temporaries additionally require path-local definitions
and a whole-body/exit no-read-before-overwrite proof. Numeric constants preserve IEEE and unsigned
high-bit values; copy and cast chains retain destination signedness; enum-byte constants with
unknown underlying signedness fail closed; and full-register jumps require a proven canonical
boolean rather than a one-byte register copy. Synthetic negative regressions cover those type and
liveness gates plus a second/wrong backedge, side-effecting or non-boolean header, outside entries,
an enclosing-loop break target and ambiguous joins; every deviation atomically restores the
`JMPP` stub path. The final zero-stub emission passes the whole-tree game compiler. A targeted
semantic-oracle gate for `UCBT_CompleteSequence::Tick` aligns all 104 original operations and
classifies the remaining differences as benign N1/N2/N4/N5 build/allocation noise with zero
semantic differences. That is a qualification of the formerly stubbed function, not a claim that
the entire emitted corpus is byte-identical. Use the generated-default limitations and the semantic
oracle, rather than compileability or zero stubs alone, when deciding whether a broad edit is
faithful.
