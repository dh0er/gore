//! Closed, reviewed gameplay semantics layered over exact fixed-leaf selectors.
//!
//! This module deliberately owns no package, filesystem, project, build, or runtime authority.
//! It recognizes one reviewed property shape, validates a semantic value, and lowers it to the
//! same fixed-width bytes already consumed by the generic patch planner.

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    FixedLeafRole, FixedLeafSelector, FixedLeafSelectorStep, FixedLeafWireType, FixedWireKind,
    PackageComponent, FIXED_LEAF_SELECTOR_FORMAT, FIXED_LEAF_SELECTOR_PROFILE,
};

/// Closed wire-format revision for reviewed DataAsset intents.
pub const REVIEWED_DATAASSET_FORMAT_V1: u32 = 1;
/// Stable schema identifier for the three reviewed native footstep presets.
pub const REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID: &str = "g1r.tracking.footstep-preset";
pub const REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION: u32 = 1;
/// The only reviewed field in schema revision 1.
pub const REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID: &str = "feet_texture_size";

const REVIEWED_FOOTSTEP_PRESET_BINDING_DOMAIN_V1: &[u8] =
    b"gore-asset.reviewed-dataasset.footstep-preset.feet-texture-size.v1\0";
const FOOTSTEP_CLASS_PATH: &str = "/Script/G1R.FootstepTag";
const G1R_MODULE_PATH: &str = "/Script/G1R";
const BONE_FEET_SCHEMA_PATH: &str = "/Script/G1R.BoneFeetData";

/// The complete reviewed target set for footstep-preset schema revision 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewedFootstepPresetTargetV1 {
    Human,
    Scavenger,
    Wolf,
}

impl ReviewedFootstepPresetTargetV1 {
    pub const ALL: [Self; 3] = [Self::Human, Self::Scavenger, Self::Wolf];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Human => "g1r:dataasset:footstep-preset:human",
            Self::Scavenger => "g1r:dataasset:footstep-preset:scavenger",
            Self::Wolf => "g1r:dataasset:footstep-preset:wolf",
        }
    }

    pub const fn target_path(self) -> &'static str {
        match self {
            Self::Human => "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_HumanFootsteps",
            Self::Scavenger => {
                "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_ScavengerFootsteps"
            }
            Self::Wolf => "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps",
        }
    }

    pub const fn object_name(self) -> &'static str {
        match self {
            Self::Human => "DA_HumanFootsteps",
            Self::Scavenger => "DA_ScavengerFootsteps",
            Self::Wolf => "DA_WolfFootsteps",
        }
    }

    pub fn from_id(value: &str) -> Result<Self, ReviewedDataAssetErrorV1> {
        Self::ALL
            .into_iter()
            .find(|target| target.id() == value)
            .ok_or(ReviewedDataAssetErrorV1::UnknownTargetId)
    }

    pub fn from_target_path(value: &str) -> Result<Self, ReviewedDataAssetErrorV1> {
        Self::ALL
            .into_iter()
            .find(|target| target.target_path() == value)
            .ok_or(ReviewedDataAssetErrorV1::UnknownTargetPath)
    }
}

/// Validate the complete closed schema identity before selecting a reviewed target.
pub fn reviewed_footstep_preset_target_from_ids_v1(
    format: u32,
    schema_id: &str,
    schema_revision: u32,
    field_id: &str,
    target_id: &str,
) -> Result<ReviewedFootstepPresetTargetV1, ReviewedDataAssetErrorV1> {
    if format != REVIEWED_DATAASSET_FORMAT_V1 {
        return Err(ReviewedDataAssetErrorV1::UnsupportedFormat);
    }
    if schema_id != REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID {
        return Err(ReviewedDataAssetErrorV1::UnsupportedSchemaId);
    }
    if schema_revision != REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION {
        return Err(ReviewedDataAssetErrorV1::UnsupportedSchemaRevision);
    }
    if field_id != REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID {
        return Err(ReviewedDataAssetErrorV1::UnsupportedFieldId);
    }
    ReviewedFootstepPresetTargetV1::from_id(target_id)
}

/// A validated semantic request. Units and gameplay behavior remain intentionally unqualified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewedFootstepPresetSizeV1 {
    x: f64,
    y: f64,
}

impl ReviewedFootstepPresetSizeV1 {
    pub fn try_new(x: f64, y: f64) -> Result<Self, ReviewedDataAssetErrorV1> {
        validate_positive_finite("x", x)?;
        validate_positive_finite("y", y)?;
        Ok(Self { x, y })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }
}

/// Exact lowering result for one reviewed intent.
///
/// `selector` remains snapshot-specific. `replacement_bytes` is suitable for the existing
/// fixed-leaf planner; no offset or publication authority is introduced here.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewedFootstepPresetReplacementV1 {
    target: ReviewedFootstepPresetTargetV1,
    selector: FixedLeafSelector,
    requested: ReviewedFootstepPresetSizeV1,
    current_components: [f64; 4],
    replacement_components: [f64; 4],
    expected_bytes: [u8; 32],
    replacement_bytes: [u8; 32],
    binding_sha256: [u8; 32],
}

impl ReviewedFootstepPresetReplacementV1 {
    pub const fn format(&self) -> u32 {
        REVIEWED_DATAASSET_FORMAT_V1
    }

    pub const fn schema_id(&self) -> &'static str {
        REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID
    }

    pub const fn schema_revision(&self) -> u32 {
        REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION
    }

    pub const fn field_id(&self) -> &'static str {
        REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID
    }

    pub const fn target(&self) -> ReviewedFootstepPresetTargetV1 {
        self.target
    }

    pub fn selector(&self) -> &FixedLeafSelector {
        &self.selector
    }

    pub const fn requested(&self) -> ReviewedFootstepPresetSizeV1 {
        self.requested
    }

    pub const fn current_components(&self) -> [f64; 4] {
        self.current_components
    }

    pub const fn replacement_components(&self) -> [f64; 4] {
        self.replacement_components
    }

    pub const fn expected_bytes(&self) -> &[u8; 32] {
        &self.expected_bytes
    }

    pub const fn replacement_bytes(&self) -> &[u8; 32] {
        &self.replacement_bytes
    }

    pub const fn binding_sha256(&self) -> &[u8; 32] {
        &self.binding_sha256
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReviewedDataAssetErrorV1 {
    #[error("unsupported reviewed DataAsset format")]
    UnsupportedFormat,
    #[error("unsupported reviewed DataAsset schema id")]
    UnsupportedSchemaId,
    #[error("unsupported reviewed DataAsset schema revision")]
    UnsupportedSchemaRevision,
    #[error("unsupported reviewed DataAsset field id")]
    UnsupportedFieldId,
    #[error("unknown reviewed footstep-preset target id")]
    UnknownTargetId,
    #[error("unknown reviewed footstep-preset target path")]
    UnknownTargetPath,
    #[error("reviewed footstep-preset size component {component} is not finite")]
    NonFiniteSize { component: &'static str },
    #[error("reviewed footstep-preset size component {component} is not positive")]
    NonPositiveSize { component: &'static str },
    #[error("fixed-leaf selector does not match reviewed fact {fact}")]
    SelectorMismatch { fact: &'static str },
    #[error("fixed-leaf selector has invalid expected Vector4 bytes")]
    InvalidExpectedBytes,
    #[error("current fixed-leaf Vector4 component {component} is not finite")]
    NonFiniteCurrentComponent { component: &'static str },
    #[error("reviewed footstep-preset edit does not change the selected value")]
    NoChange,
    #[error("reviewed footstep-preset selector could not be bound")]
    BindingSerialization,
}

/// Match and lower one reviewed FootstepTag `FeetTextureSize` edit.
///
/// The target and every reviewed selector fact are exact. Snapshot seals and observed bytes remain
/// part of the supplied selector and therefore part of the returned binding, but are not pinned to
/// one game hotfix by this schema registry.
pub fn prepare_reviewed_footstep_preset_size_v1(
    target_path: &str,
    selector: &FixedLeafSelector,
    requested: ReviewedFootstepPresetSizeV1,
) -> Result<ReviewedFootstepPresetReplacementV1, ReviewedDataAssetErrorV1> {
    let target = ReviewedFootstepPresetTargetV1::from_target_path(target_path)?;
    validate_reviewed_selector(target, selector)?;

    let expected: [u8; 32] = selector
        .expected_bytes()
        .map_err(|_| ReviewedDataAssetErrorV1::InvalidExpectedBytes)?
        .try_into()
        .map_err(|_| ReviewedDataAssetErrorV1::InvalidExpectedBytes)?;
    let current_components = decode_vector4_f64(expected);
    for (component, value) in ["x", "y", "z", "w"].into_iter().zip(current_components) {
        if !value.is_finite() {
            return Err(ReviewedDataAssetErrorV1::NonFiniteCurrentComponent { component });
        }
    }
    let replacement_components = [
        requested.x(),
        requested.y(),
        current_components[2],
        current_components[3],
    ];
    let mut replacement = expected;
    replacement[..8].copy_from_slice(&requested.x().to_le_bytes());
    replacement[8..16].copy_from_slice(&requested.y().to_le_bytes());
    if replacement == expected {
        return Err(ReviewedDataAssetErrorV1::NoChange);
    }

    let selector_json =
        serde_json::to_vec(selector).map_err(|_| ReviewedDataAssetErrorV1::BindingSerialization)?;
    let binding_sha256 = reviewed_binding_sha256(target, target_path, &selector_json, &replacement);

    Ok(ReviewedFootstepPresetReplacementV1 {
        target,
        selector: selector.clone(),
        requested,
        current_components,
        replacement_components,
        expected_bytes: expected,
        replacement_bytes: replacement,
        binding_sha256,
    })
}

fn validate_positive_finite(
    component: &'static str,
    value: f64,
) -> Result<(), ReviewedDataAssetErrorV1> {
    if !value.is_finite() {
        return Err(ReviewedDataAssetErrorV1::NonFiniteSize { component });
    }
    if value <= 0.0 {
        return Err(ReviewedDataAssetErrorV1::NonPositiveSize { component });
    }
    Ok(())
}

fn validate_reviewed_selector(
    target: ReviewedFootstepPresetTargetV1,
    selector: &FixedLeafSelector,
) -> Result<(), ReviewedDataAssetErrorV1> {
    require_fact(
        selector.format == FIXED_LEAF_SELECTOR_FORMAT,
        "selector format",
    )?;
    require_fact(
        selector.profile == FIXED_LEAF_SELECTOR_PROFILE,
        "selector profile",
    )?;
    require_fact(selector.export_index == 0, "export index")?;
    require_fact(selector.object_name == target.object_name(), "object name")?;
    require_fact(selector.class_path == FOOTSTEP_CLASS_PATH, "class path")?;
    require_fact(
        selector.component == PackageComponent::Uexp,
        "package component",
    )?;
    require_fact(selector.role == FixedLeafRole::PropertyValue, "leaf role")?;
    require_fact(selector.kind == FixedWireKind::Vector4F64x4, "wire kind")?;
    require_fact(is_canonical_sha256(&selector.usmap_sha256), "USMAP seal")?;
    require_fact(is_canonical_sha256(&selector.export_sha256), "export seal")?;

    let [FixedLeafSelectorStep::Property {
        schema_index: bone_data_schema_index,
        property_name: bone_data_property,
        array_index: bone_data_array_index,
        array_dimension: bone_data_array_dimension,
        declaring_schema_name: footstep_schema,
        declaring_module_path: footstep_module,
        property_type: FixedLeafWireType::Struct {
            name: bone_data_type,
        },
    }, FixedLeafSelectorStep::Struct {
        name: nested_struct_name,
        schema_name: nested_schema_name,
    }, FixedLeafSelectorStep::Property {
        schema_index: texture_size_schema_index,
        property_name: texture_size_property,
        array_index: texture_size_array_index,
        array_dimension: texture_size_array_dimension,
        declaring_schema_name: bone_feet_schema,
        declaring_module_path: bone_feet_module,
        property_type: FixedLeafWireType::Struct {
            name: texture_size_type,
        },
    }] = selector.path.as_slice()
    else {
        return Err(ReviewedDataAssetErrorV1::SelectorMismatch {
            fact: "semantic path shape",
        });
    };

    require_fact(*bone_data_schema_index == 0, "BoneData schema index")?;
    require_fact(bone_data_property == "BoneData", "BoneData property")?;
    require_fact(*bone_data_array_index == 0, "BoneData array index")?;
    require_fact(*bone_data_array_dimension == 1, "BoneData array dimension")?;
    require_fact(
        footstep_schema == "FootstepTag",
        "FootstepTag declaring schema",
    )?;
    require_fact(
        footstep_module.as_deref() == Some(G1R_MODULE_PATH),
        "FootstepTag declaring module",
    )?;
    require_fact(bone_data_type == "BoneFeetData", "BoneData property type")?;
    require_fact(nested_struct_name == "BoneFeetData", "nested struct name")?;
    require_fact(
        nested_schema_name == BONE_FEET_SCHEMA_PATH,
        "nested struct schema",
    )?;
    require_fact(
        *texture_size_schema_index == 0,
        "FeetTextureSize schema index",
    )?;
    require_fact(
        texture_size_property == "FeetTextureSize",
        "FeetTextureSize property",
    )?;
    require_fact(
        *texture_size_array_index == 0,
        "FeetTextureSize array index",
    )?;
    require_fact(
        *texture_size_array_dimension == 1,
        "FeetTextureSize array dimension",
    )?;
    require_fact(
        bone_feet_schema == "BoneFeetData",
        "BoneFeetData declaring schema",
    )?;
    require_fact(
        bone_feet_module.as_deref() == Some(G1R_MODULE_PATH),
        "BoneFeetData declaring module",
    )?;
    require_fact(
        texture_size_type == "Vector4",
        "FeetTextureSize property type",
    )?;
    Ok(())
}

fn require_fact(value: bool, fact: &'static str) -> Result<(), ReviewedDataAssetErrorV1> {
    if value {
        Ok(())
    } else {
        Err(ReviewedDataAssetErrorV1::SelectorMismatch { fact })
    }
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_vector4_f64(bytes: [u8; 32]) -> [f64; 4] {
    std::array::from_fn(|index| {
        let start = index * 8;
        f64::from_le_bytes(
            bytes[start..start + 8]
                .try_into()
                .expect("fixed Vector4 lane"),
        )
    })
}

fn reviewed_binding_sha256(
    target: ReviewedFootstepPresetTargetV1,
    target_path: &str,
    selector_json: &[u8],
    replacement: &[u8; 32],
) -> [u8; 32] {
    let format = REVIEWED_DATAASSET_FORMAT_V1.to_le_bytes();
    let schema_revision = REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION.to_le_bytes();
    let mut hasher = Sha256::new();
    hasher.update(REVIEWED_FOOTSTEP_PRESET_BINDING_DOMAIN_V1);
    for value in [
        format.as_slice(),
        REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID.as_bytes(),
        schema_revision.as_slice(),
        REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID.as_bytes(),
        target.id().as_bytes(),
        target_path.as_bytes(),
        selector_json,
        replacement.as_slice(),
    ] {
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WOLF_SELECTOR_JSON: &str = r#"{
      "class_path": "/Script/G1R.FootstepTag",
      "component": "uexp",
      "expected_hex": "000000000000244000000000000024400000000000000000000000000000f03f",
      "export_index": 0,
      "export_sha256": "51e80a3a218b04f00f4016c780bd5cea0c7bae2512e40fd89476b72650125e08",
      "format": 1,
      "kind": "vector4_f64x4",
      "object_name": "DA_WolfFootsteps",
      "package_seal": {
        "uasset_sha256": "50fe60ade85a393f383bf2f44caee31f6553860c145f3f18462b85a6a9aad2fc",
        "uexp_sha256": "7ae97155c68748470ddef015c17371608561b88c9bba374df3432e8dcf3190fe"
      },
      "path": [
        {
          "array_dimension": 1,
          "array_index": 0,
          "declaring_module_path": "/Script/G1R",
          "declaring_schema_name": "FootstepTag",
          "property_name": "BoneData",
          "property_type": { "name": "BoneFeetData", "type": "struct" },
          "schema_index": 0,
          "step": "property"
        },
        {
          "name": "BoneFeetData",
          "schema_name": "/Script/G1R.BoneFeetData",
          "step": "struct"
        },
        {
          "array_dimension": 1,
          "array_index": 0,
          "declaring_module_path": "/Script/G1R",
          "declaring_schema_name": "BoneFeetData",
          "property_name": "FeetTextureSize",
          "property_type": { "name": "Vector4", "type": "struct" },
          "schema_index": 0,
          "step": "property"
        }
      ],
      "profile": "g1r_ue5_4",
      "role": "property_value",
      "usmap_sha256": "73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca"
    }"#;

    fn wolf_selector() -> FixedLeafSelector {
        serde_json::from_str(WOLF_SELECTOR_JSON).expect("tracked real Wolf selector")
    }

    fn request(x: f64, y: f64) -> ReviewedFootstepPresetSizeV1 {
        ReviewedFootstepPresetSizeV1::try_new(x, y).unwrap()
    }

    fn prepare(
        selector: &FixedLeafSelector,
    ) -> Result<ReviewedFootstepPresetReplacementV1, ReviewedDataAssetErrorV1> {
        prepare_reviewed_footstep_preset_size_v1(
            ReviewedFootstepPresetTargetV1::Wolf.target_path(),
            selector,
            request(11.0, 12.0),
        )
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn closed_identity_accepts_only_the_declared_schema_field_and_targets() {
        for target in ReviewedFootstepPresetTargetV1::ALL {
            assert_eq!(
                reviewed_footstep_preset_target_from_ids_v1(
                    REVIEWED_DATAASSET_FORMAT_V1,
                    REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID,
                    REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
                    REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
                    target.id(),
                ),
                Ok(target)
            );
            assert_eq!(
                ReviewedFootstepPresetTargetV1::from_target_path(target.target_path()),
                Ok(target)
            );
        }
        assert_eq!(
            reviewed_footstep_preset_target_from_ids_v1(
                2,
                REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID,
                REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
                REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
                ReviewedFootstepPresetTargetV1::Wolf.id(),
            ),
            Err(ReviewedDataAssetErrorV1::UnsupportedFormat)
        );
        assert_eq!(
            reviewed_footstep_preset_target_from_ids_v1(
                REVIEWED_DATAASSET_FORMAT_V1,
                "g1r.tracking.near-miss",
                REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
                REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
                ReviewedFootstepPresetTargetV1::Wolf.id(),
            ),
            Err(ReviewedDataAssetErrorV1::UnsupportedSchemaId)
        );
        assert_eq!(
            reviewed_footstep_preset_target_from_ids_v1(
                REVIEWED_DATAASSET_FORMAT_V1,
                REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID,
                2,
                REVIEWED_FEET_TEXTURE_SIZE_FIELD_ID,
                ReviewedFootstepPresetTargetV1::Wolf.id(),
            ),
            Err(ReviewedDataAssetErrorV1::UnsupportedSchemaRevision)
        );
        assert_eq!(
            reviewed_footstep_preset_target_from_ids_v1(
                REVIEWED_DATAASSET_FORMAT_V1,
                REVIEWED_FOOTSTEP_PRESET_SCHEMA_ID,
                REVIEWED_FOOTSTEP_PRESET_SCHEMA_REVISION,
                "feet_texture_scale",
                ReviewedFootstepPresetTargetV1::Wolf.id(),
            ),
            Err(ReviewedDataAssetErrorV1::UnsupportedFieldId)
        );
        assert_eq!(
            ReviewedFootstepPresetTargetV1::from_id("g1r:dataasset:footstep-preset:dragon"),
            Err(ReviewedDataAssetErrorV1::UnknownTargetId)
        );
    }

    #[test]
    fn size_rejects_nan_infinity_zero_and_negative_values() {
        for (x, y, expected) in [
            (
                f64::NAN,
                1.0,
                ReviewedDataAssetErrorV1::NonFiniteSize { component: "x" },
            ),
            (
                1.0,
                f64::INFINITY,
                ReviewedDataAssetErrorV1::NonFiniteSize { component: "y" },
            ),
            (
                0.0,
                1.0,
                ReviewedDataAssetErrorV1::NonPositiveSize { component: "x" },
            ),
            (
                1.0,
                -0.0,
                ReviewedDataAssetErrorV1::NonPositiveSize { component: "y" },
            ),
            (
                -1.0,
                1.0,
                ReviewedDataAssetErrorV1::NonPositiveSize { component: "x" },
            ),
        ] {
            assert_eq!(ReviewedFootstepPresetSizeV1::try_new(x, y), Err(expected));
        }
    }

    #[test]
    fn exact_match_preserves_z_and_w_bytes() {
        let result = prepare(&wolf_selector()).unwrap();
        assert_eq!(result.target(), ReviewedFootstepPresetTargetV1::Wolf);
        assert_eq!(result.current_components(), [10.0, 10.0, 0.0, 1.0]);
        assert_eq!(result.replacement_components(), [11.0, 12.0, 0.0, 1.0]);
        assert_eq!(
            &result.replacement_bytes()[16..],
            &result.expected_bytes()[16..]
        );
        assert_eq!(&result.replacement_bytes()[..8], &11.0f64.to_le_bytes());
        assert_eq!(&result.replacement_bytes()[8..16], &12.0f64.to_le_bytes());
        assert_eq!(result.selector(), &wolf_selector());
    }

    #[test]
    fn selector_near_misses_fail_closed() {
        assert_eq!(
            prepare_reviewed_footstep_preset_size_v1(
                "/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps_Copy",
                &wolf_selector(),
                request(11.0, 12.0),
            ),
            Err(ReviewedDataAssetErrorV1::UnknownTargetPath)
        );

        let mut variants = Vec::new();
        let mut selector = wolf_selector();
        selector.class_path.push_str("NearMiss");
        variants.push(selector);
        let mut selector = wolf_selector();
        selector.object_name.push_str("NearMiss");
        variants.push(selector);
        let mut selector = wolf_selector();
        selector.role = FixedLeafRole::MapKey;
        variants.push(selector);
        let mut selector = wolf_selector();
        selector.kind = FixedWireKind::LinearColorF32x4;
        variants.push(selector);
        let mut selector = wolf_selector();
        selector.path.push(FixedLeafSelectorStep::Struct {
            name: "NearMiss".to_owned(),
            schema_name: "/Script/G1R.NearMiss".to_owned(),
        });
        variants.push(selector);
        let mut selector = wolf_selector();
        if let FixedLeafSelectorStep::Property { schema_index, .. } = &mut selector.path[0] {
            *schema_index = 1;
        }
        variants.push(selector);
        let mut selector = wolf_selector();
        if let FixedLeafSelectorStep::Struct { schema_name, .. } = &mut selector.path[1] {
            *schema_name = "/Script/G1R.NearMiss".to_owned();
        }
        variants.push(selector);
        let mut selector = wolf_selector();
        if let FixedLeafSelectorStep::Property { property_type, .. } = &mut selector.path[2] {
            *property_type = FixedLeafWireType::Struct {
                name: "Vector".to_owned(),
            };
        }
        variants.push(selector);

        for selector in variants {
            assert!(matches!(
                prepare(&selector),
                Err(ReviewedDataAssetErrorV1::SelectorMismatch { .. })
            ));
        }
    }

    #[test]
    fn no_change_is_rejected() {
        assert_eq!(
            prepare_reviewed_footstep_preset_size_v1(
                ReviewedFootstepPresetTargetV1::Wolf.target_path(),
                &wolf_selector(),
                request(10.0, 10.0),
            ),
            Err(ReviewedDataAssetErrorV1::NoChange)
        );
    }

    #[test]
    fn reviewed_binding_has_a_stable_golden_and_binds_the_semantic_intent() {
        let selector = wolf_selector();
        let first = prepare(&selector).unwrap();
        assert_eq!(
            hex(first.binding_sha256()),
            "4a0fcacfecbf87011bd435388fef54b05300f5e634809630104d0f9c0da705e4"
        );
        let changed = prepare_reviewed_footstep_preset_size_v1(
            ReviewedFootstepPresetTargetV1::Wolf.target_path(),
            &selector,
            request(11.0, 13.0),
        )
        .unwrap();
        assert_ne!(first.binding_sha256(), changed.binding_sha256());
    }

    #[test]
    fn non_finite_current_components_fail_closed() {
        for (lane, component) in ["x", "y", "z", "w"].into_iter().enumerate() {
            let mut selector = wolf_selector();
            let mut expected: [u8; 32] = selector.expected_bytes().unwrap().try_into().unwrap();
            expected[lane * 8..lane * 8 + 8].copy_from_slice(&f64::INFINITY.to_le_bytes());
            selector.expected_hex = hex(&expected);
            assert_eq!(
                prepare(&selector),
                Err(ReviewedDataAssetErrorV1::NonFiniteCurrentComponent { component })
            );
        }
    }
}
