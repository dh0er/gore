# AngelScript decompiler — completeness and known gaps

**Status: every module decompiles, the whole tree recompiles, and 99.32% of it is byte-faithful.**
The emitter reconstructs every function body it writes from the shipped cache; when it cannot
prove a body is correct it keeps the declaration and emits a clearly marked, signature-preserving
stub instead of inventing logic. The current corpus needs no such stub. What is NOT proven is that
every recompiled body is identical to vanilla — the section below says exactly how much is, how it
was measured, and what is left.

## What is measured, and on what

Measured 2026-08-29 against the shipped build whose script cache has SHA-256
`7A18F954E32AF30FC24AE3A66EA35D3B5CB98560C8F5083C7846FC9CE1D77511` (GUID
`7835bcc09c5eee488d72cb5ffb0fb0c3`). That is the audited generation `g1r-steam-24878692`, the
Steam build shipped 2026-08-27/28. Counts from the earlier `D0AFAF90…` build are not
comparable: that one had 7,308 modules and 164,607 aligned functions.

Everything except the splice test was measured on this build, over the **whole corpus** — all
7,317 modules, all 164,723 functions the vanilla and regenerated caches align:

| Measurement | Scope | Result |
|-------------|-------|--------|
| Modules emitted, fallback stubs | full corpus | 7,317 modules, **0 stubs** |
| Whole-tree recompile warnings | full corpus | **0** (the compiler treats them as errors) |
| Class defaults authored | full corpus | **0 modules suppressed** (all 30,005 `__InitDefaults`) |
| Whole-tree recompile (`as compile`) | full corpus | **0 errors** |
| Byte-faithfulness (`bytediff --norm-slots`) | full corpus, 164,723 functions | **99.32%** (`IDENTICAL`+`BENIGN`) |
| Alignment loss | full corpus | **none** — every function the cache has is regenerated |
| Splice back (`extract-remap`) | full corpus, earlier `D0AFAF90…` build | 7,278 of 7,308 (**99.59%**) |

Every measurement covers the whole corpus. The splice sweep takes about two hours (each run
re-reads both 100+ MB caches), which is why earlier revisions of this document reported it from a
627-module sample; the sample and the sweep agreed to within 0.1 points. That sweep has not been
repeated on this build — its numbers are the earlier build's, which is why the row says so.

The measurement needs the matching game's `Binds.Cache` next to the script cache it reads. An
older run without it let native enum fields fall back to the bool heuristic and then stopped with
1,474 `bool` to `E*&` errors. Current emission fails closed earlier: if a generated initializer's
native scalar target cannot be typed, it suppresses authored defaults for that whole module so a
later edit can use byte-exact carry instead of compiling a partial or mistyped default set.

## Default-target store coverage (2026-09-06)

On the same `7A18F954…` Shipping cache, the preservation guard now classifies the
three formerly unsupported store opcodes that actually occur in class initializers:

| Opcode | Occurrences | Initializers | Proof |
|---|---:|---:|---|
| `STOREOBJ` | 49,810 | 11,406 | Direct pointer write entirely inside `VariableSpace` |
| `REFCPY` | 27 | 19 | Stack destination is a resolved `this` member chain; count its root |
| `CpyRtoV8` | 1 | 1 | Direct eight-byte write entirely inside `VariableSpace` |
| `CpyVtoV4`, `CpyVtoV8`, `CpyRtoV4`, `CpyGtoV4` | 0 | 0 | Still refused |

The disassembly census covers all 30,013 `__InitDefaults`. `STOREOBJ` has 119
opcode-context patterns (two instructions on either side, truncated at function
boundaries), with destination slots 2 through 1122. `REFCPY` has 26 direct-member
stores and one embedded-member chain; the one `CpyRtoV8` captures a call result
in `UCBT_Tree_StayAway`. These counts classify stores, not complete module buildability.

The VM evidence is the vendored `as_context.cpp`: `STOREOBJ` writes the object
register directly to `frame - signed_slot`; `CpyRtoV8` independently writes the
value register there. Both require `2 <= slot <= VariableSpace` on Win64, avoiding
`this` overlap at slot 1. `REFCPY` instead writes through a stack address. Local
aliases, register-only destinations and jump entry into its apparent member
chain remain unproven. Source and regenerated-cache target multiplicities are
still checked separately.

Reproduce the cache proof with `GORE_AS_CACHE` set to the Shipping cache:

```powershell
cargo test -p gore-as --lib real_shipped_store_population_is_classified -- --ignored --nocapture
```

The four edited character modules (Diego, Viper, Bandit, Fingers) compiled through
strict standalone in 38–50 seconds each. The deployed Diego fixture was played
in a new game by the user: the resulting save records both base/current Health
and MaxHealth as 1234 under `OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN`. This is a
runtime-to-save observation of the health edit, not a runtime census of all defaults.

## What is left

**1,114 functions (0.68%) recompile to bytecode that differs semantically.** A semantic
difference means *not proven identical*, not *proven wrong*: the whole-tree compile proves the
source type-checks, and `bytediff` normalizes away reference keys, jump absolutes, constant
encodings and (opt-in) slot allocation before judging the rest.

### Known wrong programs, found and not yet fixed

A semantic difference is normally *not proven identical*, not *proven wrong*. These three are
proven wrong, and they are recorded here rather than in a bug tracker because the same measurement
found them:

* **Four loops the game leaves and our source does not.** The `break` is lost and the arm renders
  as `if (…) { } else { }`, so the loop runs forever:
  `UAIState_WaitInQueue::DoTask_Implementation`, `UAIState_WarnAggressor::DoTask_Implementation`,
  and `UAIState_CombatEndActions_Human::Heal` twice. The mechanism is now known — the top-test arm
  opened no loop scope, which is why `loop_exit_stmt` returned `None` there. That scope is set
  now, but only for `continue;`: offering `break;` on the same path fires on six functions this
  build reproduces byte for byte, so these four still need a witness that separates them.
* **Nine float constants written as their own bit pattern.**
  `ULoadingScreenSetupTest::SetupGeneralLoadingScreen` emits
  `…SpecifiedColor.R = 1065287680;`, which is `0x3F7F0000` — the bits of `0.99609375f`. The
  compiler then converts that integer, so the colour comes out a billion times too bright. Neither
  field-type channel resolves a deep NATIVE struct path, and the float rescue in `structure.rs`
  only runs on a store that has already dropped.
* **One `event` parameter rendered `int` where the thunk says `int32`**
  (`Story.Support.DialogImport.FStoryChapterChangedEvent::Broadcast`).

The class table below is the last full classification, taken when the total stood at 3,511; its
rows account for 2,926 of those functions. It says which shapes the work was aimed at, not what
the remaining 1,114 are made of.

Classified over WHOLE functions — every instruction of both sides, not the window around the
first divergence. An earlier revision of this document classified the window instead and reported
order as the largest class at 4,861; that number was an artifact of the window. The
whole-function figure is the order row below. Shares are of the 2,926 the rows cover:

| Class | Functions | Share |
|-------|-----------|-------|
| Different instructions on the two sides | 1,394 | 47.6% |
| Same instructions, different order | 562 | 19.2% |
| Other extra instructions | 469 | 16.0% |
| One or more extra slot-to-slot copies, nothing else | 304 | 10.4% |
| One or more extra handle aliases, nothing else | 138 | 4.7% |
| Identical but for a slot number, or extra copies AND aliases | 59 | 2.0% |

The classes that used to dominate — a named temporary costing a copy or an alias — are now the
small ones. Over this run's work the total went from 14,134 to 3,511, and `__InitDefaults`
differences from 37 to 6.

No single shape dominates any more: the largest signature inside the largest class is 38
functions, where it was 754. The ones worth naming: 38 where an extra constructor and destructor
pair says the emitter named a value the source built at a call site, 31 and 30 where a branch is
tested the other way round, 30 where a constant is written that vanilla copied, 28 where a
`float32` value is compared without the widening to `float` vanilla performed first (the
comparison then runs at the wrong width — the widening is rendered as a plain assignment, so the
folds collapse it as if it were an alias), and the order class above, whose instructions match
but run in a different order.

### The loop whose condition is a short circuit — RECOVERED

This is fixed; the account below is what it was and how it is read now. 84 loops came back, and
they were not only bytes: the body of each ran ONCE in the decompiled source where the game runs
it until the condition fails.

The structurer marks a then-arm whose last block jumps back to the test, and the emitter turns the
pair into a `while` once its short-circuit folds have made the condition one expression — which is
what a loop head needs. Where the condition is still a bare name whose producer cannot move into
the head, the mark is swept and the `if` stays exactly as it was, so nothing is guessed.

It used to be: 41 functions lost a loop outright — the back edge AND the `SUSPEND` that comes with it — and the
diagnosis is exact. `uncond_latch_loop` asks that the header be a SINGLE two-successor block. Where
the condition short-circuits, it is not: `while (!A() && !B())` computes its value across three
blocks and tests the result in a fourth.

The smallest case, `UAIState_TheftPursuit::WaitUntilDeadlineOrExit`, is 21 instructions:

    [0000] …IsTimeInThePast … JLowNZ ->[0009]     <- the back edge targets HERE
    [0007] SetV4 v1, 0 ; JMP ->[0014]
    [0009] …ExitEarly … CpyVtoV4 v1, v2
    [0014] CpyVtoR1 v1 ; JLowZ ->[0020]           <- the exit test
    [0016] SUSPEND ; WaitOneTick ; JMP ->[0000]   <- the latch
    [0020] RET

The detector starts at block 0, takes the false ARM of the short circuit for the loop body, finds
the exit inside the latch span and bails — correctly, for the shape it models. The `is_cond` arm
then renders the region as an `if`, and the latch's `JMP` has no rendering at all.

Rendering it faithfully needs the CONDITION as an expression at the loop head, and the structurer
does not have one: it writes the short circuit as a two-armed store and leaves the merge to a text
pass in the emitter. `while (true) { …; if (!c) break; … }` is NOT the same bytecode — the break
costs a jump vanilla does not have — so the fix is to give the structurer the expression, not to
pick a different keyword.

Some things measured and REFUSED, so they are not tried again:

* Treating a name wrapped in a CONVERSION as a movable argument candidate. Vanilla really does
  evaluate such a call inside the argument list — `TSubclassOf<T>(StaticClass())` rather than a
  statement above the call — but moving it there cost more elsewhere than it paid: 2,286 -> 2,307.
* Letting the widening-alias fold take an EXPRESSION rather than a name. `this.Radius * 2.0f`
  really is a float32 — but so is `A - B` on two FVectors by the same bracket-free test, and that
  one reaches a float parameter as "No conversion from 'FVector' to math type available" (10
  errors). Typing an expression needs more than its punctuation.
* Writing an enum constant as its enumerator NAME rather than a conversion. The cache carries the
  names for the 32 SCRIPT enums (the other 120 the corpus uses are native, and their names live in
  `Binds.Cache`, which is not decoded here). It is the spelling the source had and it is kept —
  but it is byte-neutral: 2,353 before and after. The order difference it was meant to explain has
  another cause.


* Running the producer sweep once more on the joined text: 2,470 -> 2,472.
* Letting the accumulator fold skip a declaration standing between the value and the
  accumulation. The shape is real — `float X = <member>; float k = 1.25; X = X * k;` is one
  expression in vanilla — but folding it there cost more elsewhere than it paid: 2,439 -> 2,444.


* Letting the producer-statement witness walk PAST an intervening call. A store whose slot is
  pushed much later is a producer the source named, and the walk stopped at the first call in
  between — which is exactly where the other arguments of the same statement get evaluated. Widening
  it recovered about a hundred receivers vanilla had named and cost more than three times that
  elsewhere: 2,657 -> 2,935. The witness needs the operand stack, not a wider window.
* Lifting the constant out of `SetV1 slot, k; CpyVtoR4 slot` in the return recovery. The shape is
  real — `if (cond) { return false; }` reuses the condition's slot for the constant, and reading
  the register as the SLOT names the condition the branch has just proven TRUE. 19 functions carry
  that defect (`if (X) { return X; }`, which always returns true where vanilla returns false). But
  popping the store where the return reads it moved the wrong statement in a function with several
  returns. It needs the branch structure, not the last line pushed.

The engine type ids are gone from this table, and not by fiat. A `TYPEID` operand's numeric value
is an `asCTypeInfo` id the engine assigns as it registers types; it drifts whenever the set or
order of registrations changes, which is the same build noise the reference normalizer already
takes out. N7 resolves such an operand through the side's OWN type table and compares the type it
NAMES, carrying the handle and const-handle bits along so a handle is never equated with a value.
It is fail-closed: an id either side cannot resolve stays compared by value. It moved exactly the
177 functions and nothing else — every one of which had no emitted source at all, being a
generated component accessor or delegate thunk.

### The install is shared

The game installation is not this project's alone: a second worktree runs a standalone compiler
that stages its own `.as` tree into `G1R/Script` and swaps the same caches. Cooperating GORE
processes serialize on `.gore-install-mutation.lock`, but a run started with a different or older
binary need not honour it, and two compiles overlapping do not fail — they produce a regen cache
built from a MIXED tree, which reads as alignment loss.

`scratchpad/cycle.sh` therefore waits for the game to exit AND for any live lock owner to finish,
refuses to start unless the shipping cache is the vanilla hash, and prints that hash again
afterwards. A cycle whose "after" line is not vanilla measured something else.

What limits the rest is TYPE evidence. A slot declaration also performs the conversion the direct
read would not, so moving a producer into its reader needs proof that the read has the same type.
The widenings that were measured and rejected rather than shipped:

- moving any operand without that proof costs 1,021 compile errors, almost all `int` to `bool`
  and back;
- writing every unread call result as a bare statement cost 265 — now shipped: two of the three
  reasons the compiler gave are answerable from the cache (CONSTRUCTING a value and dropping it
  is not a call, and a CONST call has no side effect to keep, which the function table records).
  The third, `nodiscard`, is a property of the C++ binding and appears in no cache; those eight
  names are cited from what the compiler reported on this corpus;
- the remaining `if`/`else`-over-one-slot is the SHORT CIRCUIT `A && B`, not `A ? false : B`:
  different AngelScript codegen paths, and only `&&` writes its deciding constant straight into
  the result slot (13,255 `SetV4 x,0` stores sitting between a conditional jump and a `JMP`,
  against 1,303 `SetV1 x,1` for the `||` mirror). Written back as the operator it was — including
  the self-referential links of a CHAIN, and arms that step through a temporary of their own — it
  compiles and reproduces vanilla's guard. Feeding `&&` a NON-bool operand takes the compiler
  down without a diagnostic, which cost two whole-tree runs before the type check went in; the
  left operand has to be turned around (`x != nullptr`) rather than wrapped in `!`, or the
  compiler materializes the negation where vanilla inverted the jump; and mixing `&&` with `||`
  without parentheses is a warning, which this compiler treats as an error;
- writing the remaining `if`/`else`-over-one-slot as the conditional expression it was is
  reachable — the witness types those merge slots `bool`, so both arms unify and the tree
  compiles — but it does NOT reproduce vanilla: the compiler still materializes the constant arm
  in a temporary and copies it (`SetV1 t,0; CpyVtoV4 slot,t`), where vanilla writes `SetV4
  slot,0` straight into the pre-allocated slot. All three source forms have now been measured
  against the real compiler — `if`/`else` over a named local, `?:`, and `?:` with the arms cast —
  and none of them emits vanilla's shape. This class is not reachable from source;
- folding a temporary into a condition needs the same proof in BOTH directions. Where the slot is
  an `int` the emitter compares against zero, dropping the comparison is right only once the
  value is PROVEN a bool — the class's own field map answers that where the local type table
  cannot. Folding any left-hand relation without that proof costs 44 errors, all of them `No
  conversion from 'int' to 'bool'`;
- treating `Cast<T>(x)` as a call the producers may move into was measured and rejected twice.
  With the cast's null-guarded if/else folded first it costs 1,172 functions (7,926 to 9,098);
  with the fold left where it was it costs 1,343 (to 9,269). Refusing the receiver position
  outright recovers 14 of them, so the receiver is not what does the damage — a cast is simply
  not a call whose operand the source evaluated at the call;
- opening up `try_eliminate_adjacent_value_slot`, which today runs only for a function the enum
  pass had something to say about, was measured and rejected over five whole-tree runs. Removing
  the enum gate alone costs 5 functions and 4 byte-identical ones. Admitting a slot by a proof
  read out of the ISA's own operand roles — every read of the slot is the instruction directly
  after a write of it, so it holds a run of one-instruction live ranges and no source variable
  ever occupied it — gains 6 and still loses the same 4. Lifting the pass's other three gates on
  top (a consumer past block punctuation, a call with several arguments, `X = !X` over any
  producer) does not compile: it drops a `const` qualifier a later pass would have written, and
  the type witness that would say so is not in reach at that point in the pipeline. The proof
  itself is sound; what is missing is the constness the slot table does not spell;
- peeling a fluent method chain from the right, so each link becomes a call site whose parameter
  row can admit a producer, is the one experiment that breaks ALIGNMENT, and it is now CONFIRMED
  under guard: the tree compiles with 0 errors, the install is verified vanilla before and after,
  and the generator still emits 287 fewer functions than vanilla has. Byte-identical collapses
  from 7,051 to 1,038 and the reference normalizer fires on nearly every function, which is what a
  shifted function table looks like. An earlier revision of this document called the result
  unconfirmed because the install is shared with another worktree's compiler; it is not that.
  (Two `Resulting reference cannot be returned` errors have to be guarded away first — nothing may
  move into the `return` of a function that returns by reference — or the tree does not compile at
  all and the alignment question never gets asked.);
- substituting a default-constructed `T()` at any ARGUMENT position took three measurements to
  get right, and the sequence is the lesson. Asking `arg_position_accepts_temporary`, which is
  keyed by the callee's NAME, costs 35 errors: for `FindFloorAtLocation(Location, FHitResult(), …)`
  it answers yes for a parameter the callee writes THROUGH. Adding that the slot must be mentioned
  NOWHERE else in the body — a value the caller reads back is read back somewhere — halves it to
  15 but cannot close it, because the compiler refuses a temporary for a non-const reference
  whether or not the caller cares. What closes it is the call's own function POINTER: it names one
  overload, and `func_params_by_ptr` gives that row's parameter flags exactly. Shipped with both
  witnesses (0 errors);
- protecting the `float32`-to-`float` widening from the folds, so that `if (x > 0.0)` runs at the
  width the original ran at, was measured THREE times and rejected three times: refusing every
  conversion destination costs 74 functions, narrowing it to the one widening that matters
  (`fTOd`) costs 12, and leaving that widening's whole statement untouched on both sides — which
  reproduces vanilla's three slots exactly, verified by hand — still costs 3. Each variant fixes
  its own 28 functions and loses slightly more elsewhere. The shape is real; what is missing is a
  reason to keep this one statement that does not also keep statements nothing depends on. The
  value reaches the comparison through a CHAIN of copies, so no test on the immediate reader can
  see it — that is what makes the cheap versions of this rule too broad;
- letting the member-read fold treat an ACCUMULATOR as its reader — `local_N = this.F;` followed
  by `local_N = <x> - local_N;`, where the slot is both the target and an operand — is the largest
  refusal that fold has (295 of its sites) and it does not pay: the tree compiles clean once the
  store is required to dominate the reader, and the corpus goes from 3,103 to 3,114. Dominance has
  to be same-BLOCK, not same-column: two sibling arms share an indent, and a path through the
  other one reaches the accumulator with nothing in it (6 warnings, then 3, then none);
- writing an enum constant as its enumerator NAME rather than as a cast of its ordinal —
  `EPerceptionCharacterType::None` for `EPerceptionCharacterType(0)` — is available and does not
  pay. An earlier note here said the cache carries no enumerator names; that is wrong for SCRIPT
  enums, which `read_enum` decodes with their entries. It is right for NATIVE ones, whose names
  live in `Binds.Cache`, which this project decodes only for field types and arities. So the rule
  reaches 15 of the corpus's 61,894 enum-constant sites, and the scoreboard does not move. The
  order class it was meant for — vanilla computing a member's ADDRESS before the value where we
  compute the value first — needs the native names, or evaluation-order control in the
  structurer;
- three separate widenings measured EXACTLY neutral and were taken back out rather than kept:
  ungating the bool-field witness from the enum state machine, stepping the return-value scan back
  over a scope's cleanup, and running the temporary folds to a fixpoint. Each asks a question the
  cache answers correctly; each was already answered by a rule that fires first.

A fourth was tried and rejected: whether a producer stood INSIDE the expression that reads it is
decidable from the bytecode — AngelScript emits each argument's own code immediately before its
push — but approximating that as "the store is adjacent to the push" refuses far more than it
should and costs 4,222 functions (8,507 to 12,729, measured). The real test is the producer's
position relative to the OTHER arguments' pushes, which needs the call's push order, not one
instruction's neighbour.

The third is now answered rather than open: it was the witness the `?:` needed, and with it the
form compiles and still does not match. What is left of that class is a codegen shape, not a
missing rule. One collision is worth recording: the emitter marks an operand it could not resolve with
` ? `, which is also how a conditional expression reads — anything that emits one has to teach
that check the difference.

Two shapes came off that list by asking the bytecode where a statement STOOD rather than what it
did. Both rest on the same fact: this fork's compiler emits a jump to the function's epilogue for
every `return`, and only a `return` that is the last statement of the function's OUTERMOST block
has that jump folded away.

- **A tail that returns from inside an `else`.** When a then-arm returns, the emitter flattens the
  else and writes the rest sequentially — right for most functions, and measured as such (writing
  the `else` cost 301 extra jumps). But where the else region's last block jumps to the bare `RET`
  row, vanilla NESTED it, and flattening drops that jump: 94 functions, nearly all generated
  dialog `Act_Implementation`. The region's own terminator decides it. The shared `RET` row must
  then not be rendered as a statement of its own — every path already returned, and this compiler
  treats unreachable code as an error.
- **A named receiver reorders its own statement.** `T local_N = <call>; local_N.Field = <rhs>;`
  and `<call>().Field = <rhs>;` hold the same instructions in a different order: a receiver held
  in a declaration is evaluated BEFORE the right-hand side, one spelled inside the assignment
  after it. Vanilla's `STOREOBJ` says which it wrote — standing directly before the push that
  consumes it means nothing was named. 56 sites.

Together: 2,231 to 2,111, compile clean, no alignment loss.

The failure in between is worth keeping: the first attempt compiled to ONE "Unreachable code"
warning, which this compiler promotes to an error, and that cost a whole cycle. `scratchpad/
unreach.py` now walks the emitted tree the way the compiler does (0 on a tree that compiled,
exactly the compiler's line on the tree that failed), next to the scope and l-value checkers.

A third witness of the same kind, and the compiler fact behind it: **this compiler cannot write a
scalar local directly.** `bool b = false;` is `SetV1 vT, 0; CpyVtoV4 vB, vT`, and a named `float`
is the destination of a copy, never of the widening itself. So a slot that is PRODUCED — a member
read, a widening, a call result, a negation — and never copied ON is the compiler's own
temporary, and the source spelled that expression where it is used. Naming it costs the copy.
Refused for their own reasons: a slot whose address is taken (`PSF`) is a real variable the callee
writes through (this is what keeps an `&out` argument a variable); a copy's destination is a name;
a slot produced twice is not one value. The reader is not always the line below — several
temporaries of one call stand in a row, each holding an argument — so the fold walks past sibling
temporaries and refuses anything else in between.

The same fact fixes a WRONG PROGRAM. `SetV*` registers a constant rather than rendering a
statement, so where no store was rendered the return kept the slot — and the slot still carried
the condition just tested. `if (!ok) { return false; }` came back as `return <the condition>`,
which is `true` there. 29 functions returned the opposite value; they now return the constant.

Also measured and then narrowed: two tests that jump to the same false target are `A && B`, and
rendering them as a nested `if` leaves the middle path — A true, B false — running nothing where
vanilla runs the `else`. But two guard clauses in a row fail to the same place as well — the
function's own epilogue — and merging THOSE only costs the carrier the compiler builds for a real
`&&` (measured: 9 functions). The merge is therefore refused when the shared target is the bare
`RET` row.

2,111 to 2,083, compile clean, no alignment loss.

Two more, both about WHERE a statement stands rather than what it says:

- **A declaration belongs where its constructor is.** A value-type declaration costs a
  constructor where it stands, so vanilla's own `PSF vX; CALLSYS ::$beh0` is the evidence: where
  it stands behind a call or a branch, the source declared it there, not in the prologue the
  emitter hoists everything to. The existing sink can only move a declaration into a DEEPER
  block; most of these belong in the same block, just later — behind the guard clauses that
  return before the value is ever needed. Each declaration moves at most once: two that share a
  reader otherwise leapfrog, one below the other, forever (measured — it hung the emitter).
- **A range-for over an expression.** The recovery required the container to be a pure path,
  because a range-for evaluates its container once and a fold must not move a side effect. But
  vanilla says which form it wrote, at the `Iterator` call: a range-for jumps STRAIGHT to the
  bottom test, while a named iterator has the container temporary's cleanup in between. With the
  witness the expression form is safe — and it is not only bytes: written as
  `auto it = <expr>.Iterator();` the container is a full expression, so its destructor runs
  BEFORE the loop and the iterator walks something that is gone.

2,083 to 2,048, compile clean, no alignment loss.

Two more wrong programs, both found by asking what vanilla NAMED:

- **The element of a range-for, read inside a larger expression.** Where an upstream fold had
  written `Modifier.IsA(local_8.Proceed())` instead of storing the element first, the loop no
  longer looked like the idiom and kept its explicit-iterator shape. Vanilla stores the element
  right after `Proceed()`, which both names it and says where its name goes: the recovery now
  puts that name back into the expression and writes the header. (The same store is why the
  unnamed-value fold must never inline a value produced right after `Proceed()`.)
- **A `const` parameter, or `this`, copied for a comparison.** The `RefCpyV` gate refused both —
  rightly for a copy that is written through or handed on, where const would not hold. But
  dropping the copy leaves the comparison reading an UNINITIALISED slot: `if (Node == OtherNode)`
  came back as `if (Node == null)` and `IsIndirectChildOf` always returned false, with its
  parameter unused. A comparison cannot break const, so the copy is materialised into a `const`
  declaration when its only consumer is a pointer compare and nothing reassigns the slot.

2,048 to 2,016, compile clean, no alignment loss.

Then the constant-return fix again, wider, and a third wrong program:

- **The constant is not always the instruction before the return read.** A return inside a scope
  that owns a temporary has that temporary's destructor between the two, and the one-instruction
  look-back missed it: 14 more functions returned the opposite value (`HasAnySensedFighter`
  returned `false` where vanilla returns `true`). The scan now walks back to the start of the
  block over the ops that cannot write the slot — an address push, a constructor or destructor
  call, a `SUSPEND`, a release of a different slot — and stops at anything else.
- **A test inside a then-arm that fails where the outer test fails.** That place is a shared
  TAIL, not an else arm: the source nested two `if`s and wrote the tail once behind them. As an
  `else` the middle path — outer true, inner false — runs nothing at all. An earlier revision
  merged the pair into `A && B` instead; that is wrong too, and vanilla says so, because a
  source-level `&&` consumed by a branch ALWAYS materialises a carrier slot, which these do not
  have. The merge was taken back out and the tail is let fall through.

Refused, measured: admitting a default-argument temporary by its constructor/destructor PAIRING
(two or more matched pairs, the slot only ever pushed by address) rather than by the push in front
of it. It reaches the 20 `IsVisible_Implementation` conversation functions — and it CRASHES the
game's compiler, which exits without a single diagnostic. Two cycles were spent before the tree
was bisected against it. The multi-reader half of the same change is kept: a default argument
spelled out at two call sites is the same temporary twice, and that compiles.

2,016 to 1,971, compile clean, no alignment loss.

The same witness answered the biggest ORDER group. A `STOREOBJ` whose very next instruction
pushes the same slot produced its value where it is consumed, so the source wrote that call inside
the expression; held in a local instead it is evaluated BEFORE the outer call's other arguments —
the same instructions in a different order. 162 modules changed, and the largest single group of
the order class went with them. The constant store joined the producers for the same reason a
member read did: a named literal is the destination of the COPY (`bool b = false;` is
`SetV1 vT,0; CpyVtoV4 vB,vT`), never of the constant store itself.

1,971 to 1,919, compile clean, no alignment loss.

One class is now understood and cannot be recovered: the 18 functions where vanilla carries one
extra `JMP`. In all 18 that jump targets the instruction after it, and deleting it makes the two
sides identical. It is what survives of an `if` whose condition folded to a constant and which had
an `else`: the test and the dead arm are gone from the bytecode, the skip-else jump is not. The
arm's text was never compiled, so nothing in the cache can reconstruct it — four of them also
carry that arm's frame slots, which is where the AttackThrow shape's extra 8 bytes come from.

Two refinements of the same witness, 1,919 to 1,897:

- **Only the FIRST definition of a slot is the one the text names.** What the compiler does with
  the same frame afterwards — a cast's null arm, another temporary — is invisible to the source
  and was disqualifying the slot. `Other.GetCharacter()` came back as a named local because slot 2
  is later the null arm of a `Cast<>`.
- **A block that declares two handles ends with two releases**, and only the last of them stands
  directly before the brace, so dropping one uncovers the next. The release drop now runs to a
  fixpoint, and last in the chain — the folds before it can delete the statement that stood
  between the release and the closing brace.

A value's life ends at the next write to its slot, not at the end of the function: the compiler
reuses a frame slot for unrelated values and the emitter names each of those separately
(`local_8`, `local_8_2`), so reads after the next write belong to someone else. Counting them
refused most of the copy class. Worth only 2 net — most of the 83 modules it changed were already
byte-identical — and it exposed one real asymmetry worth recording: this compiler accepts
`intSlot = boolLocal;` and refuses `intSlot = false;`, so a literal is not folded into a plain
copy whose destination carries a recovered type of its own.

The by-value twin of the same rule, and the largest single step so far. A function returning a
struct writes it through a hidden out-pointer: the caller pushes the destination slot's ADDRESS
before the call and pushes it again straight after, to hand the value on. Where those two pushes
bracket the call and nothing else claims the slot, the value was consumed where it was produced —
`Self.GetActorLocation().Dist2D(...)`, not a named `FVector`. The general rule refuses any slot
whose address is taken, because a callee can write through it; here the callee IS the producer.

Two guards it needs, both measured as compile failures first: such a value may only be inlined
where it is the RECEIVER (as an argument it is a temporary, and this compiler refuses a temporary
for a non-const reference parameter), and its initializer must be a call chain rather than a
bracketed operator result (whose temporary binds to the method's own non-const `this`).

Beside it, the trailing argument that IS the callee's declared default is not written at the call
site. The defaults are in the cache and already round-trip into the emitted declaration, so a call
whose last arguments are default-constructed temporaries can be written the way the source had it
— except where the parameter is a non-const reference (`FVector &inout EndPosition = FVector()`),
where omitting it makes the compiler bind its own temporary to that reference and refuse.

1,895 to 1,764, compile clean, no alignment loss.

The short-circuit recovery asked too much of the value arm: it accepted the arm's own
intermediate step only when that step WAS the whole value. Usually it is one OPERAND of it — the
compiler materialising the right-hand side's sub-expression — and putting it back where it was
read makes the arm one expression again, so `if (A) { c = true; } else { … c = <expr>; } if (c)`
folds to `A || <expr>`. Two shapes are refused: a substitution that leaves a bare member path
standing (the comparison fold behind it would write `path != 0`, which this compiler refuses for a
bool field) and an int-carrier comparison `(carrier != 0)`, which that same fold rewrites through
the declaration this one would be consuming.

Keying the temporary rules by the LIFE of a slot — the emitter numbers a reused slot's
declarations `local_8`, `local_8_2`, in program order — was recorded here as a refusal on the
strength of two runs that died without a diagnostic. It is not one: on the third attempt the same
tree compiled and measured nine functions better. The lesson is the method, not the rule.

1,743 to 1,720, compile clean, no alignment loss.

**A compile that fails with no diagnostic is a MEMORY failure, not a verdict.** The generator peaks
at 4.6 GB on this corpus (the machine has 68 GB, so the ceiling is the compiler's own), and near
that ceiling it dies nondeterministically: the same tree failed twice with exit 3 and no
diagnostic, then compiled on the third run and measured 12 functions BETTER. Every conclusion of
the form "this widening crashes the compiler" below was drawn from one or two such runs and has to
be re-tested with retries before it is believed. What a real source error looks like is a
`file:line:column: error:` line — those are deterministic and reproduce.

With that caveat, three widenings of the inline-where-produced rule were tried:

- keying the rule by the LIFE of a slot (`local_8_2`), for value and object temporaries alike;
- admitting a default-argument temporary by its constructor/destructor pairing;
- letting the consuming push be separated from the store by the other arguments of the same call
  (the object temporary "consumed where produced" window), with and without an added guard that
  the argument position provably takes a temporary.

The bisection was run for the third of them: every subset of its 31 changed modules compiles —
16, 14, 1, and 30 of them — while all 31 together do not, and a single folded statement compiles
where four of the same shape do not. That is not a construct the compiler refuses; it is the
memory ceiling above. The same widening also measures WORSE where it does compile (1,736 / 1,728 /
1,744 against 1,720), so it is dropped on its own merits. The other two are open again.

One change was kept although it costs four functions: a producer standing before a loop is no
longer inlined into the loop's condition. A loop header is evaluated once per ITERATION, and
vanilla computes such a bound once and compares against the slot — `while (i < Math::Max(a, b))`
recomputes the maximum every time round. That is not a byte difference but a different program,
in 67 functions, and the four it costs are byte shape.

An `||`/`&&` chain of three or more operands is computed into one carrier slot, and the emitter
cut the chain where the carrier was written. Putting the left half back is safe — the leftmost
operand is evaluated first either way — and two things had to be got right for it: `(A) || (B)`
starts with a bracket and ends with one WITHOUT being wrapped by a single pair, so dropping it
into a chain unbracketed re-binds it (`A || B && C` is not `(A || B) && C`); and a carrier that is
read by a STORE rather than by a `return` or a test may not be folded at all, because the store's
target carries a recovered type of its own and a bool chain in an int carrier is a conversion this
compiler refuses.

The receiver of a member read-modify-write joins it: `STOREOBJ` followed by `LoadRObjR` of the same
slot is the same "consumed where produced" witness as a push, so `X = GetG1R(); X.Field += 1;`
becomes `GetG1R().Field += 1;`.

1,687 to 1,670, compile clean, no alignment loss.

`!` is a VALUE operator in this language: it cannot fold into a branch, so every one that survives
costs a spill — `CpyRtoV4; NOT; CpyVtoR1` in front of the jump that would otherwise have tested
the result where it stood. `negate` turned a relation around but never took a `!(X)` off, so
`negate(branch_cond(...))` doubled it: `if (!((x != nullptr)))` for what vanilla wrote as
`if (x == nullptr)`. It now strips a `!` that wraps the WHOLE condition, and refuses to turn a
relation inside a short circuit, where negating one half is simply wrong.

Method note, learned the hard way: a compile that fails with no diagnostic needs THREE OR FOUR
attempts before it means anything. This tree failed twice and compiled on the third run; both of
its halves compiled first time. Every "the compiler refuses this" conclusion in this file was
drawn from too few runs and should be re-tested the same way.

1,670 to 1,657, compile clean, no alignment loss.

The default-argument pairing was re-tested the same way and this one IS refused, for a reason the
earlier note did not have: admitting those slots makes the argument-temporary pass spell out
`FInGameTime()` in the very positions the default-argument pass had just dropped, and one of them
is an `&inout` parameter — a temporary bound to a non-const reference, which this compiler will
not take. Four runs, no diagnostic, and the diff says why. The two passes work against each other
there; the drop is the one that pays.

One more refusal, measured three times: writing an anonymous `T()` in an ARGUMENT position where
the cache has no signature for the callee. `T x; f(..., x);` with nothing ever writing `x` is
`f(..., T())` — but `arg_position_is_written_through` answers "no" both when it knows the position
takes a value and when it knows nothing at all, and the calls this reaches are the ones it knows
nothing about (`SendGameplayEvent`, `OnBeginOverlap` — whose last parameter is an out-parameter).
The return form of the same rule is safe and is in: a returned value has no parameter to bind to.

**A wrong program, found by the sweep and now closed.** Where a slot is re-defined while a
reference to its PREVIOUS value is still pending as an argument, the pending reference ended up
rendered with the new life's name. `UChoiceDiego107920::Act_Implementation` comes back
as

    AGothicCharacterState local_6 = this.GetIan();      // never read
    AGothicCharacterState local_6_2 = this.GetDiego();
    ::Say(local_6_2.GetAI(), …, local_6_2.GetCharacter(), …);

where vanilla pushes Ian's character as the listener and Diego as the speaker: the compiler stores
Ian in slot 6, pushes it, then stores Diego in the same slot for the receiver. Our renderer names
both uses after the second life, so the line says Diego twice and the first store is dead. About
35 functions carry this shape, most of them generated dialog, where it makes an NPC address
itself.

The cause was one pass ahead of where it showed: the argument inliner runs BEFORE the pass that
splits a reused slot into `local_6` and `local_6_2`, so carrying a read below the second
definition handed it to the wrong life once the split happened. The inliner now refuses to move a
value past a line that re-defines a name the value reads. What it writes instead is what vanilla
wrote: `Say(GetDiego().GetAI(), …, GetIan().GetCharacter(), …)`, both calls inline. 1,648 to
1,613 — the byte count and the 35 wrong programs were the same defect.

**A declaration whose first use is inside a block stands in front of that block.** The sink
could already move a declaration down to a later statement in the same block, and down into a
deeper block when the whole life fitted there. What it could not do is the ordinary case in
between: the value is read inside an `if`, but its constructor has to stay outside because the
`else` arm or a later statement reads it too. Vanilla puts the declaration on the line above the
block header, and the byte evidence is the same as for the sink itself — the constructor call
stands after the guard, not in the prologue. `AI.CharacterAI_Gothic::IsCharacterInCombatRadius`
is the shape: `FVector local_10;` directly above `if (this.InitialConflictPosition.IsSet())`.

Two placement rules had to be measured rather than assumed. The insertion point is chosen by
BRACE DEPTH, not by indentation — the first attempt used the rendered indent and put a
declaration inside a block it was then out of scope for. And an `else` continues the statement
above it, so nothing may be inserted between a closing brace and its `else`; the compiler answers
that with `Expected expression value / Instead found reserved keyword 'else'`, and it took a
diagnostic-bearing attempt to see it, three prior attempts having died with no message at all.
1,613 to 1,594.

**A handle member's null is the compiler's, not the source's.** A `= nullptr` on a handle field
is an assignment EXPRESSION: it pops its discarded result and it moves the member into the second
init pass, behind every implicitly-defaulted one. The compiler's own zeroing does neither. Which
one vanilla wrote is in the constructor's bytecode: `PshNull; PshVPtr w0; ADDSi wOFF,TID; REFCPY`
with nothing behind it is the compiler's; a `PopPtr` behind it, or a store through the member's
assignment operator, is the source's. Dropping every `= nullptr` without asking that question
fixed 11 struct constructors and broke 11 class ones, so the witness is the whole lever.

**A slot's earlier lives can be folded away, and then its name no longer counts.** The sets that
say "vanilla never named this" are keyed by slot and life, but the pass that versions a reused
slot works on the rendered text — so a life the structurer folded away shifts the text's numbers
off the bytecode's, and a value vanilla never named kept its name. Two of those folds are now
accounted for: a store consumed where it is produced, when NO other write touches the slot, and
an alias built from a PARAMETER for an immediate null test. Both had to be bounded by measurement
rather than by argument. Reading only the store called a `Cast<T>` destination a temporary — a
cast writes its slot twice, `opCast` on one arm and `ClrVPtr` on the other — and dropping every
compared alias inlined 19 casts into their own null tests.

1,594 to 1,565.

**A call result parked in a slot and read straight back is a NAME.** The compiler branches on
the value register where it stands — 4,777 plain-`if` and 855 short-circuit sites in vanilla do
exactly that, including 1,714 for the same `IsValid` that spills 27 times. So a `CALL; CpyRtoV4;
CpyVtoR1; J*` run exists only because the source spent a `bool` on the result, and the slot is
not a temporary to fold away. An operator overload is excluded: its own call path materialises
the pair with no name involved.

**A range-for element is released INSIDE the loop.** The set that protects loop elements from
inlining took every `FreeNullV8` in the function, so a release standing after the loop's
back-edge — which belongs to the CONTAINER temporary, and is the very evidence that the container
was written inside the `for` header — was read as proof that the container had been named. It is
now bounded by the back-edge's span.

1,565 to 1,550.

**The game moved under the measurement (2026-08-28).** Steam shipped a new build: the shipping
cache went from `d0afaf90…` to `7a18f954…`, ten modules appeared, and 341 functions changed. The
numbers below are re-based on it — the rules did not change, the corpus did. Two of them the new
build turned into compile errors, and both were gaps this file can now name:

- **A const handle's declaration has to say `const`.** One getter became `const UCrimeDefinition`,
  and a declaration WITH an initializer already renders the return type in full — but a hoisted
  bare declaration is typed from the slot table, which carries only the base name. The store is
  then refused outright. The slots are found the same way their types are: a call whose return
  `DataType` is a const object handle, and the `STOREOBJ` that lands it.
- **A cast's destination type outranks a member's declaring class.** `is_subclass` walks script
  supers and stops at the first native one, so the chain `ACharacter -> APawn -> AActor -> UObject`
  is invisible to it and every narrowing across it was refused. Naming the six engine bases is the
  proof that is available: a cast to a SCRIPT class is narrower than any of them. Both type maps —
  the structurer's view and the declarations — now carry the same rule, which is the whole lever:
  narrowing only the view took the receiver wrap away while the declaration stayed at the base,
  and `AActor local_2 = Cast<AGothicCharacter>(…)` could no longer answer `GetCharacterState()`.

**`T x; return x;` and `return T();` differ only in which constructor runs first.** For a
declaration the local is built first (`PSF vN; <ctor>`) and the return object after it
(`PshVPtr vM; <the same ctor>`); for an anonymous value the return object comes first. The pair,
adjacent and with the same callee, is vanilla saying the source declared the local — so the fold
that writes `return T();` must leave it alone. The prediction was a closed list of four functions
before the run and all four came back byte-identical, which is the strongest evidence this file
carries that a witness read off vanilla's own stream says what the source wrote.

**A producer's evidence ends where its STATEMENT ends.** The walk that decides "another operand
went on the stack before this slot's own push" ran past branches and past instructions that READ
the slot without pushing it — so a `CmpPtrNull v2` behind the store, which is the null test of the
very expression the value belongs to, was walked through into the taken arm, where the cast
lowering's own `PSF` counted as somebody else's operand. It now stops at the first jump and at the
first non-push read.

1,573 to 1,549, three of them traded for the first cast-diamond shapes the tighter walk now folds.

**An alias built for a comparison is the comparison's own temporary — on either side of it.**
The rule was already there for a parameter aliased into a null test; it read only the
comparison's FIRST operand, so every `<expr> != this` vanilla wrote with the entity spelled out
stayed a named local on our side. `this` is slot 0, a parameter is negative, and both are entities
the source names outright. Three functions, no regressions.

A release is also a name: `FreeNullV8` is what a handle DECLARED inside a block costs, so a slot
carrying one can no longer be called wholly consumed — the object mirror of the scope-exit
destructor that already makes a value-type slot loose.

1,549 to 1,546.

**A destructor standing before the return object is built is deferred-argument cleanup.** The
scope has not been left and the return value does not exist yet, so that `$beh2` cannot be scope
exit — it is the cleanup of the arguments the returned expression consumed, which means those
operands were written inside the expression rather than declared above it. 1,546 to 1,545.

**The direction that was missing: a handle vanilla PARKED was named.** Every rule above runs one
way — it finds evidence that vanilla did NOT name a value and folds our local away. The mirror was
absent, and roughly half the ordering class was unreachable without it. It is the CONTENT OF THE
GAP: a producer writes a slot, exactly one later instruction reads it, that read is a plain
`PshVPtr`, and real work stands in between. This compiler evaluates a call's arguments first and
its receiver last, and turns a register result into a variable only when pending cleanup is in the
way — so a value produced, left sitting while unrelated code runs, and only then pushed cannot be
the expression's own temporary. Where the gap holds nothing but `PSF vT; CALLSYS` destructor pairs
it IS that cleanup, and the value is a temporary after all.

Measured over 19,768 producer/single-read sites in byte-faithful functions, where our text IS
vanilla's source: the rule fires 151 times and every one is a name, while the 58 cleanup-only gaps
and the 19,559 adjacent sites hold none. Two smaller rules came with it — a value slot destroyed at
two different program points was declared (232 of 232 on the same control), and adjacency now reads
past deferred cleanup.

Three things were ruled OUT by the same control, and that is worth as much: the bool spill is not a
name (61 of 69 byte-faithful sites carry it from text that names nothing), slot geometry does not
separate declarations from temporaries, and the branch-in-the-gap test is a weaker subset of the
rule above.

1,545 to 1,518, twenty-seven fixed and none broken.

**A member initializer can only be lifted onto a field this class declares.** The pass that turns
a constructor's parameter-free member store into a declaration initializer did not ask where the
field lives. The declaration loop writes the class's OWN fields and nothing else, so a store lifted
onto an INHERITED field had nowhere to land and vanished with the store — ten constructors lost a
`SetV1; LoadThisR; WRTV1` that way. 1,518 to 1,508.

**A struct's members are initialised in two passes, and the order says which.** Pass 1 takes the
members with NO initializer: for a primitive or an enum it pushes the DESTINATION first
(`LoadThisR wOFF; SetVn vT, <zero>; WRTVn vT`, or `PshVPtr v0; ADDSi wOFF; PopRPtr; …` where the
constructor takes parameters). Pass 2 takes the members that HAVE one and lowers it as an ordinary
assignment, so the VALUE is compiled first. Address-first therefore says the source wrote that
member bare — asked per FIELD, because one struct carries both kinds side by side. 1,508 to 1,495.

**The bool spill has three producers, and only one of them is a name.** `CALL*; CpyRtoV4 vN;
CpyVtoR1 vN; J*` is a bool result stored and read straight back before the branch, and this
compiler otherwise branches on the register where it stands. The round trip has three causes: a
stack TEMPORARY handed to the call by reference (its address is pushed twice, and passing a
temporary by reference forces the result out of the register even when the temporary has no
destructor to emit); the value being the left operand of a `&&`/`||` that must produce a value,
where the arms write a carrier and jump to a merge that copies it on; and the source declaring a
`bool` for it. Subtract the first two and the rest is the name — 149 sites carry the shape, 70 for
one of the other two reasons, and our own output already reproduces those.

An earlier reading of this shape as a name outright was refused on the control for exactly that
reason, and the refusal was right: without the two exclusions the rule names values vanilla never
named. 1,495 to 1,457, once the fold that removes the name a second time was gated on the same witness.

**A dropped `this` back-link is a wrong program, not a byte difference.** Where an owner makes an
object and hands it a reference back — `local_4.TauntData = this;` — the store was bailed out of
caution over const-ness, and the whole statement disappeared with it: the returned object had no
way back to the thing that made it. The caution is answered by the site itself. The store IS in
vanilla's bytecode, so the source wrote it and it compiled; only our own recovery of the
DESTINATION could still be wrong, and that is a compile error rather than a silent one.

**`++x` is one opcode and `x = x + 1` is three.** The slot-operand increment was written back in
the long form, so vanilla's `IncVi` came back as an `ADDIi` every time. The argument was already
in the file, one arm further down, for the register form. 1,457 to 1,447.

**An `int(...)` that truncates what the source copied.** A member read was wrapped whenever the
destination slot had no entry in the typed-locals map, because such a slot renders as an `int`
local and a float read into one is a precision warning the game compile treats as an error. The
presumption fails for a slot the very next member store writes straight back out at the SAME
width: that is not a declaration but the compiler's pass-through temporary for
`<member> = <member>;`, which the emitter folds into the store — so there is no `int` variable to
warn about and the wrap rounds a rotation angle to a whole degree. Vanilla says so itself: the
window is a bare `RDR<w> vT; <address ops>; WRTV<w> vT`, and had the source truncated, this
compiler would have written the `dTOi` for it. Twenty such statements in twelve functions, every
one of them divergent and none among the byte-faithful. 1,447 to 1,437.

**A returned slot the function also widens is the enum, not an `int` holding one.** `sbTOi vW, vD`
sign-extends vD before an integer use of it, and this compiler never widens an `int` — it does
exactly that with an enum variable. The existing seed only fired when a CALL of the same enum type
filled the slot, so an accumulator fed by a member read stayed untyped, was declared `int`, and
then needed an `int(...)` at every read and an `iTOb` at the return. The witness fires on 47
functions, all of them divergent, and on none of the 54,282 byte-faithful ones.

Measured: **0 fixed, 0 broken**. The declarations are now vanilla's, but every one of those
functions carries a second defect — mostly a loop rotation — so none of them flips on this alone.
It is kept because it removes one defect from 45 records and cannot cost anything, not because it
moved the number.

**`EEnum(int(x))` is `EEnum(x)`.** The inner conversion is emitted wherever a value lands in a
slot the typed-locals map says nothing about, on the reasoning that such a slot renders as an
`int` local and the conversion is then neutral. It is neutral at four bytes; a one-byte enum read
costs an `sbTOi` on the way in and an `iTOb` on the way out, and the enum cast the wrap sits
inside is what vanilla wrote instead. The enum test is what bounds it — the same syntax with an
ordinary callee is a real argument conversion. 1,437 to 1,431, six fixed and none broken.

**An alias carries no type, so the name it gives inherits the engine base.** `RefCpyV` is the
instruction that says the source named a handle — and it propagates nothing about what that handle
is. The slot therefore keeps whatever coarse type the cache recorded (`UObject`, `AActor`), and
the structurer then has to write a `Cast<Owner>(recv)` at every call on it to keep the call legal:
a cast vanilla never had, and eight extra opcodes. For an alias the producer is one hop away and
its DECLARED type is in reach — the field's own type or the callee's return, never a declaring
class, which is the weaker signal that once typed `APawn local_8 = Cast<AGothicCharacter>(…)`.

Three clauses, all load-bearing. The recorded type must be an engine base; the alias must be the
slot's ONLY object write, so nothing else can have been upcast into it; and the slot must be a
RECEIVER — a `PshVPtr` immediately before a call, since this compiler pushes arguments first and
the receiver last. Drop the last one and an `AActor local_6 = this.TargetEnemy;` that is only ever
passed to `Add()` gets retyped, which vanilla's source really did widen.

And the narrowing has to reach BOTH type maps. Landing it only in the structurer's view took the
receiver wrap away while the declaration stayed at the base, and the whole tree stopped compiling
with `No matching signatures to 'UObject::GetRelationship()'` — the same two-map mistake the cast
narrowing made before it. 1,431 to 1,419, twelve fixed and none broken.

**A chain the emitter computed into a carrier of its own belongs in its reader — and the two
passes that put it there were never asked twice.** The fold chain is a fixed sequence, not a
fixpoint, and for a whole class of bodies the shape those passes match on does not exist yet when
they run: the outer chain is still an if/else over a slot, the store appears further down, and the
pass that collapses `X = c; return X;` runs after that. One more pass over the finished text is
the whole repair.

The condition fold is restricted there to a value carrying a TOP-LEVEL `&&`/`||`. That shape
occurs in none of the 54,366 byte-faithful bodies and in 109 slots of the divergent ones; without
the restriction the late pass also takes `bool X; X = <call>; if (X)`, which vanilla wrote WITH
the name — four byte-faithful bodies say so, and read-count does not separate them. The bracketing
needed no change: both passes already wrap unless ONE pair spans the whole value, and the carrier
is always the leftmost operand of the run that reads it, so the wrap costs no bytes.

1,419 to 1,412, seven fixed and none broken.

**A conditional whose taken edge is the latch is not a `continue`.** This compiler never folds a
jump over a jump, so a source `continue` always spends an unconditional `JMP` to the latch — which
the loop-exit rule already recognises. A conditional straight to the latch is instead the compiled
form of a plain `if (<fall condition>) { <rest of the body> }`, and claiming it as a `continue`
costs an extra `JMP` and an inverted condition: a `NOT`, plus the store-and-reload where vanilla
tested the register directly.

Vanilla witnesses the premise in its own stream. `UAIState_TryUseFreepoint` carries both shapes in
one function, and the jump-over-jump guard is byte-identical on both sides while the
direct-to-latch one is exactly where the extra jump appears. The `break` arm keeps its claim: a
conditional straight to the break target still leaves a fall-through that reaches the latch.

The one function this turns into an empty `if { }` was the named risk, and the corpus answers it —
the tree already carried 154 such blocks and compiled. 1,412 to 1,386, twenty-six fixed and none
broken.

**A slot a `T&`-returning call fills holds a POINTER, not a `T`.** Rendered by value it costs a
copy constructor and a scope-exit destructor that vanilla does not have. The type map must not
carry the `&` — there are two of them and a qualifier in one poisons every comparison against the
other — so the reference slots travel as their own set and the `&` is appended at the declaration
sites only. A const return has to say `const T&`, or the initializer is refused outright.

The other half of that lever — recovering `return <name>;` where the value travels as an address
rather than through a register — is NOT in: it put a local out of scope in one function and made
another return a reference into an expression the compiler refuses to keep alive. 1,386 to 1,384.

**A value-type local is destroyed once per exit from the block it was declared in.** So vanilla
spending ONE `$beh0` and TWO OR MORE `$beh2` for a slot, all of them inside a back edge's span, is
the source declaring that local in the LOOP BODY: the constructor ran once per iteration and each
`continue`, `break` and fall-through paid its own destructor. The sink refused every loop
categorically; it now makes an exception for exactly that shape. One constructor is what makes it
a declaration — two are two unnamed temporaries sharing a slot. 1,384 to 1,375.

**A range-for whose element the body writes through is `for (auto& x : c)`.** The recovery
refused any loop that wrote through its element, because a range-for element is read-only — true
of a COPIED element, and not of one the iterator hands back by reference. Vanilla settles which
this is: it jumps straight to the bottom test, the range-for shape, while writing through the
element. The pass runs before the declarations are written, so the reference is read off the
bytecode rather than the text, and the element is spelled `auto&`. Spelled `auto` the source does
not merely fail to compile — it kills the compiler outright, which is how the read-only half of
the rule was re-confirmed. 1,375 to 1,374.

From there the run continued through the rules the sections above describe — the range-for
container, the receiver pair, the split GAS chain, the return-expression temporary, the
continue-only loop scope, and the declaration-order rules — down to the **1,114** the headline
reports.

Cutting across them, 6 are `__InitDefaults` — down from 37, because the language CAN spell
infinity after all: an overflowing decimal literal (`1e39f`) parses and rounds to the bit pattern
vanilla holds, where the largest finite float came back one ULP low every time. The belief that
it could not was carried in this file for months and was never probed.

**30 of the modules cannot be spliced back** (99.59% can). That sweep ran on the earlier
`D0AFAF90…` build, which had 7,308 modules against this one's 7,317; it has not been repeated. Each is a template instantiation
or a behaviour the base cache never recorded — 14 `$beh0` constructors, 13 `TArray` iterators, and
a tail of single cases (`opAssign`, `GetRootNode`, `AssertEquals`). They share the root
cause of the ordering classes above: vanilla wrote the expression inline where the emitter
materializes a local, and that local asks for a copy the base cache has no row for.


## Retained measured baseline (historical, 2026-07-12)

Kept for the record: the earlier build's numbers, superseded by the run above.

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
strict base-keyspace remap path; an overlay that authors defaults is spliced through the same path
with the class defaults regenerated from the source instead of carried. Separately, the offline `default-sites` / `patch-default` path
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

Full corpus on the earlier `D0AFAF90…` build (`Build55_CL171864`), not the run above.

| Metric | Value |
|--------|-------|
| Modules authoring their class defaults | 6,917 — every module that has any |
| Modules suppressed (recovery incomplete) | 0 |
| `default` statements written | 281,422 |
| Vanilla `__InitDefaults` methods | 30,005 |
| Aligned after recompile | 30,005 (**all of them**) |
| Byte-faithful (`IDENTICAL`+`BENIGN`, `--norm-slots`) | 29,999 (**99.98%**) |

The whole emitted tree recompiles with no errors, and `gore as bytediff --norm-slots` reports no
alignment loss at all and B1 **97.97%** over all 164,607 aligned functions — up from 88.78% before
this work, with 3,339 semantic differences left against 18,288.

Editing an existing module's defaults and splicing it back works. Getting there needed six
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
- **The const half of an accessor pair.** `T f()` next to `const T f() const` is the ordinary
  accessor pair and the cache records both; the emitter deduplicated by name and parameters
  alone, so the const half of every one of them was dropped and the rest aligned against the
  wrong twin. Keying the dedup on the qualifier as well took byte-IDENTICAL functions from 771 to
  6,882 and closed the last alignment loss.

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

Measured over the whole corpus, `extract-remap` against the base cache succeeds for 7,278 of 7,308
modules (**99.59%**). The same measurement scored 43 of 60 before the identity work and 58 of 60
after it, on the 60-module sample it started from.

A method's RETURN `const` is part of its identity and is emitted again. It used to be stripped
because the cache sets the flag inconsistently across an override family, and a family has to
declare ONE return type. Two rules replace the blanket strip, and both read the cache rather than
guess:

- A name whose recorded rows DISAGREE about the qualifier keeps the stripped form.
- A name whose const result some caller cannot hold keeps it too. A caller stores an object
  result with `STOREOBJ`; when that slot also takes a null store, a handle copy or a non-const
  call, no single declaration can own it — and AngelScript offers no way to drop the qualifier at
  a store ("No conversion from 'const X' to 'X' available", measured, including through a
  `Cast<>`). The scan is one pass over every function's bytecode during index building.

A local that receives a const result is declared const, at the statement that gives it its value,
because a const local cannot be hoisted. A slot the compiler re-used for several such results
gets one declaration per result. That took the whole-tree compile from 41 errors to none, with 89
const locals and 7 const-returning declarations restored.

The two remaining sample failures are no longer about the qualifier. One instantiates a
`TSoftObjectPtr<AActor>` the base cache never had; the other iterates a `TArray` whose element
type the base has no `Iterator` row for at all, from a NATIVE getter whose signature the emit run
cannot see (it runs without `Binds.Cache`). `GORE_AS_REMAP_DIAG=1` prints the two identities
behind any unresolved or ambiguous reference.

Recovery is all-or-nothing per module. A module that recovered only some of its classes would
silently drop the rest, so one unrecovered class suppresses the whole module and its header records
the class and reason. A defaults-free edit carries existing initializers byte-exact; a bounded
new-symbol hybrid may additionally author defaults on appended classes, but not on a subset of
existing classes. **No module in the shipped
corpus is suppressed any more.** Closing the last of them needed six recovery fixes:

- An in-place update (`local_4 = local_4 * local_6;`) READS the definition above it. That read
  was not counted as a use, so the definition was dropped as a dead store and the read dangled.
- A default-constructed temporary passed as an argument is legal wherever the parameter takes it
  by value or by const reference. Which calls those are is read off the cache's own parameter
  table, over every one-parameter row of that name, so a single non-const-reference overload
  disqualifies the name.
- A multi-parameter or converting construct whose argument slot the block already wrote is a
  real value, not the unrecovered pending result the drop rule was written for. The voice tables
  lost every `Texts.Add(FVoicelineAssignment(...))` to that rule.
- A `b<Upper>` field written from an int is a bool UPROPERTY by UHT's own rule, and its generated
  accessor is `bool&`.
- `Cast<T>(x)` lowers to a null-guarded diamond, and a `default` statement carries an expression,
  not a block. `Cast<T>(nullptr)` is itself null, so the diamond folds back into the cast.
- A namespaced return type (`AutomatedTest::UAIState_…`) starts with its NAMESPACE, so the
  object-factory class-head test read `Au` and refused a genuine factory, leaving its `STOREOBJ`
  slot unwritten.

The two machine-generated main-map tables — 852k dwords of worldpoints, 105k of item spawns — are
authored too. They were refused by a size bound that existed because temporary folding rescanned
the whole statement list for every temporary; the fold now walks the statements once with an index
of where each temporary occurs, which took the worldpoint table from 8m49s to 38s and made the
whole-tree emit faster (58s). `GORE_AS_MAX_DEFAULTS_DWORDS` and `GORE_AS_MAX_DEFAULT_STATEMENTS`
lower the bounds again for a faster emit.

Every recovered statement is checked against the cache's own function table: a rendered `Name()`
whose function the cache knows only WITH parameters means an argument was lost, and the module is
suppressed rather than written. That check found the fluent AI rule builders
(`Rules.Add(t).RequireTrue(a).RequireFalse(b).Then(r)`), where the structurer split the chain
after the first link and the next link took a leftover argument as its receiver. That split is
fixed — a temporary's destructor between two links no longer ends the statement — and the check
stays as the general guard against any future dropped-argument shape.

Six initializers still differ after a faithful recompile, down from 37. The rest were float
constants believed unspellable: `+inf` (`0x7F800000`) was written as the largest finite float and
came back one ULP low. The language can spell it after all — an overflowing decimal literal
(`1e39f`) parses and rounds to exactly the bit pattern vanilla holds.

## What the remaining 1,114 are made of, and which parts are reachable

Measured 2026-08-31 by classifying every record of the whole-corpus run. The point of this
section is to stop the next attempt re-deriving it — three of these were probed by hand and came
back with nothing, and that is worth more than the counts.

| Family | n | Reachable? |
|--------|---|-----------|
| Naming / initialisation boundary | ~400 | Partly. The witnesses hold, but the name alone is not enough — the declaration also has to land where vanilla put it. Forcing names without placement measured 2 fixed / 16 broken. |
| Evaluation order, no constructor moved | 99 | **No.** There is no pattern in the vanilla stream that says which spelling the source used. Four independent classifications reached this separately. |
| Bool carried at the wrong width | ~180 | Partly. The crisp sub-case is smaller than it looks: of the eighty `return (int(x) != 0);` in the tree, exactly one has a return tail with no test in it. |
| Return object built in place | 76 | **Not through the source.** Vanilla copy-constructs into the return slot (`PSF ret; PshVPtr src; $beh0`); we default-construct a temporary and `opAssign`. Writing `return T();` does not produce vanilla's shape — the compiler does not elide. This needs the construction lowered, not a different spelling. |
| `ThrowException` never emitted | 13 | **No.** The construct appears zero times in the whole regenerated corpus; no source rewrite can reach it. |
| Dead loop `while (i < 0)` | 22 | Not through the bound. Restoring `i < arr.Num()` compiles and fixes nothing: the loop FORM differs, not the operand. |
| Pure slot renumbering | ~60 | Known dead end. |

Two traps that cost measurements here, both environmental rather than emitter bugs. A stray
`gore.exe` from an earlier run holds `target/release/gore.exe`, so the build fails silently and
the next measurement scores the OLD binary — kill it first and never filter the build output. And
copying the emitted tree with Python takes the better part of an hour; `robocopy /MT` takes
seconds.


## Root causes and next work

1. **Class defaults are authored; generated defaults are still the fallback, and direct scalars
   keep their narrow offline patch path.** Every module in the shipped corpus writes its own
   `default` statements, so an edit goes through the source. `compile-module --op edit` still
   carries existing `__InitDefaults` plus every emitter-omitted executable record for a module
   that authors none, and only after exact header/tail/reference, declaration/layout,
   method-table, and cache-wide collision proofs. A bounded new-symbol hybrid may add defaults on
   appended classes while keeping all existing generated records byte-exact; authored defaults on
   only a subset of existing classes, an unsupported `__*` shape, or any metadata drift fails
   closed before publishing a mini-cache. `gore as default-sites` and `patch-default` can
   inspect and copy-on-write patch only a unique, branch-free
   `SetV{1,2,4,8} / LoadThisR / WRTV{1,2,4,8}` scalar assignment with exact field-type evidence
   (including parsed-kind proof for script enums),
   a v4 `(module, class, field_owner, field, value_type, ancestry_profile)` identity with proven target-to-owner ancestry, raw
   CAS, one terminal `RET`, and full-cache postconditions. Complex expressions, structs, and
   containers remain unsupported by that scalar workflow. Separately, `gore as tag-map-sites` and
   `patch-tag-map` can inspect and copy-on-write patch only an already-present entry in the sealed
   native `GameplayTag`-to-`float32` map shape; they cannot add a key or map, resize bytecode, or
   author arbitrary map defaults. See
   [`docs/guide/angelscript-defaults.md`](../../docs/guide/angelscript-defaults.md). That scalar
   workflow is now the narrow path, not the only one: arbitrary class defaults are authored in
   source and recompiled.
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
