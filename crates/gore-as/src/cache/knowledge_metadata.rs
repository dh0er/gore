//! Read-only dialog-knowledge metadata extraction from the shipped script cache.
//!
//! Knowledge ids stored in saves are AngelScript class names (`Topic_*`,
//! `Info_*`, `Choice*`). Their human-facing caption is not derivable from the
//! class name, especially for generated numeric ids. The generated
//! `__InitDefaults` method does, however, retain either the exact localization
//! key passed to `LocText` or a literal passed to `FText::FromString`, followed
//! by an assignment to `this.Caption`.

use std::collections::BTreeMap;

use thiserror::Error;

use super::disasm::{disassemble, Instr};
use super::model::parse_modules;
use super::refs::RefResolver;

/// Exact, cache-derived metadata for one dialog-knowledge class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeMetadata {
    /// Literal caption when the generated default uses `FText::FromString`.
    pub caption: Option<String>,
    /// Save/catalog id without the generated AngelScript `U` prefix.
    pub id: String,
    /// Exact localization key assigned to the class's `Caption` default.
    pub loc_key: Option<String>,
    /// Declaring AngelScript module, useful as deterministic context/debug data.
    pub module: String,
}

#[derive(Debug, Error)]
pub enum KnowledgeMetadataError {
    #[error("could not parse script cache: {0}")]
    Parse(String),
    #[error("could not disassemble {module}.{class}::__InitDefaults: {detail}")]
    Disassemble {
        module: String,
        class: String,
        detail: String,
    },
    #[error("knowledge class {id} occurs with conflicting cache metadata")]
    ConflictingClass { id: String },
}

/// Extract exact caption sources for all concrete knowledge classes.
///
/// Classes without one unambiguous, structurally proven Caption assignment are
/// omitted. No caption is guessed from a class id or from unrelated dialog
/// lines in `Act_Implementation`.
pub fn extract_knowledge_metadata(
    cache: &[u8],
) -> Result<Vec<KnowledgeMetadata>, KnowledgeMetadataError> {
    let modules =
        parse_modules(cache).map_err(|error| KnowledgeMetadataError::Parse(error.to_string()))?;
    let refs = RefResolver::build(cache)
        .map_err(|error| KnowledgeMetadataError::Parse(error.to_string()))?;
    let mut by_id = BTreeMap::<String, KnowledgeMetadata>::new();

    for module in &modules {
        for class in &module.classes {
            let Some(id) = knowledge_id(&class.name) else {
                continue;
            };
            let mut initializers = class
                .methods
                .iter()
                .filter(|method| method.name == "__InitDefaults");
            let Some(initializer) = initializers.next() else {
                continue;
            };
            // More than one generated initializer is not a unique source of
            // truth. Leave the class unresolved rather than choosing one.
            if initializers.next().is_some() {
                continue;
            }
            let instructions = disassemble(&initializer.bytecode).map_err(|error| {
                KnowledgeMetadataError::Disassemble {
                    module: module.name.clone(),
                    class: class.name.clone(),
                    detail: error.to_string(),
                }
            })?;
            let Some((global_ptr, caption_kind)) = caption_global(
                &instructions,
                |call| match call.op.name {
                    "CALL" => call
                        .dwords
                        .first()
                        .and_then(|id| refs.func_by_id(*id as i32))
                        .filter(|name| *name == "LocText")
                        .map(|_| CaptionKind::LocKey),
                    "CALLSYS" => call.qwords.first().and_then(|pointer| {
                        let pointer = *pointer as i64;
                        match (
                            refs.func_owner_by_ptr(pointer),
                            refs.func_ns_by_ptr(pointer),
                            refs.func_by_ptr(pointer),
                        ) {
                            (_, _, Some("LocText")) => Some(CaptionKind::LocKey),
                            (Some("FText"), _, Some("FromString"))
                            | (_, Some("FText"), Some("FromString")) => Some(CaptionKind::Literal),
                            _ => None,
                        }
                    }),
                    _ => None,
                },
                |type_id, offset| refs.member(type_id, offset) == Some("Caption"),
                |pointer| refs.global_is_string(pointer),
            ) else {
                continue;
            };
            let Some(loc_key) = refs.global_by_ptr(global_ptr) else {
                continue;
            };
            if loc_key.trim().is_empty() {
                continue;
            }
            let metadata = KnowledgeMetadata {
                caption: (caption_kind == CaptionKind::Literal).then(|| loc_key.to_owned()),
                id: id.to_owned(),
                loc_key: (caption_kind == CaptionKind::LocKey).then(|| loc_key.to_owned()),
                module: module.name.clone(),
            };
            if let Some(previous) = by_id.insert(id.to_owned(), metadata.clone()) {
                if previous != metadata {
                    return Err(KnowledgeMetadataError::ConflictingClass { id: id.to_owned() });
                }
            }
        }
    }

    Ok(by_id.into_values().collect())
}

fn knowledge_id(class_name: &str) -> Option<&str> {
    let id = class_name.strip_prefix('U')?;
    if id.starts_with("Topic_") || id.starts_with("Info_") {
        return Some(id);
    }
    let rest = id.strip_prefix("Choice")?;
    (!rest.is_empty()
        && rest
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_'))
    .then_some(id)
}

/// Recognize the generated, exact Caption assignment:
///
/// `PGA <string>; PSF <out>; CALL <caption ctor>; PSF <same out>; PshVPtr this;
///  ADDSi this.Caption; CALLSYS <FText copy>`
///
/// Returning a pointer requires exactly one candidate in the method. The
/// predicates keep the matcher independently unit-testable while production
/// callers resolve all identities through the cache's reference tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CaptionKind {
    Literal,
    LocKey,
}

fn caption_global(
    instructions: &[Instr],
    caption_kind: impl Fn(&Instr) -> Option<CaptionKind>,
    is_caption: impl Fn(i32, i32) -> bool,
    is_string_global: impl Fn(i64) -> bool,
) -> Option<(i64, CaptionKind)> {
    let mut candidates = Vec::new();
    for window in instructions.windows(7) {
        let [global, out_a, call, out_b, this_ptr, member, copy] = window else {
            unreachable!("windows(7) always yields seven instructions")
        };
        if global.op.name != "PGA"
            || out_a.op.name != "PSF"
            || !matches!(call.op.name, "CALL" | "CALLSYS")
            || out_b.op.name != "PSF"
            || this_ptr.op.name != "PshVPtr"
            || member.op.name != "ADDSi"
            || copy.op.name != "CALLSYS"
        {
            continue;
        }
        let (Some(&pointer), Some(&out_slot_a), Some(&out_slot_b)) = (
            global.qwords.first(),
            out_a.words.first(),
            out_b.words.first(),
        ) else {
            continue;
        };
        let (Some(&this_slot), Some(&member_offset), Some(&owner_type_id)) = (
            this_ptr.words.first(),
            member.words.first(),
            member.dwords.first(),
        ) else {
            continue;
        };
        let pointer = pointer as i64;
        let Some(kind) = caption_kind(call) else {
            continue;
        };
        if out_slot_a == out_slot_b
            && this_slot == 0
            && is_caption(owner_type_id as i32, member_offset as i32)
            && is_string_global(pointer)
        {
            candidates.push((pointer, kind));
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::isa::OPCODES;

    fn instruction(name: &str, words: &[u16], dwords: &[u32], qwords: &[u64]) -> Instr {
        Instr {
            offset_dw: 0,
            op: OPCODES
                .iter()
                .find(|opcode| opcode.name == name)
                .expect("known opcode"),
            words: words.to_vec(),
            dwords: dwords.to_vec(),
            qwords: qwords.to_vec(),
        }
    }

    fn caption_window(pointer: u64, function_id: u32, member_offset: u16) -> Vec<Instr> {
        vec![
            instruction("PGA", &[], &[], &[pointer]),
            instruction("PSF", &[6], &[], &[]),
            instruction("CALL", &[], &[function_id], &[]),
            instruction("PSF", &[6], &[], &[]),
            instruction("PshVPtr", &[0], &[], &[]),
            instruction("ADDSi", &[member_offset], &[0x40012be], &[]),
            instruction("CALLSYS", &[], &[], &[0x205070a4d80]),
        ]
    }

    fn resolve(instructions: &[Instr]) -> Option<i64> {
        caption_global(
            instructions,
            |call| {
                (call.op.name == "CALL" && call.dwords.first() == Some(&0x302c4))
                    .then_some(CaptionKind::LocKey)
            },
            |type_id, offset| type_id == 0x40012be && offset == 96,
            |pointer| pointer == 0x205558b80c0,
        )
        .map(|(pointer, _)| pointer)
    }

    #[test]
    fn recognizes_exact_loc_text_caption_assignment() {
        assert_eq!(
            resolve(&caption_window(0x205558b80c0, 0x302c4, 96)),
            Some(0x205558b80c0)
        );
    }

    #[test]
    fn recognizes_native_loc_text_caption_assignment() {
        let mut instructions = caption_window(0x205558b80c0, 0x302c4, 96);
        instructions[2] = instruction("CALLSYS", &[], &[], &[0x20514d8fac0]);
        assert_eq!(
            caption_global(
                &instructions,
                |call| {
                    (call.op.name == "CALLSYS" && call.qwords.first() == Some(&0x20514d8fac0))
                        .then_some(CaptionKind::Literal)
                },
                |type_id, offset| type_id == 0x40012be && offset == 96,
                |pointer| pointer == 0x205558b80c0,
            ),
            Some((0x205558b80c0, CaptionKind::Literal))
        );
    }

    #[test]
    fn rejects_wrong_function_or_member() {
        assert_eq!(resolve(&caption_window(0x205558b80c0, 0x111, 96)), None);
        assert_eq!(resolve(&caption_window(0x205558b80c0, 0x302c4, 120)), None);
    }

    #[test]
    fn rejects_ambiguous_caption_sources() {
        let mut instructions = caption_window(0x205558b80c0, 0x302c4, 96);
        instructions.extend(caption_window(0x205558b80d0, 0x302c4, 96));
        assert_eq!(
            caption_global(
                &instructions,
                |call| {
                    (call.op.name == "CALL" && call.dwords.first() == Some(&0x302c4))
                        .then_some(CaptionKind::LocKey)
                },
                |type_id, offset| type_id == 0x40012be && offset == 96,
                |_| true,
            ),
            None
        );
    }

    #[test]
    fn accepts_only_concrete_knowledge_class_names() {
        assert_eq!(knowledge_id("UTopic_Jan_148468"), Some("Topic_Jan_148468"));
        assert_eq!(knowledge_id("UInfo_Whatslife"), Some("Info_Whatslife"));
        assert_eq!(knowledge_id("UChoice66972"), Some("Choice66972"));
        assert_eq!(knowledge_id("UChoice"), None);
        assert_eq!(knowledge_id("UQuest_OldCamp"), None);
    }
}
