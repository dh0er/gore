# AngelScript decompiler — completeness and known gaps

**Status: every module decompiles, the whole tree recompiles, and 98.96% of it is byte-faithful.**
The emitter reconstructs every function body it writes from the shipped cache; when it cannot
prove a body is correct it keeps the declaration and emits a clearly marked, signature-preserving
stub instead of inventing logic. The current corpus needs no such stub. What is NOT proven is that
every recompiled body is identical to vanilla — the section below says exactly how much is, how it
was measured, and what is left.

## What is measured, and on what

Measured 2026-08-23 against build `Build55_CL171864` (script cache SHA-256
`D0AFAF909E62867FAEDC3678A1175F5E8DE5E784DC503A14FFBDE4726F297231`, GUID
`be78fe0a46ac6643968597e85c7e5b3f`). This build is not one of the audited generations, so the
numbers qualify the DECOMPILER, not the build.

Everything except the splice test runs over the **whole corpus** — all 7,308 modules, all 164,604
functions the vanilla and regenerated caches align:

| Measurement | Scope | Result |
|-------------|-------|--------|
| Modules emitted, fallback stubs | full corpus | 7,308 modules, **0 stubs** |
| Whole-tree recompile warnings | full corpus | **0** (the compiler treats them as errors) |
| Class defaults authored | full corpus | **0 modules suppressed** (all 30,005 `__InitDefaults`) |
| Whole-tree recompile (`as compile`) | full corpus | **0 errors** |
| Byte-faithfulness (`bytediff --norm-slots`) | full corpus, 164,607 functions | **98.96%** (`IDENTICAL`+`BENIGN`) |
| Alignment loss | full corpus | **none** — every function the cache has is regenerated |
| Splice back (`extract-remap`) | 305-module sample | 302 (**99.02%**) |

Every measurement now covers the whole corpus. The splice sweep takes about two hours (each run
re-reads both 100+ MB caches), which is why earlier revisions of this document reported it from a
627-module sample; the sample and the sweep agree to within 0.1 points.

The measurement needs the game's `Binds.Cache` next to the script cache it reads. Without it the
native field table is empty, every native enum field falls back to the bool heuristic, and the
tree stops compiling (1,474 `bool` to `E*&` errors) — a property of the run, not of the emitter.

## What is left

**1,720 functions (1.04%) recompile to bytecode that differs semantically.** A semantic
difference means *not proven identical*, not *proven wrong*: the whole-tree compile proves the
source type-checks, and `bytediff` normalizes away reference keys, jump absolutes, constant
encodings and (opt-in) slot allocation before judging the rest.

Classified over WHOLE functions — every instruction of both sides, not the window around the
first divergence. An earlier revision of this document classified the window instead and reported
order as the largest class at 4,861; that number was an artifact of the window, and the real
figure is 531:

| Class | Functions | Share |
|-------|-----------|-------|
| Different instructions on the two sides | 1,394 | 47.6% |
| Same instructions, different order | 562 | 18.1% |
| Other extra instructions | 469 | 15.1% |
| One or more extra slot-to-slot copies, nothing else | 304 | 9.8% |
| One or more extra handle aliases, nothing else | 138 | 4.4% |
| Identical but for a slot number, or extra copies AND aliases | 59 | 1.9% |

The classes that used to dominate — a named temporary costing a copy or an alias — are now the
small ones. Over this run's work the total went from 14,134 to 3,511, and `__InitDefaults`
differences from 37 to 4.

No single shape dominates any more: the largest signature inside the largest class is 38
functions, where it was 754. The ones worth naming: 38 where an extra constructor and destructor
pair says the emitter named a value the source built at a call site, 31 and 30 where a branch is
tested the other way round, 30 where a constant is written that vanilla copied, 28 where a
`float32` value is compared without the widening to `float` vanilla performed first (the
comparison then runs at the wrong width — the widening is rendered as a plain assignment, so the
folds collapse it as if it were an alias), and 545 whose instructions match but run in a
different order.

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

Refused, measured: keying the temporary rules by the LIFE of a slot rather than the slot — the
emitter numbers a reused slot's declarations `local_8`, `local_8_2`, in program order, so the
mapping exists and would reach 95 more functions. It crashes the game's compiler outright, with no
diagnostic, exactly like the default-argument pairing did. Both admissions widen what may be
inlined; something in that wider set is more than the compiler can parse.

1,743 to 1,720, compile clean, no alignment loss.

Three separate widenings of the inline-where-produced rule were measured and all three end the
same way — the game's compiler dies with exit 3 and no diagnostic at all:

- keying the rule by the LIFE of a slot (`local_8_2`), for value and object temporaries alike;
- admitting a default-argument temporary by its constructor/destructor pairing;
- letting the consuming push be separated from the store by the other arguments of the same call
  (the object temporary "consumed where produced" window), with and without an added guard that
  the argument position provably takes a temporary.

Each of them is right about the SOURCE — the emitted trees read like vanilla and pass the scope,
l-value and unreachable-code checkers — so what they hit is a limit of the compiler, not of the
witness. Finding it needs a bisection of the tree itself (30 changed modules, one compile each),
which is the next thing to try for this class; together the three reach about 200 functions.

Cutting across them, 6 are `__InitDefaults` — down from 37, because the language CAN spell
infinity after all: an overflowing decimal literal (`1e39f`) parses and rounds to the bit pattern
vanilla holds, where the largest finite float came back one ULP low every time. The belief that
it could not was carried in this file for months and was never probed.

**30 of the 7,308 modules cannot be spliced back** (99.59% can). Each is a template instantiation
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

Same run as "What is measured, and on what" above — full corpus, build `Build55_CL171864`.

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

Measured over the whole corpus, `extract-remap` against the base cache succeeds for 7,276 of 7,308
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

Recovery is all-or-nothing per module, because `generated_defaults` can only carry an omitted
`__InitDefaults` byte-exact for a module that authors no defaults at all. A module that recovered
only some of its classes would silently drop the rest, so one unrecovered class would suppress the
whole module and its header would record the class and the reason. **No module in the shipped
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

The 37 initializers that still differ after a faithful recompile are dominated by float constants
the emitter cannot spell: AngelScript has no infinity literal, so `+inf` (`0x7F800000`) is written
as the largest finite float and comes back one ULP low.

## Root causes and next work

1. **Class defaults are authored; generated defaults are still the fallback, and direct scalars
   keep their narrow offline patch path.** Every module in the shipped corpus writes its own
   `default` statements, so an edit goes through the source. `compile-module --op edit` still
   carries existing `__InitDefaults` plus every emitter-omitted executable record for a module
   that authors none, and only after exact header/tail/reference, declaration/layout,
   method-table, and cache-wide collision proofs. A new-symbol remap, unsupported `__*` shape, or
   any metadata drift fails closed before publishing a mini-cache. `gore as default-sites` and `patch-default` can
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
