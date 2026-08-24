//! Exact reader for Hazelight UnrealEngine-Angelscript's `Binds.Cache` database.
//!
//! The authoritative field order is `FAngelscriptBindDatabase::Serialize` together with the
//! `FArchive` operators in UNREANGEL's `AngelscriptBindDatabase.h`. Unlike the older decompiler
//! helper in `cache::binds`, this parser consumes every field and requires exact EOF alignment.

use crate::cache::wire::{Cursor, WireError};
use sha2::{Digest as _, Sha256};

const MAX_BINDS_CACHE_BYTES: usize = 128 * 1024 * 1024;
const MAX_TOP_LEVEL_BINDS: usize = 1_000_000;
const MAX_MEMBERS_PER_BIND: usize = 1_000_000;
const CANONICAL_BINDS_HASH_DOMAIN: &[u8] = b"gore-as-compiler-binds-v1\0";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BindsDatabaseError {
    #[error("Binds.Cache is {actual} bytes; maximum accepted size is {max}")]
    InputTooLarge { actual: usize, max: usize },
    #[error("{field} count {actual} exceeds the maximum {max}")]
    CountTooLarge {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("Binds.Cache has {remaining} trailing bytes after offset {offset}")]
    TrailingBytes { offset: usize, remaining: usize },
    #[error(transparent)]
    Wire(#[from] WireError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyBind {
    pub declaration: String,
    pub unreal_path: String,
    pub can_write: bool,
    pub can_read: bool,
    pub can_edit: bool,
    pub generated_getter: bool,
    pub generated_setter: bool,
    pub generated_name: String,
    pub generated_handle: bool,
    pub generated_unresolved_object: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructBind {
    pub type_name: String,
    pub unreal_path: String,
    pub properties: Vec<PropertyBind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodBind {
    pub declaration: String,
    pub unreal_path: String,
    pub static_in_unreal: bool,
    pub static_in_script: bool,
    pub global_scope: bool,
    pub not_angelscript_property: bool,
    pub trivial: bool,
    pub world_context_argument: i8,
    pub determines_output_type_argument: i8,
    pub class_name: String,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassBind {
    pub type_name: String,
    pub unreal_path: String,
    pub methods: Vec<MethodBind>,
    pub properties: Vec<PropertyBind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindsDatabase {
    pub structs: Vec<StructBind>,
    pub classes: Vec<ClassBind>,
}

impl BindsDatabase {
    pub fn parse(bytes: &[u8]) -> Result<Self, BindsDatabaseError> {
        if bytes.len() > MAX_BINDS_CACHE_BYTES {
            return Err(BindsDatabaseError::InputTooLarge {
                actual: bytes.len(),
                max: MAX_BINDS_CACHE_BYTES,
            });
        }

        let mut cursor = Cursor::new(bytes);
        let struct_count = read_bounded_count(
            &mut cursor,
            "FAngelscriptBindDatabase.Structs",
            MAX_TOP_LEVEL_BINDS,
            12,
        )?;
        let mut structs = Vec::with_capacity(struct_count);
        for _ in 0..struct_count {
            structs.push(read_struct(&mut cursor)?);
        }

        let class_count = read_bounded_count(
            &mut cursor,
            "FAngelscriptBindDatabase.Classes",
            MAX_TOP_LEVEL_BINDS,
            16,
        )?;
        let mut classes = Vec::with_capacity(class_count);
        for _ in 0..class_count {
            classes.push(read_class(&mut cursor)?);
        }

        if cursor.remaining() != 0 {
            return Err(BindsDatabaseError::TrailingBytes {
                offset: cursor.pos(),
                remaining: cursor.remaining(),
            });
        }
        Ok(Self { structs, classes })
    }

    pub fn property_count(&self) -> usize {
        self.struct_property_count() + self.class_property_count()
    }

    pub fn method_count(&self) -> usize {
        self.classes.iter().map(|bind| bind.methods.len()).sum()
    }

    pub fn struct_property_count(&self) -> usize {
        self.structs.iter().map(|bind| bind.properties.len()).sum()
    }

    pub fn class_property_count(&self) -> usize {
        self.classes.iter().map(|bind| bind.properties.len()).sum()
    }

    /// Domain-separated hash of the fully decoded semantic database.
    ///
    /// This intentionally differs from the source-file seal. It binds profile qualification to
    /// every decoded field while normalizing ANSI/UTF-16 `FString` representation details. A
    /// parser field-order or flag-width regression therefore changes the compiler-profile input.
    pub fn canonical_sha256(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(CANONICAL_BINDS_HASH_DOMAIN);
        hash_u64(&mut hash, self.structs.len());
        for bind in &self.structs {
            hash_string(&mut hash, &bind.type_name);
            hash_string(&mut hash, &bind.unreal_path);
            hash_u64(&mut hash, bind.properties.len());
            for property in &bind.properties {
                hash_property(&mut hash, property);
            }
        }
        hash_u64(&mut hash, self.classes.len());
        for bind in &self.classes {
            hash_string(&mut hash, &bind.type_name);
            hash_string(&mut hash, &bind.unreal_path);
            hash_u64(&mut hash, bind.methods.len());
            for method in &bind.methods {
                hash_method(&mut hash, method);
            }
            hash_u64(&mut hash, bind.properties.len());
            for property in &bind.properties {
                hash_property(&mut hash, property);
            }
        }
        hash.finalize().into()
    }
}

fn hash_u64(hash: &mut Sha256, value: usize) {
    hash.update((value as u64).to_le_bytes());
}

fn hash_string(hash: &mut Sha256, value: &str) {
    hash_u64(hash, value.len());
    hash.update(value.as_bytes());
}

fn hash_bool(hash: &mut Sha256, value: bool) {
    hash.update([u8::from(value)]);
}

fn hash_property(hash: &mut Sha256, property: &PropertyBind) {
    hash_string(hash, &property.declaration);
    hash_string(hash, &property.unreal_path);
    hash_bool(hash, property.can_write);
    hash_bool(hash, property.can_read);
    hash_bool(hash, property.can_edit);
    hash_bool(hash, property.generated_getter);
    hash_bool(hash, property.generated_setter);
    hash_string(hash, &property.generated_name);
    hash_bool(hash, property.generated_handle);
    hash_bool(hash, property.generated_unresolved_object);
}

fn hash_method(hash: &mut Sha256, method: &MethodBind) {
    hash_string(hash, &method.declaration);
    hash_string(hash, &method.unreal_path);
    hash_bool(hash, method.static_in_unreal);
    hash_bool(hash, method.static_in_script);
    hash_bool(hash, method.global_scope);
    hash_bool(hash, method.not_angelscript_property);
    hash_bool(hash, method.trivial);
    hash.update(method.world_context_argument.to_le_bytes());
    hash.update(method.determines_output_type_argument.to_le_bytes());
    hash_string(hash, &method.class_name);
    hash_string(hash, &method.script_name);
}

fn read_struct(cursor: &mut Cursor<'_>) -> Result<StructBind, BindsDatabaseError> {
    let type_name = cursor.read_fstring()?;
    let unreal_path = cursor.read_fstring()?;
    let count = read_bounded_count(
        cursor,
        "FAngelscriptStructBind.Properties",
        MAX_MEMBERS_PER_BIND,
        40,
    )?;
    let mut properties = Vec::with_capacity(count);
    for _ in 0..count {
        properties.push(read_property(cursor)?);
    }
    Ok(StructBind {
        type_name,
        unreal_path,
        properties,
    })
}

fn read_class(cursor: &mut Cursor<'_>) -> Result<ClassBind, BindsDatabaseError> {
    let type_name = cursor.read_fstring()?;
    let unreal_path = cursor.read_fstring()?;
    let method_count = read_bounded_count(
        cursor,
        "FAngelscriptClassBind.Methods",
        MAX_MEMBERS_PER_BIND,
        38,
    )?;
    let mut methods = Vec::with_capacity(method_count);
    for _ in 0..method_count {
        methods.push(read_method(cursor)?);
    }
    let property_count = read_bounded_count(
        cursor,
        "FAngelscriptClassBind.Properties",
        MAX_MEMBERS_PER_BIND,
        40,
    )?;
    let mut properties = Vec::with_capacity(property_count);
    for _ in 0..property_count {
        properties.push(read_property(cursor)?);
    }
    Ok(ClassBind {
        type_name,
        unreal_path,
        methods,
        properties,
    })
}

fn read_property(cursor: &mut Cursor<'_>) -> Result<PropertyBind, BindsDatabaseError> {
    Ok(PropertyBind {
        declaration: cursor.read_fstring()?,
        unreal_path: cursor.read_fstring()?,
        can_write: cursor.read_bool4()?,
        can_read: cursor.read_bool4()?,
        can_edit: cursor.read_bool4()?,
        generated_getter: cursor.read_bool4()?,
        generated_setter: cursor.read_bool4()?,
        generated_name: cursor.read_fstring()?,
        generated_handle: cursor.read_bool4()?,
        generated_unresolved_object: cursor.read_bool4()?,
    })
}

fn read_method(cursor: &mut Cursor<'_>) -> Result<MethodBind, BindsDatabaseError> {
    Ok(MethodBind {
        declaration: cursor.read_fstring()?,
        unreal_path: cursor.read_fstring()?,
        static_in_unreal: cursor.read_bool4()?,
        static_in_script: cursor.read_bool4()?,
        global_scope: cursor.read_bool4()?,
        not_angelscript_property: cursor.read_bool4()?,
        trivial: cursor.read_bool4()?,
        world_context_argument: cursor.read_i8()?,
        determines_output_type_argument: cursor.read_i8()?,
        class_name: cursor.read_fstring()?,
        script_name: cursor.read_fstring()?,
    })
}

fn read_bounded_count(
    cursor: &mut Cursor<'_>,
    field: &'static str,
    maximum: usize,
    minimum_element_bytes: usize,
) -> Result<usize, BindsDatabaseError> {
    let count = cursor.read_count(field)?;
    if count > maximum {
        return Err(BindsDatabaseError::CountTooLarge {
            field,
            actual: count,
            max: maximum,
        });
    }
    cursor.ensure_minimum_remaining(count, minimum_element_bytes, field)?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{BindsDatabase, BindsDatabaseError};
    use crate::cache::wire::WireError;

    fn push_i32(out: &mut Vec<u8>, value: i32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn push_bool(out: &mut Vec<u8>, value: bool) {
        push_i32(out, i32::from(value));
    }

    fn push_ansi(out: &mut Vec<u8>, value: &str) {
        push_i32(out, value.len() as i32 + 1);
        out.extend_from_slice(value.as_bytes());
        out.push(0);
    }

    fn representative_database() -> Vec<u8> {
        let mut out = Vec::new();
        push_i32(&mut out, 1); // Structs
        push_ansi(&mut out, "FVector");
        push_ansi(&mut out, "/Script/CoreUObject.Vector");
        push_i32(&mut out, 1); // Properties
        push_ansi(&mut out, "float X");
        push_ansi(&mut out, "/Script/CoreUObject.Vector:X");
        push_bool(&mut out, true);
        push_bool(&mut out, true);
        push_bool(&mut out, false);
        push_bool(&mut out, true);
        push_bool(&mut out, false);
        push_ansi(&mut out, "GetX");
        push_bool(&mut out, false);
        push_bool(&mut out, false);

        push_i32(&mut out, 1); // Classes
        push_ansi(&mut out, "UObject");
        push_ansi(&mut out, "/Script/CoreUObject.Object");
        push_i32(&mut out, 1); // Methods
        push_ansi(&mut out, "FString GetName() const");
        push_ansi(&mut out, "/Script/CoreUObject.Object:GetName");
        push_bool(&mut out, false);
        push_bool(&mut out, false);
        push_bool(&mut out, false);
        push_bool(&mut out, true);
        push_bool(&mut out, true);
        out.push((-1i8) as u8);
        out.push(2);
        push_ansi(&mut out, "UObject");
        push_ansi(&mut out, "GetName");
        push_i32(&mut out, 0); // Properties
        out
    }

    #[test]
    fn parses_the_authoritative_bind_database_field_order() {
        let parsed = BindsDatabase::parse(&representative_database()).unwrap();
        assert_eq!(parsed.structs.len(), 1);
        assert_eq!(parsed.classes.len(), 1);
        assert_eq!(parsed.property_count(), 1);
        assert_eq!(parsed.struct_property_count(), 1);
        assert_eq!(parsed.class_property_count(), 0);
        assert_eq!(parsed.method_count(), 1);
        assert_eq!(parsed.structs[0].properties[0].generated_name, "GetX");
        assert_eq!(parsed.classes[0].methods[0].world_context_argument, -1);
        assert_eq!(
            parsed.classes[0].methods[0].determines_output_type_argument,
            2
        );
        assert!(parsed.classes[0].methods[0].trivial);

        let stable = parsed.canonical_sha256();
        assert_eq!(stable, parsed.canonical_sha256());
        let mut changed = parsed.clone();
        changed.classes[0].methods[0].trivial = false;
        assert_ne!(stable, changed.canonical_sha256());

        let profile = crate::compiler_profile::manifest::BindsProfileV1::from_database(&parsed);
        assert_eq!(profile.struct_count, 1);
        assert_eq!(profile.class_count, 1);
        assert_eq!(profile.method_count, 1);
        assert_eq!(profile.struct_property_count, 1);
        assert_eq!(profile.class_property_count, 0);
        assert_eq!(profile.canonical_database_sha256.as_bytes(), &stable);
    }

    #[test]
    fn rejects_noncanonical_bools_and_trailing_data() {
        let mut invalid_bool = representative_database();
        let bool_offset = invalid_bool
            .windows(8)
            .position(|window| window == b"float X\0")
            .unwrap()
            + 8
            + 4
            + "/Script/CoreUObject.Vector:X".len()
            + 1;
        invalid_bool[bool_offset..bool_offset + 4].copy_from_slice(&2i32.to_le_bytes());
        assert!(matches!(
            BindsDatabase::parse(&invalid_bool),
            Err(BindsDatabaseError::Wire(WireError::BadLen {
                field: "bool",
                ..
            }))
        ));

        let mut trailing = representative_database();
        trailing.push(0);
        assert!(matches!(
            BindsDatabase::parse(&trailing),
            Err(BindsDatabaseError::TrailingBytes { remaining: 1, .. })
        ));
    }

    #[test]
    #[ignore = "set GORE_AS_BINDS to the exact shipped Binds.Cache being qualified"]
    fn parses_the_configured_real_binds_cache_to_exact_eof() {
        let path = std::env::var_os("GORE_AS_BINDS").expect("GORE_AS_BINDS is required");
        let bytes = std::fs::read(path).expect("read configured Binds.Cache");
        let parsed = BindsDatabase::parse(&bytes).expect("exact Binds.Cache parse");
        assert!(!parsed.structs.is_empty());
        assert!(!parsed.classes.is_empty());
        assert!(parsed.property_count() > 1_000);
        assert!(parsed.method_count() > 1_000);
        eprintln!(
            "structs={} struct_properties={} classes={} methods={} class_properties={} canonical_sha256={}",
            parsed.structs.len(),
            parsed.struct_property_count(),
            parsed.classes.len(),
            parsed.method_count(),
            parsed.class_property_count(),
            crate::compiler_profile::manifest::Sha256Digest::from_bytes(
                parsed.canonical_sha256()
            )
        );
    }
}
