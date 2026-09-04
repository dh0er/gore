//! `default`-Statements aus emittiertem Klassenquelltext lesen.
//!
//! Der Emitter schreibt jede Klassen-Default als eigene Zeile `    default <lhs> = <rhs>;` oder
//! als Aufruf `    default <call>(...);`. Diese Datei kennt nur diese Form und rät nie: was sie
//! nicht als Zuweisung erkennt, gibt sie unverändert als Aufruf zurück.

/// Eine Klasse, wie sie im emittierten Quelltext steht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedClass {
    pub name: String,
    pub super_class: Option<String>,
    /// `default a = b;` als (a, b), in Quelltextreihenfolge.
    pub assignments: Vec<(String, String)>,
    /// `default f(...);` unverändert, ohne führendes `default ` und ohne Semikolon.
    pub calls: Vec<String>,
}

/// Jede Klasse eines emittierten Moduls.
pub fn parse_classes(source: &str) -> Vec<EmittedClass> {
    let mut out: Vec<EmittedClass> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("class ") {
            let (name, super_class) = match rest.split_once(':') {
                Some((name, base)) => (name.trim(), Some(base.trim().to_string())),
                None => (rest.trim().trim_end_matches('{').trim(), None),
            };
            out.push(EmittedClass {
                name: name.to_string(),
                super_class,
                assignments: Vec::new(),
                calls: Vec::new(),
            });
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("default ") else {
            continue;
        };
        let Some(current) = out.last_mut() else {
            continue;
        };
        let statement = rest.trim().trim_end_matches(';').trim();
        // Ein `=` weist nur zu, wenn es vor der ersten `(` steht: alles hinter dieser Klammer
        // gehört zu den Argumenten eines Aufrufs, wo ein `=` in einem Literal stehen darf.
        let first_paren = statement.find('(').unwrap_or(statement.len());
        match statement[..first_paren].find('=') {
            Some(at) => current.assignments.push((
                statement[..at].trim().to_string(),
                statement[at + 1..].trim().to_string(),
            )),
            None => current.calls.push(statement.to_string()),
        }
    }
    out
}

/// Der Klassenname in `UFoo_Bar::StaticClass()`.
pub fn static_class_target(expression: &str) -> Option<&str> {
    let at = expression.find("::StaticClass()")?;
    let head = &expression[..at];
    let start = head
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or(0, |i| i + 1);
    let name = &head[start..];
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"
class UCharacterDefinition_Human_OC_STT_Diego : UCharacterDefinition_Human_OldCamp_Shadow
{
    default m_LightType = 4;
    default SetAttributeValue("AttributeSet_Health.Health", 540.0f, TSubclassOf<UDifficultySettings>(nullptr));
    default m_CharacterVisualsDefinition = UCharacterVisualsDefinition_Human_OC_STT_Diego::StaticClass();
    default m_UniqueName = n"OC_STT_Diego";

    UCharacterDefinition_Human_OC_STT_Diego()
    {
        super();
        return;
    }
}

class UVisualFeatures_OC_STT_Diego : UCharacterVisualFeaturesDefinition
{
    default DirtSettings.HasDirt = false;
}
"#;

    #[test]
    fn parse_classes_finds_both_classes_with_their_super() {
        let classes = parse_classes(SOURCE);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "UCharacterDefinition_Human_OC_STT_Diego");
        assert_eq!(
            classes[0].super_class.as_deref(),
            Some("UCharacterDefinition_Human_OldCamp_Shadow")
        );
        assert_eq!(classes[1].name, "UVisualFeatures_OC_STT_Diego");
    }

    #[test]
    fn parse_classes_splits_assignments_from_calls() {
        let classes = parse_classes(SOURCE);
        assert_eq!(
            classes[0].assignments,
            vec![
                ("m_LightType".to_string(), "4".to_string()),
                (
                    "m_CharacterVisualsDefinition".to_string(),
                    "UCharacterVisualsDefinition_Human_OC_STT_Diego::StaticClass()".to_string()
                ),
                ("m_UniqueName".to_string(), "n\"OC_STT_Diego\"".to_string()),
            ]
        );
        assert_eq!(classes[0].calls.len(), 1);
        assert!(classes[0].calls[0].starts_with("SetAttributeValue("));
    }

    #[test]
    fn an_equals_inside_a_call_argument_does_not_become_an_assignment() {
        let classes = parse_classes(
            "class UX : UY\n{\n    default Schedule(3, 0, UAIState_SitAround(), n\"IO=1\", 300.0f);\n}\n",
        );
        assert!(classes[0].assignments.is_empty());
        assert_eq!(classes[0].calls.len(), 1);
    }

    #[test]
    fn a_nested_field_path_on_the_left_stays_whole() {
        let classes = parse_classes(SOURCE);
        assert_eq!(
            classes[1].assignments,
            vec![("DirtSettings.HasDirt".to_string(), "false".to_string())]
        );
    }

    #[test]
    fn static_class_target_reads_the_class_out_of_the_expression() {
        assert_eq!(
            static_class_target("UAIAgentConfig_Human_OC_STT_Diego::StaticClass()"),
            Some("UAIAgentConfig_Human_OC_STT_Diego")
        );
        assert_eq!(
            static_class_target("TSubclassOf<UItemDefinition>(UItAm_Arrow::StaticClass())"),
            Some("UItAm_Arrow")
        );
        assert_eq!(static_class_target("4"), None);
        assert_eq!(static_class_target("n\"OC_STT_Diego\""), None);
    }
}
