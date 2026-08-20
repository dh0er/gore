# AngelScript decompiler — completeness and known gaps

**Status: complete body coverage for the measured 2026-07-12 build 24169431 baseline, but not
lossless.** The current emitter reconstructs every ordinary function body it writes from the
shipped cache. When it cannot prove that a body is correct, it still keeps the declaration and
emits a clearly marked, signature-preserving stub instead of silently inventing logic; the measured
24169431 cache has no such fallback body. This retained historical measurement is not a current-build
qualification.

## Retained measured baseline

Measured on 2026-07-12 against build 24169431, the then-current hotfix
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
semantic `bytediff` oracle and game compiler. The compiler-generated `__InitDefaults` method is
no longer omitted: its statements are written back as the class-scope `default` statements they
were compiled from (see "Class defaults" below), so an item, NPC or config class decompiles with
its data rather than as an empty shell. Existing-module edits still carry the proven base
records, compiler wrappers, behavior functions, and full method tables byte-for-byte through the
strict base-keyspace remap path, and an overlay that authors defaults is refused there — see
"Class defaults" for why. Separately, the offline `default-sites` / `patch-default` path
can change a uniquely proven, branch-free direct scalar assignment using a semantic selector and
raw compare-and-swap guard. It cannot reconstruct or edit arbitrary class-default data. A fresh
whole-tree game-compiler run reached
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

None in the measured 24169431 corpus. This is retained historical evidence, not a measurement of a
newer current build and not a promise that arbitrary future bytecode shapes will recover: every
unsupported or unproved construct still takes the visible signature-preserving stub path.

These are proactive stubs from the structured emitter. The old force-stub workflow and its
thousands of name-keyed compile-failure stubs are no longer part of this baseline.

The generalized `Thiscall1` stack-frame fix removed 17 fallback bodies in the measured 24169431
baseline. It uses
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

## Class defaults

Measured on 2026-08-20 against build `Build55_CL171864` (script cache SHA-256
`D0AFAF909E62867FAEDC3678A1175F5E8DE5E784DC503A14FFBDE4726F297231`, GUID
`be78fe0a46ac6643968597e85c7e5b3f`) with the matching `Binds.Cache` loaded. This build is not one
of the audited generations, so these numbers qualify the decompiler, not the build.

| Metric | Value |
|--------|-------|
| Modules authoring their class defaults | 6,885 of 7,308 that have any |
| Modules suppressed (recovery incomplete) | 32 |
| `default` statements written | 206,146 |
| Vanilla `__InitDefaults` methods | 30,005 |
| Aligned after recompile | 29,033 (972 unaligned, from the suppressed modules) |
| Byte-faithful (`IDENTICAL`+`BENIGN`, `--norm-slots`) | 28,999 (**99.88%**) |

The whole emitted tree recompiles with no errors, and `gore as bytediff --norm-slots` reports 0
module-level alignment loss, no only-in-regen functions, and B1 **90.20%** over all 163,632
aligned functions — up from 88.78% before the body work below, with 16,035 semantic differences
left against 18,288.

Editing an existing module's defaults and splicing it back works. Getting there needed five
identity fixes, because a decompiled module is only re-splicable when every symbol it references
still composes to the identity the base cache recorded:

- **StaticNames.** `STR` and the `PshC4` before `__STATIC_NAME` carry an index into the T6 name
  pool, and the strict remap left them alone — a regen assigns its own pool, so every `FName` in
  a recompiled module silently denoted a different name. A sword's mesh came back as a scroll's.
  They are now remapped by text, and a name the base lacks fails closed.
- **Namespaces.** The emitter dropped them, so `UQuest_NewCamp` — declared in `G1R::Quest` —
  recompiled as a global-scope class that matched nothing. 1,503 modules are affected, including
  every quest, document and conversation. Declarations now reopen their namespace and references
  are qualified.
- **`const` methods.** The qualifier is part of a method's identity. It used to be re-emitted
  only for ~20 allowlisted methods because a blanket restore once cost 636 compile errors; on the
  current tree all 6,247 restore with a single family failing, which a body check now covers.
- **Class references.** `PshGPtr __StaticType_X` is the bare class name; rendering it as
  `X::StaticClass()` made the compiler generate `StaticClass` functions the base never had.
- **Parameter defaults.** They are recorded in the cache and were skipped; declarations carry
  them again and calls omit arguments that only restate them.

The collision-rename workaround disappeared with the namespaces: the emitter no longer invents
`_g1234`-suffixed symbols that the base cache cannot know.

Identity alone was not enough: a module also has to be re-rendered in a SHAPE whose recompilation
references no symbol the base cache lacks. Six body-fidelity fixes closed that gap.

- **A named value temporary.** `T t = f(); Use(t);` asks for a copy the base has no `$beh0` /
  `opAssign` row for. The compiler never named it, so the emitter folds the producer into its
  consumer — the transform commits only when every reference to the slot disappears, and a store
  that carries a declaration-site conversion (`FText x = "id";`) is left alone.
- **The range-for.** `Iterator()` / `CanProceed` / `Proceed()` is what `for (auto X : c)`
  desugars to. Writing it back as a while-loop has to NAME the iterator, and a named iterator is
  copy-constructed. The idiom is folded back into the range-for the source wrote — unless the
  body writes through the element, which only the while-shape allows.
- **The container copy.** The structurer materialized the iterated member into its own slot;
  the loop now iterates the member, provided the body never touches that path.
- **A re-used slot.** One VM slot carries two source temporaries; the second assignment stayed a
  bare `local_N = …`. Each definition now gets its own declaration, the later ones under a fresh
  name, and only while every reference stays inside that definition's block.
- **A default-constructed temporary.** `PSF t; CALLSYS $beh0()` has no source form, so nothing
  declared `t` and every read of it dangled. The value is written where it is read — restricted
  to a whole-value assignment, the one position where a temporary is legal.
- **`Super::`.** An override calling the method it overrides was rendered `this.Method(...)`,
  which recompiles into infinite recursion and a function identity the base cache does not have.
  A same-arity override of an ancestor's method now renders `Super::`.

Which shape a value local takes is decided by the cache's own function table, not by a rule of
thumb: a type that has a copy constructor is declared with its initializer, a type that has a
default constructor and an `opAssign` keeps the hoisted declaration and its assignment. Both
shapes compile; only the one the base cache has a row for can be spliced back.

Measured over a 627-module random sample, `extract-remap` against the base cache succeeds for 625
(**99.7%**). The same measurement scored 43 of 60 before the identity work and 58 of 60 after it.
Both remaining failures are the one `const` the emitter deliberately drops: a return type's
`const` is stripped because the cache sets that flag inconsistently across an override family
(restoring it costs 41 "Can't implicitly convert from 'const X' to 'X'" errors, because the
locals that receive those values are typed without it), and a local typed from such a return
loses it too — which picks the non-const `Iterator()` overload and, with it, a template
instantiation the base cache does not have. `GORE_AS_REMAP_DIAG=1` prints the two identities
behind any unresolved or ambiguous reference.

Recovery is all-or-nothing per module, because `generated_defaults` can only carry an omitted
`__InitDefaults` byte-exact for a module that authors no defaults at all. A module that recovered
only some of its classes would silently drop the rest, so one unrecovered class suppresses the
whole module and its header records the class and the reason. The 32 suppressed modules break
down as: 25 whose bodies keep a compiler temporary that cannot fold, 2 machine-generated
world/voice tables over the size bound, and 5 individually distinct shapes.

The 25 unfoldable ones are fail-closed for a reason worth keeping: they are the bodies where a
`double` member read is rendered through an `int()` cast, so authoring them would round a cost
or a multiplier to a whole number. The numeric-kind inference has to be fixed before their
defaults are worth writing.

Every recovered statement is also checked against the cache's own function table: a rendered
`Name()` whose function the cache knows only WITH parameters means an argument was lost, and the
module is suppressed rather than written. That check found the fluent AI rule builders
(`Rules.Add(t).RequireTrue(a).RequireFalse(b).Then(r)`), where the structurer split the chain
after the first link and the next link took a leftover argument as its receiver. That split is
fixed — a temporary's destructor between two links no longer ends the statement — and the check
stays as the general guard against any future dropped-argument shape.

The 34 initializers that still differ after a faithful recompile are dominated by float constants
the emitter cannot spell: AngelScript has no infinity literal, so `+inf` (`0x7F800000`) is written
as the largest finite float and comes back one ULP low.

## Root causes and next work

1. **Generated defaults are retained; direct scalars have a narrow offline patch path.**
   `compile-module --op edit` carries
   existing `__InitDefaults` plus every emitter-omitted executable record only after exact
   header/tail/reference, declaration/layout, method-table, and cache-wide collision proofs. An
   authored CDO `default` token, new-symbol remap, unsupported `__*` shape, or any metadata drift
   fails closed before publishing a mini-cache. `gore as default-sites` and `patch-default` can
   inspect and copy-on-write patch only a unique, branch-free
   `SetV{1,2,4,8} / LoadThisR / WRTV{1,2,4,8}` scalar assignment with exact field-type evidence
   (including parsed-kind proof for script enums),
   a v4 `(module, class, field_owner, field, value_type, ancestry_profile)` identity with proven target-to-owner ancestry, raw
   CAS, one terminal `RET`, and full-cache postconditions. Complex expressions, structs, and
   containers remain unsupported by that scalar workflow. Separately, `gore as tag-map-sites` and
   `patch-tag-map` can inspect and copy-on-write patch only an already-present entry in the sealed
   native `GameplayTag`-to-`float32` map shape; they cannot add a key or map, resize bytecode, or
   author arbitrary map defaults. See
   [`docs/guide/angelscript-defaults.md`](../../docs/guide/angelscript-defaults.md). A faithful
   source representation is still required before arbitrary class-default authoring can be
   claimed; new modules may continue to use explicit defaults through `--op add`.
2. **Whole-tree compiler gate -- passed for the then-current installed 1.0.3 hotfix.** The shipping
   build suppresses AngelScript diagnostics from
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
   preserved the complete loose-source and JIT trees byte-for-byte. Archived 1.0.0 through 1.0.5
   executables pass the same offline structural check. Runtime injection is also proven on the
   installed 1.0.5 / Steam BuildID 24878692 executable: the 27-case embedded qualification captured
   native file/line/column diagnostics, and the frozen whole-tree failure surfaced through the same
   callback boundary without accepting an output cache.

   The compiler library now also exposes a bounded structured report instead of forcing callers
   to recover diagnostics from formatted error strings. It retains file, line, column, severity,
   and message together with one of `Captured`, `CaptureInvalid`, `UnavailableFallback`,
   `UnavailableWithoutFallback`, `ProcessExitUnconfirmed`, or `Disabled`. True signature, hook, or
   preflight unavailability runs the normal generator exactly once. If the first generator has
   already exited before capture becomes available, `UnavailableWithoutFallback` keeps that first
   result and never starts a second process. Invalid, truncated, oversized, or unrepresentable
   capture becomes `CaptureInvalid` and rejects an otherwise usable cache; it is not treated as
   clean hook unavailability. An unconfirmed compiler-process exit exposes no possibly
   live-written diagnostics, preserves recovery artifacts, and never starts a fallback. Raw and
   formatted capture are capped at 8 MiB; the structured envelope permits at most 65,536 records,
   32 KiB per filename, 64 KiB per message, and 16 MiB retained diagnostic text. This is a native
   API foundation. The managed exact-current Quest and NPC compiler checks now consume this
   structured report while compiling their derived managed source. That integration does not widen
   executable-generation admission, callback-runtime qualification, or the managed source scope;
   unsupported diagnostics continue through the normal fallback contract.

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
