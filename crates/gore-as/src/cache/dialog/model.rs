//! Closed data model for the dialog tree of one conversation.
//!
//! Every field here is something the shipping script cache states directly. The model has no
//! room for a guess: a construct the extractor recognizes becomes a typed node, a construct it
//! does not recognize becomes [`StepKind::Call`] carrying the resolved symbol name, and anything
//! that could not be resolved at all is counted in [`Coverage`]. A reader must be able to tell
//! the difference between "the game does not do this" and "the tool did not model this".

use serde::{Deserialize, Serialize};

/// Every conversation the cache declares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogGraph {
    pub conversations: Vec<Conversation>,
}

/// One conversation module: its participants, its topics, and how they nest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    /// AngelScript module name, e.g. `Story.G1R.Conversation.Conversation_OM_STT_VIPER_302`.
    pub module: String,
    /// The module's dialog-topic base class, e.g. `UTopic_Hero__OM_STT_VIPER_302`. `None` for a
    /// conversation that declares only character settings and no topics at all.
    pub root_class: Option<String>,
    /// Participants parsed out of the base class name, e.g. `["Hero", "OM_STT_VIPER_302"]`.
    pub participants: Vec<String>,
    /// Every topic in the module, ordered by class name.
    pub topics: Vec<Topic>,
    /// Topic classes that are not reached through any `Subdialog`, ordered by class name.
    pub roots: Vec<String>,
    pub coverage: Coverage,
}

impl Conversation {
    /// The non-hero participants, which is what a person searches by.
    pub fn npc_participants(&self) -> impl Iterator<Item = &str> {
        self.participants
            .iter()
            .map(String::as_str)
            .filter(|name| !name.eq_ignore_ascii_case("Hero"))
    }

    pub fn topic(&self, class: &str) -> Option<&Topic> {
        self.topics.iter().find(|topic| topic.class == class)
    }
}

/// One dialog option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Topic {
    /// AngelScript class name including the generated `U` prefix.
    pub class: String,
    /// Direct base class, normally the conversation's root class.
    pub super_class: Option<String>,
    /// The menu entry's text source.
    pub caption: Caption,
    /// `default PriorityRank`, which orders the options in the menu. Ascending: sub-topics carry
    /// `0` and the exit option almost always carries the highest observed rank. `None` where the
    /// class leaves it at the engine default.
    pub priority: Option<i64>,
    pub flags: TopicFlags,
    /// Declarative `Rules.*` from the generated defaults, in declaration order.
    pub rules: Vec<Rule>,
    /// The remaining generated defaults, in declaration order.
    pub settings: Vec<Setting>,
    pub visibility: Visibility,
    /// The body of `Act_Implementation`, in bytecode order.
    pub act: Vec<Step>,
}

/// Where the option's menu text comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Caption {
    /// `default Caption = LocText("KEY")` — the normal, translatable form.
    LocKey { key: String },
    /// `default Caption = FText::FromString("...")` — an untranslated literal.
    Literal { text: String },
    /// The class declares no caption, or none this extractor can prove.
    Unresolved,
}

impl Caption {
    pub fn loc_key(&self) -> Option<&str> {
        match self {
            Caption::LocKey { key } => Some(key),
            _ => None,
        }
    }
}

/// Boolean and character defaults that decide where a topic can appear.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TopicFlags {
    /// `default bIsSubTopic = true`: the option belongs to a sub-menu, not the root menu.
    pub is_sub_topic: bool,
    /// `default bIsAmbientTopic = true`: played without the player choosing it.
    pub is_ambient: bool,
    /// `default bIsFollowupTopic = true`: offered directly after its predecessor.
    pub is_followup: bool,
    /// `default ForCharacter = n"..."`: the participant this topic belongs to.
    pub for_character: Option<String>,
    /// `default WithCharacter = n"..."`: the other participant of an NPC-to-NPC topic.
    pub with_character: Option<String>,
}

/// One `Rules.*` entry from the generated defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub kind: RuleKind,
    pub args: Vec<Arg>,
}

/// The closed `Rules.*` vocabulary of the shipping cache. `Other` keeps an unrecognized rule
/// visible instead of dropping it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum RuleKind {
    /// Hide once the character knows the referenced topic — the usual "ask only once".
    HideIfKnows,
    /// Same, addressed by knowledge id rather than class.
    HideIfKnowsId,
    /// Show only once the character knows the referenced topic — the parent/child edge of the
    /// knowledge graph.
    AllowIfCharacterHasKnowledgeOf,
    /// Same, addressed by knowledge id rather than class.
    AllowIfCharacterHasKnowledgeOfId,
    /// Require that a character has heard a specific line.
    RequireCharacterHasListenedTo,
    /// Require that a character has *not* heard a specific line.
    RequireCharacterHasNotListenedTo,
    /// Require a character near a named waypoint, within a radius.
    RequireCharacterCloseToWaypoint,
    /// `Rules.Add(...)`: a composed rule object.
    Add,
    Other {
        name: String,
    },
}

/// One generated default that is neither the caption, a flag, nor a rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    /// Assignment target as written, e.g. `ForceSettings.ApproachBy` or `JustOne.Add`.
    pub target: String,
    pub args: Vec<Arg>,
}

/// Whether the topic overrides `IsVisible_Implementation`, and what that override checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Visibility {
    /// No override: visibility is decided by [`Topic::rules`] alone.
    Always,
    /// An override exists. `checks` lists the calls it makes, in bytecode order; the boolean
    /// algebra between them is deliberately not reconstructed.
    Scripted { checks: Vec<Check> },
}

/// One thing a visibility override looks at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub source: CheckSource,
    pub name: String,
    pub args: Vec<Arg>,
    /// True when the check sits behind a conditional branch inside the override.
    pub conditional: bool,
}

/// Whether a check calls something or reads state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckSource {
    Call,
    Field,
}

/// One statement of `Act_Implementation`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub guard: Guard,
    pub kind: StepKind,
}

/// Whether a step always runs when the topic is chosen.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Guard {
    /// The step's basic block is only reachable through a conditional branch.
    pub conditional: bool,
    /// Names of calls made by the branches that decide whether this step runs. Best effort and
    /// unordered with respect to the boolean algebra; useful for reading, not for evaluation.
    pub hints: Vec<String>,
}

impl Guard {
    pub fn unconditional() -> Self {
        Self::default()
    }
}

/// What a step does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum StepKind {
    /// A spoken line.
    Say {
        /// The accessor that produced the speaker, e.g. `GetHero` or `GetViper`, without its
        /// `Get` prefix where one is present.
        speaker: Option<String>,
        /// Localization key of the spoken text.
        loc_key: Option<String>,
        /// Facial/gesture expression tag, e.g. `Expression_Neutral`.
        expression: Option<String>,
    },
    /// Opens a sub-menu offering `children`, in argument order.
    Subdialog { children: Vec<String> },
    /// Returns to the menu this topic was chosen from.
    ReturnToLastSelection,
    /// Ends the conversation.
    EndConversation,
    /// Any other call the topic makes: quest and knowledge effects, item transfers, routine
    /// changes, and everything this extractor has no typed node for.
    Call { name: String, args: Vec<Arg> },
}

/// A resolved call argument. Unresolvable operands stay [`Arg::Opaque`] rather than being
/// rendered as a plausible-looking value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "arg", rename_all = "snake_case")]
pub enum Arg {
    /// A class reference, e.g. from `UTopic_X::StaticClass()`.
    Class {
        name: String,
    },
    /// An `n"..."` FName literal.
    Name {
        value: String,
    },
    /// A reference to a named global, e.g. `GameplayTag::Expression_Neutral`.
    Symbol {
        name: String,
    },
    /// A string literal.
    Text {
        value: String,
    },
    Int {
        value: i64,
    },
    Float {
        value: f32,
    },
    /// A null topic/class slot, which the compiler emits to pad fixed-arity calls.
    Null,
    Opaque,
}

/// What the extractor could and could not model, per conversation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Coverage {
    /// Topics whose generated defaults were read.
    pub topics: usize,
    /// Chooseable topics with no provable caption. Ambient topics are excluded: having no menu
    /// entry is what makes them ambient.
    pub topics_without_caption: usize,
    /// Steps emitted across all `Act_Implementation` bodies.
    pub steps: usize,
    /// Steps that got a typed node rather than [`StepKind::Call`].
    pub steps_typed: usize,
    /// `Say` steps missing a speaker or a localization key.
    pub says_incomplete: usize,
    /// Calls suppressed as instrumentation or as part of a recognized composite.
    pub calls_suppressed: usize,
    /// Call sites whose symbol could not be resolved at all.
    pub calls_unresolved: usize,
    /// Topics that declare `bIsSubTopic` but are reached by no `Subdialog`, and topics reached
    /// by a `Subdialog` without declaring it.
    pub sub_topic_flag_disagreements: usize,
    /// Ambient topics, which play without being chosen and therefore carry no menu caption.
    pub ambient_topics: usize,
    /// `Subdialog` children that name a class this module does not declare.
    pub dangling_children: usize,
}

impl Coverage {
    /// Steps that only carry a resolved symbol name.
    pub fn steps_untyped(&self) -> usize {
        self.steps.saturating_sub(self.steps_typed)
    }
}
