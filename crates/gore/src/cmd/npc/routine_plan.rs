//! Deterministic daily schedules using the activity paths exercised by the NPC campaign.

use anyhow::{bail, ensure, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Activity {
    Stand,
    Read,
    Drink,
    Sit,
    Sleep,
    Guard,
    Alchemy,
}

impl Activity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stand => "stand",
            Self::Read => "read",
            Self::Drink => "drink",
            Self::Sit => "sit",
            Self::Sleep => "sleep",
            Self::Guard => "guard",
            Self::Alchemy => "alchemy",
        }
    }

    fn state(self, npc_id: &str) -> String {
        match self {
            Self::Stand => format!("UAIState_GoreRoutine_{npc_id}_Stand"),
            Self::Read => format!("UAIState_GoreRoutine_{npc_id}_Read"),
            Self::Drink => format!("UAIState_GoreRoutine_{npc_id}_Drink"),
            Self::Sit => "UAIState_Sit".to_owned(),
            Self::Sleep => "UAIState_Sleep".to_owned(),
            Self::Guard => "UAIState_GuardWatch".to_owned(),
            Self::Alchemy => "UAIState_PotionAlchemy".to_owned(),
        }
    }

    fn radius(self) -> u16 {
        match self {
            Self::Stand | Self::Read | Self::Drink => 100,
            Self::Sit | Self::Sleep | Self::Guard | Self::Alchemy => 150,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phase {
    pub time: String,
    pub activity: Activity,
    pub spot: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub phases: Vec<Phase>,
}

/// Parse a canonical 24-hour clock time, without accepting partial or ambiguous forms.
pub fn parse_time(time: &str) -> Result<u16> {
    let bytes = time.as_bytes();
    ensure!(
        bytes.len() == 5
            && bytes[2] == b':'
            && [bytes[0], bytes[1], bytes[3], bytes[4]]
                .iter()
                .all(u8::is_ascii_digit),
        "routine time must use HH:MM (00:00 through 23:59): {time:?}"
    );
    let hours = u16::from(bytes[0] - b'0') * 10 + u16::from(bytes[1] - b'0');
    let minutes = u16::from(bytes[3] - b'0') * 10 + u16::from(bytes[4] - b'0');
    ensure!(
        hours < 24 && minutes < 60,
        "routine time is out of range: {time}"
    );
    Ok(hours * 60 + minutes)
}

pub(super) fn validate_identifier(value: &str, what: &str) -> Result<()> {
    ensure!(
        value
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
            && value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_'),
        "{what} must be an ASCII identifier: {value:?}"
    );
    Ok(())
}

fn validate_name_literal(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| (0x20..=0x7e).contains(&byte) && byte != b'"' && byte != b'\\'),
        "{what} must be safe ASCII text for an n\"...\" literal: {value:?}"
    );
    Ok(())
}

/// Name of the explicit, non-teleporting helper emitted by [`Plan::render`].
pub fn activation_function(npc_id: &str) -> String {
    format!("GoreApplyRoutine_{npc_id}")
}

impl Plan {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.phases.is_empty(),
            "a routine must contain at least one phase"
        );
        let mut times = BTreeSet::new();
        for phase in &self.phases {
            let time = parse_time(&phase.time)?;
            ensure!(times.insert(time), "duplicate routine time: {}", phase.time);
            validate_name_literal(&phase.spot, "routine spot")?;
        }
        Ok(())
    }

    /// Insert or replace a phase at its clock time, keeping chronological source order.
    pub fn set(&mut self, phase: Phase) -> Result<()> {
        parse_time(&phase.time)?;
        let mut next = self.clone();
        next.phases.retain(|old| old.time != phase.time);
        next.phases.push(phase);
        next.validate()?;
        next.phases.sort_by(|a, b| a.time.cmp(&b.time));
        *self = next;
        Ok(())
    }

    pub fn remove(&mut self, time: &str) -> Result<()> {
        parse_time(time)?;
        self.validate()?;
        let Some(index) = self.phases.iter().position(|phase| phase.time == time) else {
            bail!("no routine phase at {time}");
        };
        ensure!(
            self.phases.len() > 1,
            "cannot remove the last routine phase"
        );
        self.phases.remove(index);
        Ok(())
    }

    /// Render classes and an explicit existing-save activation helper.
    ///
    /// Spot existence, compatible action tags, restrictions and occupancy belong to the caller's
    /// catalog/runtime checks. Read/drink and direct standing do not search for ambient spot tags.
    pub fn render(&self, npc_id: &str) -> Result<String> {
        self.validate()?;
        validate_identifier(npc_id, "NPC id")?;
        let has = |activity| self.phases.iter().any(|phase| phase.activity == activity);
        let mut out = String::new();
        if has(Activity::Stand) {
            // UAIState_TestGotoWP's proven direct-location loop, with explicit authored events.
            out.push_str(&STAND_HELPER.replace("$ID", npc_id));
        }
        if has(Activity::Read) || has(Activity::Drink) {
            // npc-appearance-routine/npc-b-c.as: the user-tested free-activity and exit path.
            out.push_str(&ACTIVITY_HELPER.replace("$ID", npc_id));
            for (activity, suffix, tag) in [
                (Activity::Read, "Read", "Action_Conversation_ReadBook"),
                (Activity::Drink, "Drink", "Action_Conversation_Drink"),
            ] {
                if has(activity) {
                    writeln!(
                        out,
                        "class UAIState_GoreRoutine_{npc_id}_{suffix} : UAIState_GoreRoutine_{npc_id}_Activity\n{{\n    default ActionTag = GameplayTag::{tag};\n}}\n"
                    )?;
                }
            }
        }
        writeln!(
            out,
            "class UDailyRoutine_{npc_id}_Start : UAIState_DailyRoutine_Human\n{{"
        )?;
        out.push_str("    default ScheduleTimeOffsetMinutesMin = 0.0f;\n");
        out.push_str("    default ScheduleTimeOffsetMinutesMax = 0.0f;\n");
        out.push_str("    default TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;\n");
        let mut phases: Vec<_> = self.phases.iter().collect();
        phases.sort_by(|a, b| a.time.cmp(&b.time));
        for phase in phases {
            let time = parse_time(&phase.time)?;
            writeln!(
                out,
                "    default Schedule({}, {}, {}(), n\"{}\", {}.0f, TSubclassOf<UNavArea>(nullptr), nullptr);",
                time / 60,
                time % 60,
                phase.activity.state(npc_id),
                phase.spot,
                phase.activity.radius()
            )?;
        }
        out.push_str("}\n\n");
        writeln!(
            out,
            "// Call explicitly to replace an existing NPC's saved routine; this does not teleport.\n\
             bool {}()\n{{\n    AGothicNPCState Subject = FCharacterUniqueName(n\"{npc_id}\").GetNPCState();\n    if (Subject == nullptr)\n    {{\n        return false;\n    }}\n    ::ExchangeDailyRoutineToClass(Subject, UDailyRoutine_{npc_id}_Start);\n    return true;\n}}",
            activation_function(npc_id)
        )?;
        Ok(out)
    }
}

const STAND_HELPER: &str = r#"class UAIState_GoreRoutine_$ID_Stand : UGothicCharacterSimulateableAIState
{
    default bSupportsSimulatedSteps = true;

    UFUNCTION(BlueprintOverride)
    void OnGracefulExitRequested()
    {
        this.bShouldExitState = true;
        this.StopWaitingAndContinueTask();
    }

    UFUNCTION(BlueprintOverride)
    void DoTask()
    {
        while (!this.bShouldExitState)
        {
            ::GotoPreferredLocation(this.AI);
            if (this.bShouldExitState) { return; }
            this.WaitSeconds(1.0f);
        }
    }
}

"#;

const ACTIVITY_HELPER: &str = r#"class UAIState_GoreRoutine_$ID_Activity : UGothicCharacterSimulateableAIState
{
    default bSupportsSimulatedSteps = true;

    UPROPERTY()
    FGameplayTag ActionTag;

    UFUNCTION(BlueprintOverride)
    void OnGracefulExitRequested()
    {
        this.bShouldExitState = true;
        this.StopWaitingAndContinueTask();
    }

    UFUNCTION(BlueprintOverride)
    void DoTask()
    {
        while (!this.bShouldExitState)
        {
            ::GotoPreferredLocation(this.AI);
            if (this.bShouldExitState) { return; }
            ::TryInteractionWithoutSpot(this.AI, this.ActionTag, 20.0f);
            if (this.bShouldExitState) { return; }
            this.WaitSeconds(2.0f);
        }
    }
}

"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(time: &str, activity: Activity, spot: &str) -> Phase {
        Phase {
            time: time.into(),
            activity,
            spot: spot.into(),
        }
    }

    #[test]
    fn clock_times_are_unambiguous_and_bounded() {
        assert_eq!(parse_time("00:00").unwrap(), 0);
        assert_eq!(parse_time("12:34").unwrap(), 754);
        assert_eq!(parse_time("23:59").unwrap(), 1439);
        for invalid in [
            "",
            "8:00",
            "08:0",
            "24:00",
            "12:60",
            "-1:00",
            "12:00 ",
            "１２:００",
            "aa:bb",
        ] {
            assert!(parse_time(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn upsert_orders_phases_and_invalid_edits_leave_the_plan_intact() {
        let mut plan = Plan::default();
        plan.set(phase("18:00", Activity::Sleep, "Bed")).unwrap();
        plan.set(phase("08:00", Activity::Stand, "WP_Start"))
            .unwrap();
        plan.set(phase("18:00", Activity::Read, "WP_End")).unwrap();
        assert_eq!(plan.phases.len(), 2);
        assert_eq!(plan.phases[0].time, "08:00");
        assert_eq!(plan.phases[1], phase("18:00", Activity::Read, "WP_End"));
        let before = plan.clone();
        assert!(plan
            .set(phase("08:00", Activity::Drink, "bad\"spot"))
            .is_err());
        assert!(plan.remove("12:00").is_err());
        assert_eq!(plan, before);
        plan.remove("08:00").unwrap();
        let before = plan.clone();
        assert!(plan.remove("18:00").is_err());
        assert_eq!(plan, before);
    }

    #[test]
    fn catalog_style_spot_names_with_spaces_are_safe_literals() {
        let mut plan = Plan::default();
        plan.set(phase("12:00", Activity::Stand, "FP_ST_CREATURE ROAMING"))
            .unwrap();
        let source = plan.render("TEST_A").unwrap();
        assert!(source.contains("n\"FP_ST_CREATURE ROAMING\""));
        for unsafe_name in ["", "bad\\spot", "bad\"spot", "bad\nspot"] {
            assert!(Plan {
                phases: vec![phase("12:00", Activity::Stand, unsafe_name)]
            }
            .validate()
            .is_err());
        }
    }

    #[test]
    fn serialized_plans_reject_duplicate_times_and_source_injection() {
        assert!(Plan::default().validate().is_err());
        let duplicate = Plan {
            phases: vec![
                phase("08:00", Activity::Stand, "WP_A"),
                phase("08:00", Activity::Read, "WP_B"),
            ],
        };
        assert!(duplicate.validate().is_err());
        let plan = Plan {
            phases: vec![phase("08:00", Activity::Stand, "WP_A")],
        };
        assert!(plan.render("NPC\"; //").is_err());
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("\"activity\":\"stand\""));
        assert_eq!(serde_json::from_str::<Plan>(&json).unwrap(), plan);
    }

    #[test]
    fn render_preserves_proven_activity_paths_and_authored_event_bindings() {
        let plan = Plan {
            phases: vec![
                phase("00:00", Activity::Stand, "WP_A"),
                phase("03:00", Activity::Read, "WP_A"),
                phase("06:00", Activity::Drink, "WP_B"),
                phase("09:00", Activity::Sit, "Chair"),
                phase("12:00", Activity::Sleep, "Bed"),
                phase("15:00", Activity::Guard, "Watch"),
                phase("18:00", Activity::Alchemy, "Lab"),
            ],
        };
        let source = plan.render("TEST_A").unwrap();
        for state in [
            "UAIState_GoreRoutine_TEST_A_Stand",
            "UAIState_GoreRoutine_TEST_A_Read",
            "UAIState_GoreRoutine_TEST_A_Drink",
            "UAIState_Sit",
            "UAIState_Sleep",
            "UAIState_GuardWatch",
            "UAIState_PotionAlchemy",
        ] {
            assert!(source.contains(&format!("{state}()")), "missing {state}");
        }
        assert_eq!(source.matches("default Schedule(").count(), 7);
        assert_eq!(
            source
                .matches("default bSupportsSimulatedSteps = true;")
                .count(),
            2
        );
        assert_eq!(source.matches("UFUNCTION(BlueprintOverride)").count(), 4);
        assert!(!source.contains("_Implementation"));
        assert!(!source.contains("UAIState_Stand()"));
        assert!(source.contains("GameplayTag::Action_Conversation_ReadBook"));
        assert!(source.contains("GameplayTag::Action_Conversation_Drink"));
        assert!(source.contains("::TryInteractionWithoutSpot(this.AI, this.ActionTag, 20.0f);"));
        assert!(source.contains("ScheduleTimeOffsetMinutesMin = 0.0f;"));
        assert!(source.contains("ScheduleTimeOffsetMinutesMax = 0.0f;"));
        assert!(source.contains("TeleportToCurrentTaskWhen = EDailyRoutineTeleportMode::Never;"));
        assert!(source.contains("if (Subject == nullptr)"));
        assert!(
            source.contains("::ExchangeDailyRoutineToClass(Subject, UDailyRoutine_TEST_A_Start);")
        );
        assert!(source.contains(&format!("bool {}()", activation_function("TEST_A"))));
        assert!(!source.contains("TeleportToWaypoint"));
    }

    #[test]
    fn rendering_orders_imported_phases_and_scopes_only_needed_helpers() {
        let plan = Plan {
            phases: vec![
                phase("18:00", Activity::Sleep, "Bed"),
                phase("06:00", Activity::Read, "WP_A"),
            ],
        };
        let a = plan.render("TEST_A").unwrap();
        let b = plan.render("TEST_B").unwrap();
        assert!(a.find("Schedule(6, 0,").unwrap() < a.find("Schedule(18, 0,").unwrap());
        assert!(!a.contains("_Stand"));
        assert!(!a.contains("_Drink"));
        assert!(!b.contains("TEST_A"));
        assert_eq!(a.replace("TEST_A", "TEST_B"), b);
    }
}
