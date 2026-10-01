//! Rename a single cooked Texture2D package without touching its export/bulk payloads.

use anyhow::{Context, Result, ensure};
use retoc::legacy_asset::{
    EObjectDataResourceVersion, EPackageFlags, FLegacyPackageHeader, FObjectImport, FPackageNameMap,
};
use retoc::logging::Log;
use retoc::version::EngineVersion;
use retoc::zen::FPackageIndex;
use std::io::Cursor;

/// Change both the package identity and its top-level Texture2D export name.
///
/// Input is a split, cooked UE5.4 `.uasset` from retoc. The caller must retain
/// `.uexp` and `.ubulk` byte-for-byte and package the result at `target`; changing
/// only the filesystem path does not change the IoStore package identity.
pub fn rename_texture_package(uasset: &[u8], source: &str, target: &str) -> Result<Vec<u8>> {
    let source_leaf = package_leaf(source)?;
    let target_leaf = package_leaf(target)?;
    ensure!(
        !source.eq_ignore_ascii_case(target),
        "texture clone target must differ from source, including case-insensitive package identity"
    );

    let mut header = parse(uasset)?;
    validate(&header, uasset.len(), source, source_leaf)?;
    let old_header_size = i64::from(header.summary.versioning_info.total_header_size);
    let export_offset = header.exports[0].serial_offset;
    let relative_offset = export_offset
        .checked_sub(old_header_size)
        .context("texture export offset subtraction overflow")?;
    ensure!(
        relative_offset >= 0,
        "texture export starts inside the header"
    );
    let export_size = header.exports[0].serial_size;
    ensure!(
        export_size > 0 && export_offset.checked_add(export_size).is_some(),
        "invalid texture export size"
    );

    // Deserialize retains file-absolute offsets. Serialize expects uexp-relative
    // offsets and adds its newly computed header size (retoc legacy_asset.rs).
    header.exports[0].serial_offset = relative_offset;
    let baseline = serialize(&header)?;
    let original_names = header.name_map.copy_raw_names();
    let renamed_names: Vec<_> = original_names
        .iter()
        .map(|name| {
            if name == source {
                target.to_owned()
            } else if name == source_leaf {
                target_leaf.to_owned()
            } else {
                name.clone()
            }
        })
        .collect();
    header.summary.package_name = target.to_owned();
    // Preserve every name index/number used by the unchanged .uexp. Do not use
    // store(), which can merge names or split a numeric suffix into FName.number.
    header.name_map = FPackageNameMap::create_from_names(renamed_names.clone());
    let output = serialize(&header)?;
    let mut checked = parse(&output).context("read back renamed texture header")?;
    validate(&checked, output.len(), target, target_leaf)?;
    ensure!(
        checked.name_map.copy_raw_names() == renamed_names,
        "texture clone changed name-map order"
    );
    ensure!(
        checked.exports[0].object_name.index == header.exports[0].object_name.index
            && checked.exports[0].object_name.number == header.exports[0].object_name.number,
        "texture clone changed export FName identity"
    );
    let new_header_size = i64::from(checked.summary.versioning_info.total_header_size);
    ensure!(
        checked.exports[0].serial_size == export_size
            && checked.exports[0]
                .serial_offset
                .checked_sub(new_header_size)
                == Some(relative_offset),
        "texture clone moved or resized the uexp payload"
    );
    ensure!(
        resource_signature(&checked) == resource_signature(&header),
        "texture clone changed bulk-data resources"
    );

    // Reversing only the intended changes must reproduce the entire canonical
    // header. This includes import/export flags, dependencies, GUID and resources.
    checked.exports[0].serial_offset = relative_offset;
    checked.summary.package_name = source.to_owned();
    checked.name_map = FPackageNameMap::create_from_names(original_names);
    ensure!(
        serialize(&checked)? == baseline,
        "texture clone changed unrelated header metadata"
    );
    Ok(output)
}

fn package_leaf(package: &str) -> Result<&str> {
    let relative = package
        .strip_prefix("/Game/")
        .context("texture package must start with /Game/")?;
    ensure!(
        !relative.is_empty()
            && relative.split('/').all(|segment| {
                !segment.is_empty()
                    && segment
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            }),
        "texture package must contain only nonempty ASCII letter/digit/underscore segments"
    );
    Ok(relative.rsplit('/').next().unwrap())
}

fn parse(bytes: &[u8]) -> Result<FLegacyPackageHeader> {
    FLegacyPackageHeader::deserialize(
        &mut Cursor::new(bytes),
        Some(EngineVersion::UE5_4.package_file_version()),
    )
    .context("parse cooked UE5.4 texture header")
}

fn serialize(header: &FLegacyPackageHeader) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    header
        .serialize(&mut output, None, &Log::no_log())
        .context("serialize renamed texture header")?;
    Ok(output.into_inner())
}

fn import(header: &FLegacyPackageHeader, index: FPackageIndex) -> Result<&FObjectImport> {
    ensure!(
        index.is_import(),
        "Texture2D class must resolve through an import"
    );
    let index = -i64::from(index.index) - 1;
    header
        .imports
        .get(index as usize)
        .context("texture class import index out of range")
}

fn validate(header: &FLegacyPackageHeader, length: usize, package: &str, leaf: &str) -> Result<()> {
    let summary = &header.summary;
    ensure!(
        summary.versioning_info.package_file_version == EngineVersion::UE5_4.package_file_version(),
        "texture clone supports UE5.4 only"
    );
    ensure!(
        summary.has_package_flags(EPackageFlags::Cooked) && summary.is_filter_editor_only(),
        "texture clone requires a cooked, editor-filtered package"
    );
    ensure!(
        summary.versioning_info.total_header_size > 0
            && summary.versioning_info.total_header_size as usize == length,
        "input must be exactly one split uasset header"
    );
    ensure!(
        summary.package_name == package,
        "texture source package mismatch: expected {package}, got {}",
        summary.package_name
    );
    ensure!(
        header.exports.len() == 1 && summary.exports.count == 1,
        "texture clone requires exactly one export"
    );
    ensure!(
        header.cell_imports.is_empty()
            && header.cell_exports.is_empty()
            && summary.cell_imports.count == 0
            && summary.cell_exports.count == 0,
        "texture clone does not support cells"
    );
    ensure!(
        summary.soft_object_paths.count == 0 && summary.world_tile_info_data_offset == 0,
        "texture clone does not support soft object paths or world data"
    );
    ensure!(
        header.data_resource_version.is_none()
            || header.data_resource_version == Some(EObjectDataResourceVersion::Initial),
        "texture clone requires the UE5.4 bulk-resource layout"
    );
    ensure!(
        header
            .data_resources
            .iter()
            .all(|resource| resource.cooked_index.is_none()),
        "cooked-index bulk resources are unsupported"
    );
    let export = &header.exports[0];
    ensure!(
        export.outer_index.is_null() && export.super_index.is_null(),
        "texture export must be top-level and have no superclass export"
    );
    ensure!(
        export.object_name.number == 0 && header.name_map.get(export.object_name)? == leaf,
        "texture export name must exactly match its package leaf without an FName numeric suffix"
    );
    let class = import(header, export.class_index)?;
    ensure!(
        header.name_map.get(class.object_name)? == "Texture2D"
            && header.name_map.get(class.class_name)? == "Class"
            && header.name_map.get(class.class_package)? == "/Script/CoreUObject",
        "export is not an Engine Texture2D class import"
    );
    let engine = import(header, class.outer_index)?;
    ensure!(
        engine.outer_index.is_null()
            && header.name_map.get(engine.object_name)? == "/Script/Engine"
            && header.name_map.get(engine.class_name)? == "Package"
            && header.name_map.get(engine.class_package)? == "/Script/CoreUObject",
        "Texture2D class import is not owned by /Script/Engine"
    );
    // All import FNames must remain unchanged by the intended local identity rename.
    for entry in &header.imports {
        for name in [entry.class_package, entry.class_name, entry.object_name] {
            let resolved = header.name_map.get(name)?;
            ensure!(
                resolved != package && resolved != leaf,
                "texture package/export name is also used by an import"
            );
        }
    }
    Ok(())
}

fn resource_signature(
    header: &FLegacyPackageHeader,
) -> Vec<(u32, Option<u8>, i64, i64, i64, i64, i32, u32)> {
    header
        .data_resources
        .iter()
        .map(|r| {
            (
                r.flags,
                r.cooked_index,
                r.serial_offset,
                r.duplicate_serial_offset,
                r.serial_size,
                r.raw_size,
                r.outer_index.index,
                r.legacy_bulk_data_flags,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use retoc::legacy_asset::{FObjectDataResource, FObjectExport};

    const SOURCE: &str = "/Game/Textures/T_Source_D";

    fn fixture() -> FLegacyPackageHeader {
        let mut header = FLegacyPackageHeader::default();
        header.summary.versioning_info.package_file_version =
            EngineVersion::UE5_4.package_file_version();
        header.summary.versioning_info.is_unversioned = true;
        header.summary.package_name = SOURCE.to_owned();
        header.summary.package_flags = EPackageFlags::Cooked as u32
            | EPackageFlags::FilterEditorOnly as u32
            | EPackageFlags::UsesUnversionedProperties as u32;
        header.name_map = FPackageNameMap::create_from_names(
            [
                "/Script/CoreUObject",
                "Class",
                "Package",
                "/Script/Engine",
                "Texture2D",
                "T_Source_D",
                SOURCE,
                "Unrelated",
            ]
            .map(str::to_owned)
            .to_vec(),
        );
        let mut name = |s| header.name_map.store(s);
        header.imports = vec![
            FObjectImport {
                class_package: name("/Script/CoreUObject"),
                class_name: name("Package"),
                object_name: name("/Script/Engine"),
                ..Default::default()
            },
            FObjectImport {
                class_package: name("/Script/CoreUObject"),
                class_name: name("Class"),
                object_name: name("Texture2D"),
                outer_index: FPackageIndex::create_import(0),
                ..Default::default()
            },
        ];
        header.exports = vec![FObjectExport {
            class_index: FPackageIndex::create_import(1),
            object_name: name("T_Source_D"),
            serial_offset: 19,
            serial_size: 777,
            is_asset: true,
            generate_public_hash: true,
            ..Default::default()
        }];
        header.data_resource_version = Some(EObjectDataResourceVersion::Initial);
        header.data_resources = vec![
            FObjectDataResource {
                serial_offset: 37,
                duplicate_serial_offset: -1,
                serial_size: 128,
                raw_size: 128,
                outer_index: FPackageIndex::create_export(0),
                legacy_bulk_data_flags: 0x40,
                ..Default::default()
            },
            FObjectDataResource {
                serial_offset: 4096,
                duplicate_serial_offset: 8192,
                serial_size: 512,
                raw_size: 1024,
                outer_index: FPackageIndex::create_export(0),
                legacy_bulk_data_flags: 0x100,
                ..Default::default()
            },
        ];
        header
    }

    #[test]
    fn different_length_identity_preserves_payload_offsets_names_and_resources() {
        let input = serialize(&fixture()).unwrap();
        let before = parse(&input).unwrap();
        let target = "/Game/GoreMods/NpcBeardSwitch/LongerDirectory/T_Clone_D";
        let output = rename_texture_package(&input, SOURCE, target).unwrap();
        let after = parse(&output).unwrap();
        assert_ne!(input.len(), output.len());
        assert_eq!(after.summary.package_name, target);
        assert_eq!(
            after.name_map.get(after.exports[0].object_name).unwrap(),
            "T_Clone_D"
        );
        assert_eq!(
            after.exports[0].object_name.index,
            before.exports[0].object_name.index
        );
        assert_eq!(after.exports[0].serial_offset - output.len() as i64, 19);
        assert_eq!(after.exports[0].serial_size, 777);
        assert_eq!(resource_signature(&before), resource_signature(&after));
        assert_eq!(after.name_map.copy_raw_names()[6], target);
        assert_eq!(after.name_map.copy_raw_names()[7], "Unrelated");
        // A second, shorter rename must not accumulate either header size.
        let again = rename_texture_package(&output, target, SOURCE).unwrap();
        assert_eq!(again, input);
    }

    #[test]
    fn rejects_noncanonical_or_same_package_destinations() {
        let input = serialize(&fixture()).unwrap();
        for target in [
            SOURCE,
            "/Game/textures/t_source_d",
            "/Game/",
            "/Game//T_New",
            "/Game/../T_New",
            "/Game/A/./T_New",
            "/Game/A\\T_New",
            "/Game/T_New.T_New",
            "/Game/T New",
            "/Game/T_ä",
            "/Engine/T_New",
        ] {
            assert!(
                rename_texture_package(&input, SOURCE, target).is_err(),
                "accepted {target}"
            );
        }
        assert!(rename_texture_package(&input, "/Game/Other/T_Source_D", "/Game/T_New").is_err());
    }

    #[test]
    fn rejects_wrong_class_extra_export_and_non_top_level_asset() {
        let mut wrong_class = fixture();
        wrong_class.imports[1].object_name = wrong_class.name_map.store("Material");
        let mut multiple = fixture();
        multiple.exports.push(multiple.exports[0].clone());
        let mut nested = fixture();
        nested.exports[0].outer_index = FPackageIndex::create_import(0);
        for header in [wrong_class, multiple, nested] {
            let input = serialize(&header).unwrap();
            assert!(rename_texture_package(&input, SOURCE, "/Game/T_New").is_err());
        }
    }
}
