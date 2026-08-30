//! Recover class-scope `default` statements from a rendered `__InitDefaults` body.
//!
//! The AngelScript fork this game ships lowers every class-scope `default <expression>;` of a
//! class into one compiler-generated `void __InitDefaults()` method — the compiler sets its
//! "compiling a default statement" state exactly while compiling a function with that name, and
//! generates the method only when the class declared at least one default statement. The
//! statement grammar is `default` followed by an arbitrary expression: assignments
//! (`default m_Value = 10;`), member chains including calls mid-chain
//! (`default List.Last().Pos = …;`), and bare calls whose result is discarded
//! (`default AddItemSpec(GameplayTag::Item_Property_Equipable);`).
//!
//! Everything an item, NPC or config class *is* lives there: the emitter used to drop those
//! methods, so decompiled data classes came out empty. This module turns the already-rendered
//! method body back into the source statements that produced it.
//!
//! Two properties are load-bearing:
//!
//! * **No locals.** A default statement cannot declare one, so every `local_N` in the body is a
//!   compiler temporary and MUST fold back into the expression that uses it. A body still
//!   carrying a temporary after folding is not recovered — it is rejected.
//! * **Fail closed.** A partially recovered body would silently drop game data on recompile, so
//!   recovery is all-or-nothing per class, and the caller makes it all-or-nothing per module:
//!   a module either authors every one of its defaults or none, and the "none" case keeps the
//!   byte-exact carry-through in `generated_defaults` valid.

/// Outcome of recovering one class's default statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DefaultsRecovery {
    /// Every statement of the body became a class-scope default statement. The strings are the
    /// expressions only — the caller writes the `default ` prefix and the indentation.
    Recovered(Vec<String>),
    /// The body could not be represented as default statements; the string says why.
    Rejected(String),
}

/// Sentinel characters the body renderer uses for unresolved / argument-mismatch / RVO-default /
/// const-store markers. None of them may survive into source.
const SENTINELS: [char; 4] = ['\u{1}', '\u{2}', '\u{3}', '\u{4}'];

/// Statement bound for one initializer, overridable per run through
/// `GORE_AS_MAX_DEFAULT_STATEMENTS`. The default covers the whole shipped corpus — the largest
/// body is the main map's worldpoint table at ~190k statements.
const MAX_STATEMENTS: usize = 200_000;

/// Recover the default statements from a fully rendered `__InitDefaults` method — signature line,
/// braces, hoisted local declarations and all, exactly as the function emitter produced it.
pub(crate) fn recover(rendered_method: &str) -> DefaultsRecovery {
    if rendered_method.contains("body not fully recovered") {
        return DefaultsRecovery::Rejected("body was stubbed".into());
    }
    let mut statements = match body_statements(rendered_method) {
        Ok(v) => v,
        Err(reason) => return DefaultsRecovery::Rejected(reason),
    };
    if let Err(reason) = drop_trailing_return(&mut statements) {
        return DefaultsRecovery::Rejected(reason);
    }
    for statement in &statements {
        if statement.contains(SENTINELS) {
            return DefaultsRecovery::Rejected("body carries an unresolved marker".into());
        }
        if statement.starts_with("//") {
            return DefaultsRecovery::Rejected(format!("body carries a raw opcode: {statement}"));
        }
        if !statement.ends_with(';') {
            return DefaultsRecovery::Rejected(format!("statement is not terminated: {statement}"));
        }
    }
    let max_statements = std::env::var("GORE_AS_MAX_DEFAULT_STATEMENTS")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(MAX_STATEMENTS);
    if statements.len() > max_statements {
        return DefaultsRecovery::Rejected(format!(
            "body has {} statements, over the {max_statements} recovery bound",
            statements.len()
        ));
    }
    if std::env::var_os("GORE_AS_DEFAULTS_DEBUG").is_some() {
        for statement in &statements {
            eprintln!("[defaults] {statement}");
        }
    }
    if let Err(reason) = fold_temporaries(&mut statements) {
        return DefaultsRecovery::Rejected(reason);
    }
    let mut out = Vec::with_capacity(statements.len());
    for statement in statements {
        let stripped = strip_this(&statement);
        // A bare `this` SURVIVES into the default statement (`m_Collision.OnBeginOverlap
        // .AddUFunction(this, n"Fn");`). It reads as the CDO there, which is what the generated
        // initializer did with it in the first place, and the game compiler accepts it — the
        // whole tree recompiles with these statements in place.
        if let Some(local) = first_temporary(&stripped) {
            return DefaultsRecovery::Rejected(format!(
                "temporary `{local}` did not fold: {stripped}"
            ));
        }
        if let Some(operator) = explicit_operator_call(&stripped) {
            return DefaultsRecovery::Rejected(format!(
                "statement calls the operator overload `{operator}` explicitly: {stripped}"
            ));
        }
        out.push(stripped);
    }
    DefaultsRecovery::Recovered(out)
}

/// The statement lines of a rendered method: everything between the outermost braces, minus the
/// hoisted local declarations. A nested block means control flow, which a default statement
/// cannot express — the corpus has exactly one such initializer, and it is rejected here.
fn body_statements(rendered_method: &str) -> Result<Vec<String>, String> {
    let lines: Vec<&str> = rendered_method.lines().collect();
    let open = lines
        .iter()
        .position(|l| l.trim() == "{")
        .ok_or_else(|| "rendered method has no body".to_string())?;
    let close = lines
        .iter()
        .rposition(|l| l.trim() == "}")
        .ok_or_else(|| "rendered method has no body end".to_string())?;
    if close <= open {
        return Err("rendered method body is inverted".into());
    }
    let mut out = Vec::new();
    for line in &lines[open + 1..close] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains('{') || trimmed.contains('}') {
            return Err(format!("body contains a nested block: {trimmed}"));
        }
        if is_local_declaration(trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}

/// `int local_1;` / `TSubclassOf<UAttributeSet> local_6;` — a hoisted temporary declaration the
/// function emitter writes above the body. Never a statement: a statement always carries `=`,
/// `(` or `.`, and a declaration's last token is the temporary's name.
fn is_local_declaration(line: &str) -> bool {
    let Some(head) = line.strip_suffix(';') else {
        return false;
    };
    if head.contains('=') || head.contains('(') || head.contains('.') {
        return false;
    }
    let mut tokens = head.split_whitespace();
    let first = tokens.next();
    let last = tokens.last();
    match (first, last) {
        (Some(_), Some(name)) => is_temporary_name(name),
        _ => false,
    }
}

fn drop_trailing_return(statements: &mut Vec<String>) -> Result<(), String> {
    match statements.last().map(String::as_str) {
        Some("return;") => {
            statements.pop();
            Ok(())
        }
        Some(other) => Err(format!("body does not end in a plain return: {other}")),
        None => Err("body is empty".into()),
    }
}

/// A compiler temporary: the body renderer names frame slots `local_<frame offset>`.
/// `TSubclassOf<UGA_Spell> local_6` — a declaration WITH an initializer defines the temporary
/// exactly as a bare assignment does. The emitter prefers this form for value types (a bare
/// declaration would ask for a default constructor the base cache may not have), so a defaults
/// body reaches recovery full of them.
fn declares_temporary(lhs: &str) -> bool {
    let mut tokens = lhs.split_whitespace();
    let Some(first) = tokens.next() else {
        return false;
    };
    let Some(name) = tokens.last() else {
        return false;
    };
    !first.contains(['.', '(', '[']) && !is_temporary_name(first) && is_temporary_name(name)
}

fn is_temporary_name(name: &str) -> bool {
    // `local_4` and the emitter's fresh name for a re-used slot, `local_4_2`.
    name.strip_prefix("local_").is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .split('_')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Fold every `local_N = <expr>;` back into the single expression that consumes it, so the body
/// becomes the sequence of source-level default statements that produced it. Runs to a fixpoint;
/// a temporary that cannot be folded without changing evaluation is a rejection, never a guess.
use std::collections::HashMap;

/// Where each compiler temporary appears, so the fold does not have to re-scan the whole body for
/// every one of them. Statements are only ever rewritten in place until the fold ends, so a
/// statement's position is a stable key.
struct TemporaryIndex {
    /// name -> (statement index, occurrence count), ascending by index.
    occurrences: HashMap<String, Vec<(usize, usize)>>,
    /// name -> statement indices that DEFINE it, ascending.
    definitions: HashMap<String, Vec<usize>>,
}

impl TemporaryIndex {
    fn build(statements: &[String]) -> Self {
        let mut occurrences: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut definitions: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, statement) in statements.iter().enumerate() {
            for (name, count) in temporary_occurrences(statement) {
                occurrences.entry(name).or_default().push((index, count));
            }
            if let Some((name, _)) = temporary_definition(statement) {
                definitions.entry(name.to_owned()).or_default().push(index);
            }
        }
        Self {
            occurrences,
            definitions,
        }
    }

    /// The next statement that redefines `name` after `from`, or `len`.
    fn next_definition(&self, name: &str, from: usize, len: usize) -> usize {
        self.definitions
            .get(name)
            .and_then(|indices| indices.iter().find(|index| **index > from).copied())
            .unwrap_or(len)
    }

    /// Occurrences of `name` strictly between `from` and `until`.
    fn uses_between(&self, name: &str, from: usize, until: usize) -> Vec<(usize, usize)> {
        self.occurrences
            .get(name)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(index, _)| *index > from && *index < until)
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Record that a statement changed from `before` to `after` at `index`. Folding a value in
    /// carries that value's own temporaries along, so they have to be registered here.
    fn refresh(&mut self, index: usize, before: &str, after: &str) {
        self.forget(index, before);
        for (name, count) in temporary_occurrences(after) {
            let entries = self.occurrences.entry(name).or_default();
            let at = entries.partition_point(|(other, _)| *other < index);
            entries.insert(at, (index, count));
        }
    }

    /// Drop the occurrences `statement` contributed at `index` — only the names it mentions are
    /// touched, so a body with a hundred thousand temporaries stays linear.
    fn forget(&mut self, index: usize, statement: &str) {
        for (name, _) in temporary_occurrences(statement) {
            if let Some(entries) = self.occurrences.get_mut(&name) {
                entries.retain(|(other, _)| *other != index);
            }
        }
    }
}

/// Every compiler temporary a statement mentions, with how often — string literals excluded.
fn temporary_occurrences(statement: &str) -> Vec<(String, usize)> {
    let mask = mask_literals(statement);
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut index = 0usize;
    while index + 6 <= mask.len() {
        if &mask[index..index + 6] != b"local_" {
            index += 1;
            continue;
        }
        if index > 0 && is_word_byte(mask[index - 1]) {
            index += 1;
            continue;
        }
        let mut end = index;
        while end < mask.len() && is_word_byte(mask[end]) {
            end += 1;
        }
        let word = &statement[index..end];
        if is_temporary_name(word) {
            *counts.entry(word.to_owned()).or_default() += 1;
        }
        index = end.max(index + 1);
    }
    counts.into_iter().collect()
}

fn fold_temporaries(statements: &mut Vec<String>) -> Result<(), String> {
    // One left-to-right pass: folding a definition only rewrites statements AFTER it, so a
    // definition the cursor has passed can never become foldable again. A consumed definition is
    // left as an empty string and dropped at the end — removing it from the middle of the vector
    // instead made the pass quadratic, which is why the machine-generated map tables (200k
    // statements) could not be recovered at all.
    let mut occurrences = TemporaryIndex::build(statements);
    let mut index = 0usize;
    while index < statements.len() {
        let Some((name, value)) = temporary_definition(&statements[index])
            .map(|(name, value)| (name.to_string(), value.to_string()))
        else {
            index += 1;
            continue;
        };
        // Uses are only the ones before the temporary is redefined; a later definition owns the
        // uses after it and is folded when the cursor reaches it.
        let region_end = occurrences.next_definition(&name, index, statements.len());
        let mut uses = occurrences.uses_between(&name, index, region_end);
        // A redefinition may READ the temporary it redefines — `local_4 = local_4 * local_6;`
        // is how the compiler writes an in-place update. That read is a use of THIS definition,
        // and not counting it dropped this definition as a dead store and left the read
        // dangling, which cost the whole module its authored defaults.
        let redefinition_reads = statements
            .get(region_end)
            .and_then(|statement| definition_rhs(statement))
            .map(|rhs| count_word(rhs, &name))
            .unwrap_or(0);
        if redefinition_reads > 0 {
            uses.push((region_end, redefinition_reads));
        }
        let total: usize = uses.iter().map(|(_, count)| *count).sum();
        if total == 0 {
            // A dead store. Dropping it is only safe when the value cannot have an effect —
            // a call could, so its result being unused is a shape we do not understand.
            if value.contains('(') {
                return Err(format!("unused temporary `{name}` holds a call: {value}"));
            }
            occurrences.forget(index, &statements[index]);
            statements[index].clear();
            index += 1;
            continue;
        }
        if total > 1 && !is_literal(&value) {
            return Err(format!(
                "temporary `{name}` is used {total} times but is not a literal: {value}"
            ));
        }
        let replacement = if needs_parentheses(&value) {
            format!("({value})")
        } else {
            value.clone()
        };
        for (offset, _) in &uses {
            // The redefinition keeps its own left-hand side; only its right-hand side reads the
            // value being folded.
            if *offset == region_end {
                let statement = &statements[*offset];
                let (head, rhs) = statement
                    .rsplit_once(" = ")
                    .expect("a definition splits at its assignment");
                let rhs = replace_word(rhs, &name, &replacement);
                let rewritten = format!("{head} = {rhs}");
                occurrences.refresh(*offset, &statements[*offset], &rewritten);
                statements[*offset] = rewritten;
                continue;
            }
            let statement = &statements[*offset];
            // A temporary at the head of a statement is either the RECEIVER of a call — the
            // fluent-builder idiom `CreateNewTauntGroup().AddProperty(…).Set();`, which the
            // compiler spills into one temporary per link and which folds back exactly — or the
            // ROOT OF AN ASSIGNMENT TARGET, where folding would turn an lvalue into a temporary
            // expression and silently drop the write.
            if assignment_target_is_rooted_at(statement, &name) {
                return Err(format!(
                    "temporary `{name}` is assigned through: {statement}"
                ));
            }
            let rewritten = replace_word(statement, &name, &replacement);
            occurrences.refresh(*offset, statement, &rewritten);
            statements[*offset] = rewritten;
        }
        occurrences.forget(index, &statements[index]);
        statements[index].clear();
        index += 1;
    }
    statements.retain(|statement| !statement.is_empty());
    Ok(())
}

/// `local_4 = <expr>;` -> `("local_4", "<expr>")`. Only a WHOLE temporary is a definition:
/// `local_4.Field = x;` writes through one and must stay a statement.
/// The right-hand side of a statement that defines a temporary, if it is one.
fn definition_rhs(statement: &str) -> Option<&str> {
    temporary_definition(statement).map(|(_, rhs)| rhs)
}

fn temporary_definition(statement: &str) -> Option<(&str, &str)> {
    let body = statement.strip_suffix(';')?;
    let (lhs, rhs) = split_top_level_assignment(body)?;
    let lhs = lhs.trim();
    if !is_temporary_name(lhs) && !declares_temporary(lhs) {
        return None;
    }
    let lhs = lhs.split_whitespace().last()?;
    let rhs = rhs.trim();
    if rhs.is_empty() {
        return None;
    }
    Some((lhs, rhs))
}

/// True when the statement assigns THROUGH the named temporary — the temporary is the root of the
/// assignment target (`local_24.m_IsAim = true;`, `local_9[0] = x;`). Folding such a statement
/// would replace an lvalue with a temporary expression and lose the write. A call whose RECEIVER
/// is the temporary (`local_4.Set();`) is not an assignment and folds normally.
fn assignment_target_is_rooted_at(statement: &str, name: &str) -> bool {
    let Some(body) = statement.strip_suffix(';') else {
        return false;
    };
    let Some((lhs, _)) = split_top_level_assignment(body) else {
        return false;
    };
    starts_with_word(lhs.trim(), name)
}

/// Split on the first top-level `=` that is a real assignment (not `==`, `!=`, `<=`, `>=`).
fn split_top_level_assignment(body: &str) -> Option<(&str, &str)> {
    let bytes = body.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    for (index, &byte) in bytes.iter().enumerate() {
        match quote {
            Some(q) => {
                if byte == b'\\' {
                    continue;
                }
                if byte == q {
                    quote = None;
                }
            }
            None => match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                b'=' if depth == 0 => {
                    let previous = index.checked_sub(1).map(|i| bytes[i]);
                    let next = bytes.get(index + 1).copied();
                    if next == Some(b'=')
                        || matches!(
                            previous,
                            Some(b'=' | b'!' | b'<' | b'>' | b'+' | b'-' | b'*' | b'/')
                        )
                    {
                        continue;
                    }
                    return Some((&body[..index], &body[index + 1..]));
                }
                _ => {}
            },
        }
    }
    None
}

/// An expression that can be substituted more than once without changing behaviour.
fn is_literal(value: &str) -> bool {
    if value.starts_with('"') && value.ends_with('"') {
        return true;
    }
    if let Some(rest) = value.strip_prefix("n\"") {
        return rest.ends_with('"');
    }
    if matches!(value, "true" | "false" | "nullptr") {
        return true;
    }
    let numeric = value
        .trim_end_matches(['f', 'd'])
        .trim_start_matches('-')
        .trim_start_matches('+');
    !numeric.is_empty()
        && numeric
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.' || b == b'x' || b.is_ascii_hexdigit())
}

/// True when substituting the expression into a larger one needs protecting brackets — i.e. it
/// carries a top-level space, which is what every binary operator and cast render leaves behind.
fn needs_parentheses(value: &str) -> bool {
    if value.starts_with('(') && value.ends_with(')') {
        return false;
    }
    let bytes = value.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    for &byte in bytes {
        match quote {
            Some(q) => {
                if byte == b'\\' {
                    continue;
                }
                if byte == q {
                    quote = None;
                }
            }
            None => match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                b' ' if depth == 0 => return true,
                _ => {}
            },
        }
    }
    false
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Whole-word occurrences of `word`, ignoring the inside of string literals.
fn word_positions(text: &str, word: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let needle = word.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(q) => {
                if byte == b'\\' {
                    index += 2;
                    continue;
                }
                if byte == q {
                    quote = None;
                }
                index += 1;
            }
            None => {
                if byte == b'"' || byte == b'\'' {
                    quote = Some(byte);
                    index += 1;
                    continue;
                }
                if bytes[index..].starts_with(needle) {
                    let before_ok = index == 0 || !is_word_byte(bytes[index - 1]);
                    let after = index + needle.len();
                    let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
                    if before_ok && after_ok {
                        out.push(index);
                        index = after;
                        continue;
                    }
                }
                index += 1;
            }
        }
    }
    out
}

fn count_word(text: &str, word: &str) -> usize {
    word_positions(text, word).len()
}

fn starts_with_word(text: &str, word: &str) -> bool {
    word_positions(text, word).first() == Some(&0)
}

fn replace_word(text: &str, word: &str, replacement: &str) -> String {
    let positions = word_positions(text, word);
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for start in positions {
        out.push_str(&text[cursor..start]);
        out.push_str(replacement);
        cursor = start + word.len();
    }
    out.push_str(&text[cursor..]);
    out
}

/// Drop every `this.` qualifier. A default statement is compiled against the class default
/// object, and the receiver is implicit; the body renderer spells it out.
fn strip_this(statement: &str) -> String {
    let mut out = statement.to_string();
    loop {
        let positions = word_positions(&out, "this");
        let Some(&start) = positions
            .iter()
            .find(|&&start| out[start..].starts_with("this."))
        else {
            return out;
        };
        out.replace_range(start..start + "this.".len(), "");
    }
}

/// An explicitly written AngelScript operator overload (`a.opOr(b)`, `x.opAssign(y)`), if any.
///
/// Source never spells these out — they are what `|` and `=` compile INTO — so one appearing in a
/// recovered statement proves the body was not reconstructed back to source. It is also a
/// reliable smoke signal for the surrounding statement: these bodies are fluent rule builders
/// whose call arguments the structurer does not fully recover, so the rest of the statement is
/// suspect too. Reject rather than emit a plausible-looking default that means something else.
fn explicit_operator_call(statement: &str) -> Option<String> {
    let mask = mask_literals(statement);
    let mut index = 0usize;
    while index + 4 <= mask.len() {
        if &mask[index..index + 3] != b".op" || !mask[index + 3].is_ascii_uppercase() {
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < mask.len() && is_word_byte(mask[end]) {
            end += 1;
        }
        if mask.get(end) == Some(&b'(') {
            return Some(statement[start..end].to_string());
        }
        index = end.max(index + 1);
    }
    None
}

/// A byte mask of the statement in which every string-literal body is blanked out, so a scan can
/// look for identifiers without matching text inside a quoted literal. Positions line up with the
/// original, and every blanked byte becomes ASCII, so an identifier found in the mask always
/// slices the original at a char boundary.
fn mask_literals(text: &str) -> Vec<u8> {
    let mut out = text.as_bytes().to_vec();
    let mut index = 0usize;
    while index < out.len() {
        let byte = out[index];
        if byte != b'"' && byte != b'\'' {
            index += 1;
            continue;
        }
        let quote = byte;
        index += 1;
        while index < out.len() {
            if out[index] == b'\\' {
                out[index] = b'x';
                if index + 1 < out.len() {
                    out[index + 1] = b'x';
                }
                index += 2;
                continue;
            }
            if out[index] == quote {
                index += 1;
                break;
            }
            out[index] = b'x';
            index += 1;
        }
    }
    out
}

/// The first surviving compiler temporary, if any — `local_N` is a single word, so this scans for
/// the prefix outside string literals and checks the word boundary itself.
fn first_temporary(statement: &str) -> Option<String> {
    let mask = mask_literals(statement);
    let mut index = 0usize;
    while index + 6 <= mask.len() {
        if &mask[index..index + 6] != b"local_" {
            index += 1;
            continue;
        }
        if index > 0 && is_word_byte(mask[index - 1]) {
            index += 1;
            continue;
        }
        let mut end = index;
        while end < mask.len() && is_word_byte(mask[end]) {
            end += 1;
        }
        let word = &statement[index..end];
        if is_temporary_name(word) {
            return Some(word.to_string());
        }
        index = end.max(index + 1);
    }
    None
}

/// Refuse source whose effective declarations depend on compiler preprocessing.
///
/// The safety inventories run on authored source before the standalone frontend. Until they can
/// use that exact frontend configuration, accepting a directive would let disabled declarations
/// satisfy a source-level completeness proof and then disappear from compiled output.
pub(crate) fn reject_preprocessor_directives(source: &str) -> Result<(), String> {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut line = 1usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                line += 1;
                index += 1;
            }
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\n' {
                        line += 1;
                    }
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        closed = true;
                        break;
                    }
                    index += 1;
                }
                if !closed {
                    return Err("source has an unterminated block comment".into());
                }
            }
            quote @ (b'\'' | b'"') => {
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        index += 1;
                        closed = true;
                        break;
                    } else {
                        if bytes[index] == b'\n' {
                            line += 1;
                        }
                        index += 1;
                    }
                }
                if !closed {
                    return Err("source has an unterminated quoted literal".into());
                }
            }
            b'#' => {
                let directive_line = line;
                let start = index;
                index += 1;
                while index < bytes.len() && matches!(bytes[index], b' ' | b'\t') {
                    index += 1;
                }
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                let directive = source[start..index].trim_end();
                return Err(format!(
                    "line {directive_line}: preprocessor directive `{directive}` is unsupported: authored-default coverage is checked before compiler preprocessing"
                ));
            }
            _ => index += 1,
        }
    }
    Ok(())
}

/// The classes whose BODY declares at least one class-scope `default` statement.
///
/// Class scope only: a `default` inside a method body is a switch label and sits at brace depth
/// two or more, so it can never be mistaken for a class default. Comments and quoted literals are
/// skipped; an unterminated one is an error rather than an excuse to treat a class as covered.
///
/// The recompile path needs this: once an authored module declares its own defaults, the compiler
/// regenerates `__InitDefaults` from them, and the byte-exact copy carried from the base cache
/// becomes stale. Skipping that carry is only safe for a class this set contains.
pub(crate) fn classes_with_default_statements(
    source: &str,
) -> Result<std::collections::HashSet<String>, String> {
    reject_preprocessor_directives(source)?;
    let bytes = source.as_bytes();
    let mut index = 0usize;
    // One entry per open brace: the class it opened, or `None` for a namespace or any other
    // block. Raw brace DEPTH is not enough — a class inside `namespace G1R::Conversation` sits
    // one level deeper than one at file scope, and a depth test silently reports it as having
    // no defaults at all.
    let mut blocks: Vec<Option<String>> = Vec::new();
    let mut pending_class: Option<String> = None;
    let mut expect_class_name = false;
    let mut out = std::collections::HashSet::new();
    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        closed = true;
                        break;
                    }
                    index += 1;
                }
                if !closed {
                    return Err("source has an unterminated block comment".into());
                }
            }
            quote @ (b'\'' | b'"') => {
                index += 1;
                let mut closed = false;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == quote {
                        index += 1;
                        closed = true;
                        break;
                    } else {
                        index += 1;
                    }
                }
                if !closed {
                    return Err("source has an unterminated quoted literal".into());
                }
            }
            b'{' => {
                blocks.push(pending_class.take());
                index += 1;
            }
            b'}' => {
                blocks.pop();
                index += 1;
            }
            b';' => {
                // A forward declaration carries no body.
                pending_class = None;
                expect_class_name = false;
                index += 1;
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                let word = &bytes[start..index];
                if matches!(word, b"class" | b"struct") {
                    expect_class_name = true;
                } else if expect_class_name {
                    expect_class_name = false;
                    pending_class = Some(String::from_utf8_lossy(word).into_owned());
                } else if word == b"default" {
                    // Attribute it to the class whose body we are DIRECTLY inside. Anything
                    // deeper is a `default:` switch label in a method body.
                    if let Some(Some(class)) = blocks.last() {
                        out.insert(class.clone());
                    }
                }
            }
            _ => index += 1,
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn method(body: &[&str]) -> String {
        let mut s = String::from("    void __InitDefaults()\n    {\n");
        for line in body {
            s.push_str("        ");
            s.push_str(line);
            s.push('\n');
        }
        s.push_str("    }\n");
        s
    }

    fn recovered(body: &[&str]) -> Vec<String> {
        match recover(&method(body)) {
            DefaultsRecovery::Recovered(v) => v,
            DefaultsRecovery::Rejected(reason) => panic!("unexpected rejection: {reason}"),
        }
    }

    fn rejection(body: &[&str]) -> String {
        match recover(&method(body)) {
            DefaultsRecovery::Recovered(v) => panic!("unexpected recovery: {v:?}"),
            DefaultsRecovery::Rejected(reason) => reason,
        }
    }

    #[test]
    fn plain_member_defaults_lose_the_receiver() {
        assert_eq!(
            recovered(&[
                "this.m_Name = \"ItMw_1H_Sword_Old_01\";",
                "this.m_Value = 10;",
                "return;",
            ]),
            vec![
                "m_Name = \"ItMw_1H_Sword_Old_01\";".to_string(),
                "m_Value = 10;".to_string(),
            ]
        );
    }

    #[test]
    fn bare_calls_are_default_statements_too() {
        assert_eq!(
            recovered(&[
                "this.SetItemType(GameplayTag::Item_Weapon_Sword_OneHand);",
                "return;",
            ]),
            vec!["SetItemType(GameplayTag::Item_Weapon_Sword_OneHand);".to_string()]
        );
    }

    #[test]
    fn a_single_use_temporary_folds_into_its_consumer() {
        assert_eq!(
            recovered(&[
                "float32 local_2;",
                "local_2 = 10.0f;",
                "this.m_DamageBase.Add(GameplayTag::Item_Damage_Physical_Edge, local_2);",
                "return;",
            ]),
            vec!["m_DamageBase.Add(GameplayTag::Item_Damage_Physical_Edge, 10.0f);".to_string()]
        );
    }

    #[test]
    fn a_temporary_chain_folds_all_the_way_down() {
        assert_eq!(
            recovered(&[
                "local_4 = UAttributeSet_Strength::StaticClass();",
                "local_6 = TSubclassOf<UAttributeSet>(local_4);",
                "local_20 = this.GetAttribute(local_6, n\"Strength\");",
                "local_2 = 5.0f;",
                "this.m_RequiredStats.Add(local_20, local_2);",
                "return;",
            ]),
            vec![concat!(
                "m_RequiredStats.Add(",
                "GetAttribute(TSubclassOf<UAttributeSet>(UAttributeSet_Strength::StaticClass()), ",
                "n\"Strength\"), 5.0f);"
            )
            .to_string()]
        );
    }

    #[test]
    fn a_dead_constant_store_is_dropped_but_a_dead_call_is_not() {
        assert_eq!(
            recovered(&["local_2 = 10.0f;", "this.m_Value = 3;", "return;"]),
            vec!["m_Value = 3;".to_string()]
        );
        assert!(
            rejection(&["local_2 = Compute();", "this.m_Value = 3;", "return;"])
                .contains("holds a call")
        );
    }

    #[test]
    fn a_repeated_literal_temporary_folds_into_every_use() {
        assert_eq!(
            recovered(&[
                "local_2 = 1.5f;",
                "this.A = local_2;",
                "this.B = local_2;",
                "return;",
            ]),
            vec!["A = 1.5f;".to_string(), "B = 1.5f;".to_string()]
        );
    }

    #[test]
    fn a_repeated_non_literal_temporary_is_rejected_rather_than_duplicated() {
        assert!(rejection(&[
            "local_2 = Compute();",
            "this.A = local_2;",
            "this.B = local_2;",
            "return;",
        ])
        .contains("used 2 times"));
    }

    #[test]
    fn an_operator_expression_is_parenthesised_when_substituted() {
        assert_eq!(
            recovered(&["local_2 = a + b;", "this.Add(local_2);", "return;"]),
            vec!["Add((a + b));".to_string()]
        );
    }

    #[test]
    fn a_temporary_written_through_is_rejected() {
        assert!(rejection(&[
            "local_24 = FLightValues();",
            "local_24.m_IsAim = true;",
            "return;",
        ])
        .contains("assigned through"));
        // The same shape with a consumer is rejected as well, just by the duplication guard.
        assert!(matches!(
            recover(&method(&[
                "local_24 = FLightValues();",
                "local_24.m_IsAim = true;",
                "this.Values.Add(local_24);",
                "return;",
            ])),
            DefaultsRecovery::Rejected(_)
        ));
    }

    #[test]
    fn a_fluent_builder_chain_folds_back_into_one_statement() {
        assert_eq!(
            recovered(&[
                "local_2 = this.CreateNewTauntGroup();",
                "local_4 = local_2.AddProperty(GameplayTag::Combat_Taunt_InPlace);",
                "local_2 = local_4.AddMontage(this.Biter_Taunt_02_Montage);",
                "local_2.Set();",
                "return;",
            ]),
            vec![concat!(
                "CreateNewTauntGroup()",
                ".AddProperty(GameplayTag::Combat_Taunt_InPlace)",
                ".AddMontage(Biter_Taunt_02_Montage).Set();"
            )
            .to_string()]
        );
    }

    #[test]
    fn a_redefinition_owns_only_the_uses_after_it() {
        assert_eq!(
            recovered(&[
                "local_2 = 10.0f;",
                "this.A = local_2;",
                "local_2 = 5.0f;",
                "this.B = local_2;",
                "return;",
            ]),
            vec!["A = 10.0f;".to_string(), "B = 5.0f;".to_string()]
        );
    }

    #[test]
    fn control_flow_and_raw_opcodes_are_rejected() {
        assert!(rejection(&["if (x)", "{", "this.A = 1;", "}", "return;"]).contains("nested block"));
        assert!(rejection(&["// JMPP w0", "return;"]).contains("raw opcode"));
    }

    #[test]
    fn an_unresolved_marker_is_rejected() {
        assert!(rejection(&["this.A = \u{1}unresolved;", "return;"]).contains("unresolved marker"));
    }

    #[test]
    fn a_stubbed_body_is_rejected() {
        let stub = "    void __InitDefaults()\n    {\n        // body not fully recovered — stub [x]\n    }\n";
        assert!(matches!(recover(stub), DefaultsRecovery::Rejected(_)));
    }

    #[test]
    fn a_missing_return_is_rejected() {
        assert!(rejection(&["this.A = 1;"]).contains("does not end in a plain return"));
    }

    #[test]
    fn an_in_place_update_keeps_the_value_it_reads() {
        // `local_4 = local_4 * local_6;` READS the definition above it; not counting that read
        // dropped the definition as a dead store and left the read dangling.
        assert_eq!(
            recovered(&[
                "local_4 = this.WaterCost;",
                "local_6 = 0.33;",
                "local_4 = local_4 * local_6;",
                "this.WaterCost = local_4;",
                "return;",
            ]),
            vec!["WaterCost = (WaterCost * 0.33);"]
        );
    }

    #[test]
    fn a_bare_this_argument_is_kept() {
        // `this` inside a default statement is the CDO — the same object the generated
        // initializer wrote to — and the game compiler accepts it.
        assert_eq!(
            recovered(&["this.Register(this);", "return;"]),
            vec!["Register(this);"]
        );
    }

    #[test]
    fn class_scope_defaults_are_attributed_to_their_class() {
        let source = concat!(
            "class UA : UBase\n{\n    default m_Value = 1;\n",
            "    void Tick()\n    {\n        switch (x)\n        {\n",
            "            default:\n                break;\n        }\n    }\n}\n",
            "class UB : UBase\n{\n    UB()\n    {\n        return;\n    }\n}\n",
            "struct FC\n{\n    default Flag = true;\n}\n",
        );
        let found = classes_with_default_statements(source).unwrap();
        assert!(found.contains("UA"), "{found:?}");
        assert!(found.contains("FC"), "{found:?}");
        assert!(!found.contains("UB"), "{found:?}");
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn a_class_inside_a_namespace_still_counts() {
        // The emitted tree wraps namespaced declarations in a block, so the class body is one
        // level deeper than at file scope. A raw brace-depth test silently reported those
        // classes as having no defaults, and the recompile then refused the edit.
        let source = concat!(
            "namespace G1R::Conversation",
            "\n{\nclass UChoiceA : UTopic\n{\n    default PriorityRank = 5;\n",
            "    void Tick()\n    {\n        switch (x)\n        {\n            default:\n                break;\n        }\n    }\n}\n",
            "class UChoiceB : UTopic\n{\n    UChoiceB()\n    {\n        return;\n    }\n}\n}\n",
        );
        let found = classes_with_default_statements(source).unwrap();
        assert!(found.contains("UChoiceA"), "{found:?}");
        assert!(!found.contains("UChoiceB"), "{found:?}");
    }

    #[test]
    fn a_default_in_a_comment_or_literal_is_not_a_class_default() {
        let source = concat!(
            "class UA : UBase\n{\n    // default m_Value = 1;\n",
            "    /* default m_Other = 2; */\n",
            "    UA()\n    {\n        m_Name = \"default\";\n    }\n}\n",
        );
        assert!(classes_with_default_statements(source).unwrap().is_empty());
    }

    #[test]
    fn an_unterminated_construct_is_an_error_not_an_empty_answer() {
        assert!(classes_with_default_statements("class UA\n{\n/* open").is_err());
        assert!(classes_with_default_statements("class UA\n{\nx = \"open;\n").is_err());
    }

    #[test]
    fn an_explicit_operator_overload_call_is_rejected() {
        assert!(rejection(&[
            "local_16 = FAssessmentBits(Assessed::AffectedIsSelf);",
            "this.Rules.RequireTrue(local_16.opOr(Assessed::IsPettyFight));",
            "return;",
        ])
        .contains("opOr"));
        // A member merely NAMED like one is not an operator call.
        assert_eq!(
            recovered(&["this.Options.Add(\"opAssign\");", "return;"]),
            vec!["Options.Add(\"opAssign\");".to_string()]
        );
    }

    #[test]
    fn string_literals_are_never_rewritten() {
        assert_eq!(
            recovered(&["this.m_Icon = \"a this. local_1 text\";", "return;"]),
            vec!["m_Icon = \"a this. local_1 text\";".to_string()]
        );
    }
}
