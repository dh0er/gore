//! Synthetic test-only fixtures shared by downstream crates.
//!
//! This module is deliberately hidden behind the default-off `test-fixtures`
//! feature. It does not grant production authoring or publication authority.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use retoc::iostore_writer::IoStoreWriter;
use retoc::legacy_asset::{
    EPackageFlags, FLegacyPackageFileSummary, FLegacyPackageHeader, FObjectExport, FObjectImport,
    FSerializedAssetBundle,
};
use retoc::logging::Log;
use retoc::name_map::{EMappedNameType, FNameMap};
use retoc::script_objects::{FPackageObjectIndex, FScriptObjectEntry, ZenScriptObjects};
use retoc::version::EngineVersion;
use retoc::zen::FPackageIndex;
use retoc::zen_asset_conversion::build_zen_asset;
use retoc::{build_verse_cell_store, EIoChunkType, FIoChunkId, UEPath, UEPathBuf};

pub const SYNTHETIC_WOLF_ASSET: &str =
    "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps";
pub const SYNTHETIC_WOLF_COOKED_PATH: &str =
    "../../../G1R/Content/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps.uasset";

/// Write a valid UE5.4 Zen package plus ScriptObjects into `utoc`/`ucas`, and
/// the same ScriptObjects into sibling `global.utoc`/`global.ucas`.
pub fn write_valid_zen_fixture(utoc: &Path, components: [f64; 4]) -> Result<()> {
    let version = EngineVersion::UE5_4;
    let mut package = FLegacyPackageHeader::default();
    package.summary.versioning_info.package_file_version = version.package_file_version();
    package.summary.versioning_info.is_unversioned = true;
    package.summary.package_name = SYNTHETIC_WOLF_ASSET.to_owned();
    package.summary.package_flags = EPackageFlags::Cooked as u32
        | EPackageFlags::FilterEditorOnly as u32
        | EPackageFlags::UsesUnversionedProperties as u32;
    let core_uobject = package.name_map.store("/Script/CoreUObject");
    let package_class = package.name_map.store("Package");
    let class_class = package.name_map.store("Class");
    let module_name = package.name_map.store("/Script/G1R");
    let class_name = package.name_map.store("FootstepTag");
    let module_index = package.imports.len();
    package.imports.push(FObjectImport {
        class_package: core_uobject,
        class_name: package_class,
        object_name: module_name,
        ..FObjectImport::default()
    });
    let class_index = package.imports.len();
    package.imports.push(FObjectImport {
        class_package: core_uobject,
        class_name: class_class,
        outer_index: FPackageIndex::create_import(module_index as u32),
        object_name: class_name,
        ..FObjectImport::default()
    });
    let object_name = package.name_map.store("DA_WolfFootsteps");
    let mut exports = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
    exports.extend_from_slice(&[0x00, 0x09]);
    for value in components {
        exports.extend_from_slice(&value.to_le_bytes());
    }
    exports.extend_from_slice(&1_i32.to_le_bytes());
    exports.extend_from_slice(&2_i32.to_le_bytes());
    exports.extend_from_slice(&3_i32.to_le_bytes());
    exports.extend_from_slice(&0_i32.to_le_bytes());
    exports.extend_from_slice(&2_i32.to_le_bytes());
    exports.extend_from_slice(&11_i32.to_le_bytes());
    exports.extend_from_slice(&0_i32.to_le_bytes());
    exports.extend_from_slice(&[0x80, 0x03, 0x01]);
    exports.extend_from_slice(&22_i32.to_le_bytes());
    exports.extend_from_slice(&1_i32.to_le_bytes());
    exports.extend_from_slice(&[0x00, 0x03, 0x01]);
    assert_eq!(exports.len(), 82);
    exports.extend_from_slice(&[0_u8; 4]);
    package.exports.push(FObjectExport {
        class_index: FPackageIndex::create_import(class_index as u32),
        object_name,
        serial_offset: 0,
        serial_size: exports.len() as i64,
        ..FObjectExport::default()
    });
    let mut header = Cursor::new(Vec::new());
    package.serialize(&mut header, None, &Log::no_log())?;
    exports.extend_from_slice(&FLegacyPackageFileSummary::PACKAGE_FILE_TAG.to_le_bytes());
    let bundle = FSerializedAssetBundle {
        asset_file_buffer: header.into_inner(),
        exports_file_buffer: exports,
        ..FSerializedAssetBundle::default()
    };
    let mut global_name_map = FNameMap::create(EMappedNameType::Global);
    let package_name = global_name_map.store("/Script/G1R");
    let imported_class_name = global_name_map.store("FootstepTag");
    let default_object_name = global_name_map.store("Default__FootstepTag");
    let package_index = FPackageObjectIndex::create_script_import("/Script/G1R");
    let imported_class_index = FPackageObjectIndex::create_script_import("/Script/G1R.FootstepTag");
    let default_object_index =
        FPackageObjectIndex::create_script_import("/Script/G1R.Default__FootstepTag");
    let script_entries = vec![
        FScriptObjectEntry {
            object_name: package_name,
            global_index: package_index,
            outer_index: FPackageObjectIndex::create_null(),
            cdo_class_index: FPackageObjectIndex::create_null(),
        },
        FScriptObjectEntry {
            object_name: imported_class_name,
            global_index: imported_class_index,
            outer_index: package_index,
            cdo_class_index: FPackageObjectIndex::create_null(),
        },
        FScriptObjectEntry {
            object_name: default_object_name,
            global_index: default_object_index,
            outer_index: package_index,
            cdo_class_index: imported_class_index,
        },
    ];
    let script_objects_table = ZenScriptObjects {
        global_name_map,
        script_object_lookup: script_entries
            .iter()
            .map(|entry| (entry.global_index, *entry))
            .collect(),
        script_objects: script_entries,
    };
    let mut converted = build_zen_asset(
        bundle,
        &HashMap::new(),
        UEPath::new(SYNTHETIC_WOLF_COOKED_PATH),
        Some(version.package_file_version()),
        version.container_header_version(),
        false,
        Some(Arc::new(script_objects_table.clone())),
        Some(build_verse_cell_store(&Vec::new())),
        &Log::no_log(),
    )?;
    let mut writer = IoStoreWriter::new(
        utoc,
        version.toc_version(),
        Some(version.container_header_version()),
        UEPathBuf::from("../../../"),
    )?;
    converted.write(&mut writer)?;
    let mut script_objects = Vec::new();
    script_objects_table.serialize_new(&mut script_objects)?;
    writer.write_chunk(
        FIoChunkId::create(0, 0, EIoChunkType::ScriptObjects),
        Some(UEPath::new("../../../G1R/Content/ScriptObjects.bin")),
        &script_objects,
    )?;
    writer.finalize()?;

    let mut global_writer = IoStoreWriter::new(
        utoc.with_file_name("global.utoc"),
        version.toc_version(),
        Some(version.container_header_version()),
        UEPathBuf::from("../../../"),
    )?;
    global_writer.write_chunk(
        FIoChunkId::create(0, 0, EIoChunkType::ScriptObjects),
        Some(UEPath::new("../../../G1R/Content/ScriptObjects.bin")),
        &script_objects,
    )?;
    global_writer.finalize()?;
    Ok(())
}

/// Write the exact USMAP schema needed to inspect the synthetic Wolf fixture,
/// including its nested `Vector4` of doubles.
pub fn write_valid_usmap(path: &Path) -> Result<()> {
    let mapping = usmap::Usmap {
        enums: Vec::new(),
        structs: vec![
            usmap::Struct {
                name: "FootstepTag".to_owned(),
                super_struct: None,
                properties: vec![
                    usmap::Property {
                        name: "BoneData".to_owned(),
                        array_dim: 1,
                        index: 0,
                        inner: usmap::PropertyInner::Struct {
                            name: "BoneFeetData".to_owned(),
                        },
                    },
                    usmap::Property {
                        name: "BonesToTrack".to_owned(),
                        array_dim: 1,
                        index: 1,
                        inner: usmap::PropertyInner::Map {
                            key: Box::new(usmap::PropertyInner::Name),
                            value: Box::new(usmap::PropertyInner::Struct {
                                name: "BoneTrackedData".to_owned(),
                            }),
                        },
                    },
                ],
            },
            usmap::Struct {
                name: "BoneFeetData".to_owned(),
                super_struct: None,
                properties: vec![
                    usmap::Property {
                        name: "FeetTextureSize".to_owned(),
                        array_dim: 1,
                        index: 0,
                        inner: usmap::PropertyInner::Struct {
                            name: "Vector4".to_owned(),
                        },
                    },
                    usmap::Property {
                        name: "Diffuse".to_owned(),
                        array_dim: 1,
                        index: 1,
                        inner: usmap::PropertyInner::Object,
                    },
                    usmap::Property {
                        name: "Normal".to_owned(),
                        array_dim: 1,
                        index: 2,
                        inner: usmap::PropertyInner::Object,
                    },
                    usmap::Property {
                        name: "AO".to_owned(),
                        array_dim: 1,
                        index: 3,
                        inner: usmap::PropertyInner::Object,
                    },
                ],
            },
            usmap::Struct {
                name: "BoneTrackedData".to_owned(),
                super_struct: None,
                properties: vec![usmap::Property {
                    name: "InvertX".to_owned(),
                    array_dim: 1,
                    index: 0,
                    inner: usmap::PropertyInner::Bool,
                }],
            },
            usmap::Struct {
                name: "Vector4".to_owned(),
                super_struct: None,
                properties: ["X", "Y", "Z", "W"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, name)| usmap::Property {
                        name: name.to_owned(),
                        array_dim: 1,
                        index: index as u16,
                        inner: usmap::PropertyInner::Double,
                    })
                    .collect(),
            },
        ],
        cext: None,
        ppth: Some(usmap::ExtPpth {
            version: 0,
            enums: Vec::new(),
            structs: vec![
                "/Script/G1R".to_owned(),
                "/Script/G1R".to_owned(),
                "/Script/G1R".to_owned(),
                "/Script/CoreUObject".to_owned(),
            ],
        }),
        eatr: Some(usmap::ExtEatr {
            version: 0,
            enum_flags: Vec::new(),
            struct_flags: vec![
                usmap::StructFlags {
                    type_: usmap::FlagsType::Class,
                    value: 0,
                    prop_flags: Vec::new(),
                },
                usmap::StructFlags {
                    type_: usmap::FlagsType::Struct,
                    value: 0,
                    prop_flags: Vec::new(),
                },
                usmap::StructFlags {
                    type_: usmap::FlagsType::Struct,
                    value: 0,
                    prop_flags: Vec::new(),
                },
                usmap::StructFlags {
                    type_: usmap::FlagsType::Struct,
                    value: 0,
                    prop_flags: Vec::new(),
                },
            ],
        }),
        envp: None,
    };
    let mut bytes = Vec::new();
    mapping.write(&mut bytes)?;
    fs::write(path, bytes)?;
    Ok(())
}
