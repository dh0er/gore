//! Game-start reference saves embedded for the inventory-reset feature, one per
//! Resources difficulty level. The bytes are the raw GSAV files; the reset apply
//! decodes them with a locally-constructed codec backend.

/// The Resources difficulty sub-level that determines an actor's start
/// inventory. Ordered/derived from the difficulty picker labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourcesLevel {
    Novice,
    Gothic,
    Hard,
}

/// Map a difficulty label to a level, falling back to `Gothic` for anything
/// unknown/missing (the standard preset).
pub fn resolve_level(label: Option<&str>) -> ResourcesLevel {
    match label {
        Some("Novice") => ResourcesLevel::Novice,
        Some("Hard") => ResourcesLevel::Hard,
        _ => ResourcesLevel::Gothic,
    }
}

/// The embedded start-save bytes for a level.
pub fn start_save_bytes(level: ResourcesLevel) -> &'static [u8] {
    match level {
        ResourcesLevel::Novice => {
            include_bytes!("../assets/start_saves/resources_novice.sav")
        }
        ResourcesLevel::Gothic => {
            include_bytes!("../assets/start_saves/resources_gothic.sav")
        }
        ResourcesLevel::Hard => include_bytes!("../assets/start_saves/resources_hard.sav"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_level_falls_back_to_gothic() {
        assert_eq!(resolve_level(Some("Novice")), ResourcesLevel::Novice);
        assert_eq!(resolve_level(Some("Hard")), ResourcesLevel::Hard);
        assert_eq!(resolve_level(Some("Gothic")), ResourcesLevel::Gothic);
        assert_eq!(resolve_level(None), ResourcesLevel::Gothic);
        assert_eq!(resolve_level(Some("bogus")), ResourcesLevel::Gothic);
    }

    #[test]
    fn every_level_has_nonempty_gsav_bytes() {
        for level in [
            ResourcesLevel::Novice,
            ResourcesLevel::Gothic,
            ResourcesLevel::Hard,
        ] {
            let bytes = start_save_bytes(level);
            assert!(bytes.starts_with(b"GSAV"), "start save must be a GSAV file");
        }
    }
}
