//! The game's loose story images.
//!
//! Not everything the game draws lives in the IoStore container. The glossary
//! portraits, the tutorial pictures, the writings and the loading-screen art are
//! plain PNGs under `G1R/Story/Conversation/images`, next to the localization
//! cache and the voice-over archives — the same Alkimia story-file layout. A
//! container scan will never find them, which is exactly why they looked missing.

use std::path::{Path, PathBuf};

use crate::error::{Result, TexError};

/// Where the loose story images live, relative to the game root.
pub const STORY_IMAGE_DIRECTORY: &str = "G1R/Story/Conversation/images";

/// One loose image file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoryImage {
    /// Path below [`STORY_IMAGE_DIRECTORY`], with `/` separators — the stable
    /// name, independent of where the game is installed.
    pub relative_path: String,
    /// The file itself.
    pub path: PathBuf,
    pub byte_length: u64,
}

/// Every loose story image in the installation, sorted by [`StoryImage::relative_path`].
///
/// `filter` keeps only the entries whose relative path contains it, matched
/// case-insensitively. An installation without the folder yields an empty list
/// rather than an error: an older build simply had none.
pub fn list_story_images(game_root: &Path, filter: Option<&str>) -> Result<Vec<StoryImage>> {
    let root = game_root.join(STORY_IMAGE_DIRECTORY.replace('/', std::path::MAIN_SEPARATOR_STR));
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let needle = filter.map(str::to_lowercase);
    let mut out = Vec::new();
    collect(&root, &root, needle.as_deref(), &mut out)?;
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(out)
}

fn collect(
    root: &Path,
    directory: &Path,
    filter: Option<&str>,
    out: &mut Vec<StoryImage>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        // Read the entry's own metadata, not the target's: a symlinked tree
        // could otherwise walk out of the installation or loop forever.
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect(root, &path, filter, out)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| {
                TexError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "story image escaped its own directory",
                ))
            })?
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(filter) = filter {
            if !relative.to_lowercase().contains(filter) {
                continue;
            }
        }
        out.push(StoryImage {
            relative_path: relative,
            path,
            byte_length: metadata.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn lists_every_image_below_the_story_folder_sorted_and_filtered() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("G1R/Story/Conversation/images");
        write(&root.join("Glossary/Characters/T_GlossaryImage_Diego_S.png"), b"a");
        write(&root.join("Glossary/Creatures/T_GlossaryImage_Biter_M.png"), b"bb");
        write(&root.join("Tutorials/T_Tutorial_Fight.png"), b"ccc");
        write(&root.join("MAP_WORLD.jpg"), b"dddd");

        let all = list_story_images(temp.path(), None).unwrap();
        assert_eq!(
            all.iter()
                .map(|image| image.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Glossary/Characters/T_GlossaryImage_Diego_S.png",
                "Glossary/Creatures/T_GlossaryImage_Biter_M.png",
                "MAP_WORLD.jpg",
                "Tutorials/T_Tutorial_Fight.png",
            ]
        );
        assert_eq!(all[0].byte_length, 1);
        assert!(all[0].path.is_file());

        // The filter matches the relative path, case-insensitively.
        let creatures = list_story_images(temp.path(), Some("creatures")).unwrap();
        assert_eq!(creatures.len(), 1);
        assert_eq!(creatures[0].byte_length, 2);
        assert!(list_story_images(temp.path(), Some("nothing")).unwrap().is_empty());
    }

    #[test]
    fn an_installation_without_the_folder_lists_nothing_instead_of_failing() {
        let temp = tempfile::tempdir().unwrap();
        assert!(list_story_images(temp.path(), None).unwrap().is_empty());
    }

    #[test]
    #[ignore = "requires a local Gothic 1 Remake installation"]
    fn the_real_installation_carries_the_glossary_artwork() {
        let game = std::env::var_os("GORE_REAL_GAME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake")
            });
        let images = list_story_images(&game, None).unwrap();
        assert!(images.len() > 500, "found only {}", images.len());
        for folder in [
            "Glossary/Characters/",
            "Glossary/Creatures/",
            "Glossary/Locations/",
            "Tutorials/",
            "Writings/",
        ] {
            assert!(
                images
                    .iter()
                    .any(|image| image.relative_path.starts_with(folder)),
                "no image under {folder}"
            );
        }
    }
}
