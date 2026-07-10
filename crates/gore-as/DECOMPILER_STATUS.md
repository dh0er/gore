# AngelScript decompiler — completeness & known gaps

**Status: NOT complete.** The decompiler recovers the overwhelming majority of function
bodies, but a small tail cannot be reconstructed correctly and is emitted as
signature-preserving **stubs**. This document records exactly what is missing and why, so the
gaps are not mistaken for finished work.

## Headline numbers

Measured on the full shipped cache (`PrecompiledScript_Shipping.Cache`, 7264 modules,
~162,831 functions), with the Binds.Cache native API loaded:

| Metric | Value |
|--------|-------|
| Functions with a fully recovered body | ~160,578 (**98.62%**) |
| Functions emitted as a stub | ~2,253 (**1.38%**) |
| — of which proactive (category A, no loop) | ~1,046 |
| — of which force-stubbed compile failures (category B) | ~1,207 |
| Modules containing ≥1 stub | 620 / 7265 |
| In-game compile (all modules) | **100%** — the emitted tree compiles |

The 100% compile rate is *not* the same as 100% recovery: it is achieved by stubbing the
functions the decompiler cannot render correctly (see *force-stub loop* below). Every stub is
a real gap. The category-B count is inflated by the force-stub mechanism keying on
`Class::method` *names*, so stubbing one failing overload also stubs its (possibly fine)
same-named siblings — the true unrecoverable count is somewhat lower.

## What a stub is

Only the **body** is replaced; the declaration is byte-correct, so the module still compiles
and every other function around it is real source:

```angelscript
bool DoesEntryApplyToCurrentSituation_Implementation()
{
    // body not fully recovered — stub [<reason>]
    bool __r; return __r;
}
```

A stub preserves the signature (name, parameter types, return type, `UFUNCTION()`/`UPROPERTY()`
markers, `const`) but **loses the original logic**. The raw bytecode for a stubbed function is
still inspectable via `gore as disasm <needle>`.

## Two categories of gap

### A. The decompiler genuinely cannot render the body (~1,046 functions, "raw" stubs)

These stub *regardless* of the compiler — the structured emitter detects it cannot produce a
correct body and bails out proactively. Reason codes appear in the stub comment; regenerate
the breakdown with:

```
GORE_AS_BINDS=.../Binds.Cache gore as emit-all <cache> <out>   # no GORE_AS_STUBLIST
grep -rhoE 'stub \[[^]]*\]' <out> | sort | uniq -c | sort -rn
```

| Reason | Count | Why |
|--------|-------|-----|
| `argmismatch:argint` | ~492 | A call argument the decompiler recovered as a plain integer where the callee wants a different scalar/enum/handle — the operand's real type isn't pinned by any side table. |
| `argmismatch:argtype` | ~376 | A call argument whose recovered struct/object type can't match the callee parameter — same root cause (no slot-type inference). |
| `opcode-uncovered` | ~164 | The function uses an asBC opcode the stack machine in `cache/structure.rs` does not yet model (and the conservative fixes now bail here rather than silently dropping it). |
| `argmismatch:copyctor` | ~12 | A compiler-generated struct copy-constructor / `opAssign` on `this` — has no hand-written source form to recover. |
| `unresolved-operand` | ~2 | A comparison/operand left a `?` placeholder: the value tested was produced by an op whose result the decompiler couldn't track. |

### B. The body decompiles but does not COMPILE (~1,207 functions, force-stubbed)

The structured emitter produces a body, but the in-game AngelScript compiler rejects it, so the
**force-stub loop** routes it to a clean stub (emit → headless compile → collect the failing
`Compiling Class::method` lines → `GORE_AS_STUBLIST` → re-emit → repeat until the compiler
reports zero diagnostics). The dominant rejection classes:

| Class | Why it's hard |
|-------|---------------|
| **Arg-type mismatch** (`int` → `FGameplayTag` / `FRememberedPerception`, `EPerceptionCharacterType` → `int`, …) | The decompiler recovers arg *values* and *count* but not always the exact *type*. Bytecode is type-erased at the operand level; reconstructing the precise struct/enum type of a temporary needs full slot-type inference that isn't implemented. |
| **GAS gameplay-tag / delegate patterns** | These rely on the `__STATIC_NAME(idx)` accessor and delegate-handle plumbing, where the recovered statement boundaries break the dataflow (the resolved `n"Tag"` value isn't threaded into the call that consumes it). The tag/delegate argument ends up dropped or misplaced. |
| **`'X' is not a member of 'int'`** | A local the decompiler couldn't type is hoisted as the default `int`, but the body uses it as a struct/object. Correct typing needs the same slot-type inference. |
| **`No default constructor`** | A recovered default-return / default-local of a value type that has no parameterless constructor. |
| **`Illegal operation`, `loses precision`, `Result of expression unused`** | Residual mis-modelled arithmetic / cast / discarded-value cases. |

Key property: these functions almost always fail for **several** of the above reasons at once,
so fixing any single class does not reduce the stub count much — the function only flips to
"compiles" when *every* defect in it is fixed. That is why the remaining tail is stubborn.

## Why the gaps exist (root causes)

1. **Type erasure in bytecode.** asBC addresses operands by stack slot, not by type. Names and
   high-level types are reconstructed from side tables (FunctionReferences params/returns,
   PropertyReferences, the class hierarchy, Binds.Cache native signatures). Where a temporary's
   type isn't pinned by any of those, the decompiler guesses (`int` default) — wrong guesses
   become compile errors. **Full slot-type inference is the single biggest missing piece.**
2. **Native signatures only carry arity, not always exact param types at the call.** Binds.Cache
   gives `(class, name) → arity` (used to trim phantom args), but the per-argument type
   reconstruction at a call site still depends on operand tracking.
3. **Statement-boundary loss in compiler-generated idioms.** GAS tag/delegate registration and
   struct copy/destruct behaviours don't map 1:1 to source statements; the stack-machine
   reconstruction splits them in ways that lose the original dataflow.
4. **Unmodelled opcodes.** A handful of asBC ops aren't yet handled (category A above).

## Path to higher completeness (not yet done)

- **Slot-type inference pass**: propagate types across the operand stack (from known producers
  — call returns, field loads, casts — to consumers) so temporaries get their real struct/enum
  type instead of the `int` default. This would address arg-type mismatches and the
  `not-a-member-of-int` class — the largest buckets.
- **Model the remaining asBC opcodes** to clear category A `opcode-uncovered`.
- **GAS idiom recognition**: special-case the tag-event / delegate-handle registration patterns
  to thread the `n"Tag"` value into its consuming call.

Until then: per-module editing of any **non-stub** function (99.4% of functions) round-trips
cleanly; editing a **stub** function means rewriting its body by hand (signature is provided,
raw bytecode available via `disasm`). See `work/reversing/gore-as/` (local scratch) for the
headless compile harness and the current force-stub list.
