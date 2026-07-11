//! AngelScript bytecode instruction-set (ISA) tables.
//!
//! `BcType` variants intentionally mirror the C `asEBCType` enum names
//! (e.g. `wW_rW_DW_ARG`) for 1:1 traceability to the source, hence the
//! non-camel-case allow.
#![allow(non_camel_case_types)]
//!
//! Source: `WillGordon9999/UNREANGEL@main`,
//! `Angelscript/Source/AngelscriptCode/Public/angelscript.h`
//! (`enum asEBCInstr`, `enum asEBCType`, `asBCTypeSize[]`, `asBCInfo[256]`).
//!
//! Version: `ANGELSCRIPT_VERSION 23300` ("2.33.0 WIP"), Hazelight UE5.4 fork.
//!
//! This build defines `AS_64BIT_PTR` (target `_M_X64`), so `AS_PTR_SIZE == 2`
//! and the pointer-sized macro types resolve to their 64-bit variants:
//! `PTR_ARG -> QW_ARG`, `PTR_DW_ARG -> QW_DW_ARG`,
//! `wW_PTR_ARG -> wW_QW_ARG`, `rW_PTR_ARG -> rW_QW_ARG`.
//!
//! Bytecode is a stream of 32-bit dwords. Each instruction starts on a dword
//! boundary: byte 0 = opcode (`asEBCInstr`), byte 1 = always 0, the remaining
//! bytes hold the operands per the format (`BcType`). Total instruction length
//! in dwords = [`OpInfo::size_dwords`] (derived from `asBCTypeSize`).

/// Operand layout of a bytecode instruction (mirrors `asEBCType`).
///
/// Field codes: `W` = 16-bit word arg, `rW`/`wW` = 16-bit stack-var ref,
/// `DW` = 32-bit, `QW` = 64-bit. Word args occupy word slots 1,2,3 of the
/// instruction (byte offsets 2,4,6); DW/QW args start at dword offset 1 (or 2
/// when preceded by two word args).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BcType {
    /// `INFO` (0 dwords): meta token, 0 dwords (never emitted to final bytecode).
    INFO,
    /// `NO_ARG` (1 dwords): opcode only.
    NO_ARG,
    /// `W_ARG` (1 dwords): one 16-bit word arg (bytes 2-3).
    W_ARG,
    /// `wW_ARG` (1 dwords): one 16-bit dst stack-var ref (bytes 2-3).
    wW_ARG,
    /// `DW_ARG` (2 dwords): one 32-bit arg at dword 1 (bytes 4-7).
    DW_ARG,
    /// `rW_DW_ARG` (2 dwords): rW (bytes 2-3) + 32-bit at dword 1 (bytes 4-7).
    rW_DW_ARG,
    /// `QW_ARG` (3 dwords): one 64-bit arg at dword 1 (bytes 4-11).
    QW_ARG,
    /// `DW_DW_ARG` (3 dwords): two 32-bit args at dwords 1,2 (bytes 4-11).
    DW_DW_ARG,
    /// `wW_rW_rW_ARG` (2 dwords): three 16-bit refs at words 1,2,3 (bytes 2-7).
    wW_rW_rW_ARG,
    /// `wW_QW_ARG` (3 dwords): wW (bytes 2-3) + 64-bit at dword 1 (bytes 4-11).
    wW_QW_ARG,
    /// `wW_rW_ARG` (2 dwords): two 16-bit refs at words 1,2 (bytes 2-5).
    wW_rW_ARG,
    /// `rW_ARG` (1 dwords): one 16-bit stack-var ref (bytes 2-3).
    rW_ARG,
    /// `wW_DW_ARG` (2 dwords): wW (bytes 2-3) + 32-bit at dword 1 (bytes 4-7).
    wW_DW_ARG,
    /// `wW_rW_DW_ARG` (3 dwords): wW,rW at words 1,2 + 32-bit at dword 2 (bytes 8-11).
    wW_rW_DW_ARG,
    /// `rW_rW_ARG` (2 dwords): two 16-bit refs at words 1,2 (bytes 2-5).
    rW_rW_ARG,
    /// `wW_W_ARG` (2 dwords): wW + W at words 1,2 (bytes 2-5).
    wW_W_ARG,
    /// `QW_DW_ARG` (4 dwords): 64-bit at dword 1 (bytes 4-11) + 32-bit at dword 3 (bytes 12-15).
    QW_DW_ARG,
    /// `rW_QW_ARG` (3 dwords): rW (bytes 2-3) + 64-bit at dword 1 (bytes 4-11).
    rW_QW_ARG,
    /// `W_DW_ARG` (2 dwords): W (bytes 2-3) + 32-bit at dword 1 (bytes 4-7).
    W_DW_ARG,
    /// `rW_W_DW_ARG` (3 dwords): rW,W at words 1,2 + 32-bit at dword 2 (bytes 8-11).
    rW_W_DW_ARG,
    /// `rW_DW_DW_ARG` (3 dwords): rW (bytes 2-3) + two 32-bit at dwords 1,2 (bytes 4-11).
    rW_DW_DW_ARG,
    /// `W_rW_ARG` (2 dwords): W + rW at words 1,2 (bytes 2-5).
    W_rW_ARG,
}

impl BcType {
    /// Total instruction length in 32-bit dwords (from `asBCTypeSize`).
    pub const fn size_dwords(self) -> u8 {
        match self {
            BcType::INFO => 0,
            BcType::NO_ARG => 1,
            BcType::W_ARG => 1,
            BcType::wW_ARG => 1,
            BcType::DW_ARG => 2,
            BcType::rW_DW_ARG => 2,
            BcType::QW_ARG => 3,
            BcType::DW_DW_ARG => 3,
            BcType::wW_rW_rW_ARG => 2,
            BcType::wW_QW_ARG => 3,
            BcType::wW_rW_ARG => 2,
            BcType::rW_ARG => 1,
            BcType::wW_DW_ARG => 2,
            BcType::wW_rW_DW_ARG => 3,
            BcType::rW_rW_ARG => 2,
            BcType::wW_W_ARG => 2,
            BcType::QW_DW_ARG => 4,
            BcType::rW_QW_ARG => 3,
            BcType::W_DW_ARG => 2,
            BcType::rW_W_DW_ARG => 3,
            BcType::rW_DW_DW_ARG => 3,
            BcType::W_rW_ARG => 2,
        }
    }
}

/// `stackInc` sentinel for opcodes whose stack delta is computed at runtime
/// (the source uses `0xFFFF`): CALL, RET, CALLSYS, CALLBND, ALLOC, CALLINTF,
/// CallPtr.
pub const STACK_INC_VARIABLE: i32 = 0xFFFF;

/// One row of the `asBCInfo[256]` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpInfo {
    /// Mnemonic without the `asBC_` prefix (e.g. `"PshC4"`).
    pub name: &'static str,
    /// Opcode byte (`asEBCInstr` value); first byte of the instruction.
    pub opcode: u8,
    /// Operand layout.
    pub fmt: BcType,
    /// Instruction length in dwords (== `fmt.size_dwords()`).
    pub size_dwords: u8,
    /// Stack delta in dwords; [`STACK_INC_VARIABLE`] when runtime-computed.
    pub stack_inc: i32,
}

/// Real opcodes 0..=212 (`asBC_MAXBYTECODE == 212`). 213 entries.
///
/// NOTE: opcodes 200..=212 (`Thiscall1`..`ThrowException`) are Hazelight/Unreal
/// extensions not present in stock AngelScript 2.33.
pub const OPCODES: [OpInfo; 213] = [
    OpInfo {
        name: "PopPtr",
        opcode: 0,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: -2,
    },
    OpInfo {
        name: "PshGPtr",
        opcode: 1,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 2,
    },
    OpInfo {
        name: "PshC4",
        opcode: 2,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 1,
    },
    OpInfo {
        name: "PshV4",
        opcode: 3,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 1,
    },
    OpInfo {
        name: "PSF",
        opcode: 4,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 2,
    },
    OpInfo {
        name: "SwapPtr",
        opcode: 5,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "NOT",
        opcode: 6,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "PshG4",
        opcode: 7,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 1,
    },
    OpInfo {
        name: "LdGRdR4",
        opcode: 8,
        fmt: BcType::wW_QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "CALL",
        opcode: 9,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: STACK_INC_VARIABLE,
    },
    OpInfo {
        name: "RET",
        opcode: 10,
        fmt: BcType::W_ARG,
        size_dwords: 1,
        stack_inc: STACK_INC_VARIABLE,
    },
    OpInfo {
        name: "JMP",
        opcode: 11,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JZ",
        opcode: 12,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JNZ",
        opcode: 13,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JS",
        opcode: 14,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JNS",
        opcode: 15,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JP",
        opcode: 16,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JNP",
        opcode: 17,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "TZ",
        opcode: 18,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "TNZ",
        opcode: 19,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "TS",
        opcode: 20,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "TNS",
        opcode: 21,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "TP",
        opcode: 22,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "TNP",
        opcode: 23,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "NEGi",
        opcode: 24,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "NEGf",
        opcode: 25,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "NEGd",
        opcode: 26,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "INCi16",
        opcode: 27,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "INCi8",
        opcode: 28,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "DECi16",
        opcode: 29,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "DECi8",
        opcode: 30,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "INCi",
        opcode: 31,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "DECi",
        opcode: 32,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "INCf",
        opcode: 33,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "DECf",
        opcode: 34,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "INCd",
        opcode: 35,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "DECd",
        opcode: 36,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "IncVi",
        opcode: 37,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "DecVi",
        opcode: 38,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "BNOT",
        opcode: 39,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "BAND",
        opcode: 40,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BOR",
        opcode: 41,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BXOR",
        opcode: 42,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BSLL",
        opcode: 43,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BSRL",
        opcode: 44,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BSRA",
        opcode: 45,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "COPY",
        opcode: 46,
        fmt: BcType::W_DW_ARG,
        size_dwords: 2,
        stack_inc: -2,
    },
    OpInfo {
        name: "PshC8",
        opcode: 47,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 2,
    },
    OpInfo {
        name: "PshVPtr",
        opcode: 48,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 2,
    },
    OpInfo {
        name: "RDSPtr",
        opcode: 49,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPd",
        opcode: 50,
        fmt: BcType::rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPu",
        opcode: 51,
        fmt: BcType::rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPf",
        opcode: 52,
        fmt: BcType::rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPi",
        opcode: 53,
        fmt: BcType::rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPIi",
        opcode: 54,
        fmt: BcType::rW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPIf",
        opcode: 55,
        fmt: BcType::rW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPIu",
        opcode: 56,
        fmt: BcType::rW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JMPP",
        opcode: 57,
        fmt: BcType::rW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "PopRPtr",
        opcode: 58,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: -2,
    },
    OpInfo {
        name: "PshRPtr",
        opcode: 59,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 2,
    },
    OpInfo {
        name: "STR",
        opcode: 60,
        fmt: BcType::W_ARG,
        size_dwords: 1,
        stack_inc: 3,
    },
    OpInfo {
        name: "CALLSYS",
        opcode: 61,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: STACK_INC_VARIABLE,
    },
    OpInfo {
        name: "CALLBND",
        opcode: 62,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: STACK_INC_VARIABLE,
    },
    OpInfo {
        name: "SUSPEND",
        opcode: 63,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "ALLOC",
        opcode: 64,
        fmt: BcType::QW_DW_ARG,
        size_dwords: 4,
        stack_inc: STACK_INC_VARIABLE,
    },
    OpInfo {
        name: "FREE",
        opcode: 65,
        fmt: BcType::wW_QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "LOADOBJ",
        opcode: 66,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "STOREOBJ",
        opcode: 67,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "GETOBJ",
        opcode: 68,
        fmt: BcType::W_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "REFCPY",
        opcode: 69,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: -2,
    },
    OpInfo {
        name: "CHKREF",
        opcode: 70,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "GETOBJREF",
        opcode: 71,
        fmt: BcType::W_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "GETREF",
        opcode: 72,
        fmt: BcType::W_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "PshNull",
        opcode: 73,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 2,
    },
    OpInfo {
        name: "ClrVPtr",
        opcode: 74,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "OBJTYPE",
        opcode: 75,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 2,
    },
    OpInfo {
        name: "TYPEID",
        opcode: 76,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 1,
    },
    OpInfo {
        name: "SetV4",
        opcode: 77,
        fmt: BcType::wW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SetV8",
        opcode: 78,
        fmt: BcType::wW_QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "ADDSi",
        opcode: 79,
        fmt: BcType::W_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyVtoV4",
        opcode: 80,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyVtoV8",
        opcode: 81,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyVtoR4",
        opcode: 82,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyVtoR8",
        opcode: 83,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyVtoG4",
        opcode: 84,
        fmt: BcType::rW_QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyRtoV4",
        opcode: 85,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyRtoV8",
        opcode: 86,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyGtoV4",
        opcode: 87,
        fmt: BcType::wW_QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "WRTV1",
        opcode: 88,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "WRTV2",
        opcode: 89,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "WRTV4",
        opcode: 90,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "WRTV8",
        opcode: 91,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "RDR1",
        opcode: 92,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "RDR2",
        opcode: 93,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "RDR4",
        opcode: 94,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "RDR8",
        opcode: 95,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "LDG",
        opcode: 96,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "LDV",
        opcode: 97,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "PGA",
        opcode: 98,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 2,
    },
    OpInfo {
        name: "CmpPtr",
        opcode: 99,
        fmt: BcType::rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "VAR",
        opcode: 100,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 2,
    },
    OpInfo {
        name: "iTOf",
        opcode: 101,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "fTOi",
        opcode: 102,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "uTOf",
        opcode: 103,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "fTOu",
        opcode: 104,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "sbTOi",
        opcode: 105,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "swTOi",
        opcode: 106,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "ubTOi",
        opcode: 107,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "uwTOi",
        opcode: 108,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "dTOi",
        opcode: 109,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "dTOu",
        opcode: 110,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "dTOf",
        opcode: 111,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "iTOd",
        opcode: 112,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "uTOd",
        opcode: 113,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "fTOd",
        opcode: 114,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "ADDi",
        opcode: 115,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SUBi",
        opcode: 116,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MULi",
        opcode: 117,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "DIVi",
        opcode: 118,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MODi",
        opcode: 119,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "ADDf",
        opcode: 120,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SUBf",
        opcode: 121,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MULf",
        opcode: 122,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "DIVf",
        opcode: 123,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MODf",
        opcode: 124,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "ADDd",
        opcode: 125,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SUBd",
        opcode: 126,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MULd",
        opcode: 127,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "DIVd",
        opcode: 128,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MODd",
        opcode: 129,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "ADDIi",
        opcode: 130,
        fmt: BcType::wW_rW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "SUBIi",
        opcode: 131,
        fmt: BcType::wW_rW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "MULIi",
        opcode: 132,
        fmt: BcType::wW_rW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "ADDIf",
        opcode: 133,
        fmt: BcType::wW_rW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "SUBIf",
        opcode: 134,
        fmt: BcType::wW_rW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "MULIf",
        opcode: 135,
        fmt: BcType::wW_rW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "SetG4",
        opcode: 136,
        fmt: BcType::QW_DW_ARG,
        size_dwords: 4,
        stack_inc: 0,
    },
    OpInfo {
        name: "ChkRefS",
        opcode: 137,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "ChkNullV",
        opcode: 138,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CALLINTF",
        opcode: 139,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: STACK_INC_VARIABLE,
    },
    OpInfo {
        name: "iTOb",
        opcode: 140,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "iTOw",
        opcode: 141,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SetV1",
        opcode: 142,
        fmt: BcType::wW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SetV2",
        opcode: 143,
        fmt: BcType::wW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "Cast",
        opcode: 144,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: -2,
    },
    OpInfo {
        name: "i64TOi",
        opcode: 145,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "uTOi64",
        opcode: 146,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "iTOi64",
        opcode: 147,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "fTOi64",
        opcode: 148,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "dTOi64",
        opcode: 149,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "fTOu64",
        opcode: 150,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "dTOu64",
        opcode: 151,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "i64TOf",
        opcode: 152,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "u64TOf",
        opcode: 153,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "i64TOd",
        opcode: 154,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "u64TOd",
        opcode: 155,
        fmt: BcType::wW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "NEGi64",
        opcode: 156,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "INCi64",
        opcode: 157,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "DECi64",
        opcode: 158,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "BNOT64",
        opcode: 159,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "ADDi64",
        opcode: 160,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SUBi64",
        opcode: 161,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MULi64",
        opcode: 162,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "DIVi64",
        opcode: 163,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MODi64",
        opcode: 164,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BAND64",
        opcode: 165,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BOR64",
        opcode: 166,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BXOR64",
        opcode: 167,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BSLL64",
        opcode: 168,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BSRL64",
        opcode: 169,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "BSRA64",
        opcode: 170,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPi64",
        opcode: 171,
        fmt: BcType::rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "CMPu64",
        opcode: 172,
        fmt: BcType::rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "ChkNullS",
        opcode: 173,
        fmt: BcType::W_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "ClrHi",
        opcode: 174,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "JitEntry",
        opcode: 175,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "CallPtr",
        opcode: 176,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: STACK_INC_VARIABLE,
    },
    OpInfo {
        name: "FuncPtr",
        opcode: 177,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: 2,
    },
    OpInfo {
        name: "LoadThisR",
        opcode: 178,
        fmt: BcType::W_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "PshV8",
        opcode: 179,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 2,
    },
    OpInfo {
        name: "DIVu",
        opcode: 180,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MODu",
        opcode: 181,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "DIVu64",
        opcode: 182,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "MODu64",
        opcode: 183,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "LoadRObjR",
        opcode: 184,
        fmt: BcType::rW_W_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "LoadVObjR",
        opcode: 185,
        fmt: BcType::rW_W_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "RefCpyV",
        opcode: 186,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: -2,
    },
    OpInfo {
        name: "JLowZ",
        opcode: 187,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "JLowNZ",
        opcode: 188,
        fmt: BcType::DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "AllocMem",
        opcode: 189,
        fmt: BcType::wW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "SetListSize",
        opcode: 190,
        fmt: BcType::rW_DW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "PshListElmnt",
        opcode: 191,
        fmt: BcType::rW_DW_ARG,
        size_dwords: 2,
        stack_inc: 2,
    },
    OpInfo {
        name: "SetListType",
        opcode: 192,
        fmt: BcType::rW_DW_DW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "POWi",
        opcode: 193,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "POWu",
        opcode: 194,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "POWf",
        opcode: 195,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "POWd",
        opcode: 196,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "POWdi",
        opcode: 197,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "POWi64",
        opcode: 198,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "POWu64",
        opcode: 199,
        fmt: BcType::wW_rW_rW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "Thiscall1",
        opcode: 200,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: -3,
    },
    OpInfo {
        name: "FinConstruct",
        opcode: 201,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: -2,
    },
    OpInfo {
        name: "DestructScript",
        opcode: 202,
        fmt: BcType::rW_QW_ARG,
        size_dwords: 3,
        stack_inc: 0,
    },
    OpInfo {
        name: "CopyScript",
        opcode: 203,
        fmt: BcType::QW_ARG,
        size_dwords: 3,
        stack_inc: -2,
    },
    OpInfo {
        name: "ResolveObjectPtr",
        opcode: 204,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "FreeNullV8",
        opcode: 205,
        fmt: BcType::wW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "TrackRef",
        opcode: 206,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "UntrackRef",
        opcode: 207,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "ValidateRef",
        opcode: 208,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CpyVtoR1",
        opcode: 209,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "SaveReturnValue",
        opcode: 210,
        fmt: BcType::NO_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "CmpPtrNull",
        opcode: 211,
        fmt: BcType::rW_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "ThrowException",
        opcode: 212,
        fmt: BcType::W_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
];

/// Temporary compiler tokens (251..=255). These never appear in serialized
/// bytecode; included for completeness only.
pub const TEMP_TOKENS: [OpInfo; 5] = [
    OpInfo {
        name: "VarDecl",
        opcode: 251,
        fmt: BcType::W_ARG,
        size_dwords: 1,
        stack_inc: 0,
    },
    OpInfo {
        name: "Block",
        opcode: 252,
        fmt: BcType::INFO,
        size_dwords: 0,
        stack_inc: 0,
    },
    OpInfo {
        name: "ObjInfo",
        opcode: 253,
        fmt: BcType::rW_DW_ARG,
        size_dwords: 2,
        stack_inc: 0,
    },
    OpInfo {
        name: "LINE",
        opcode: 254,
        fmt: BcType::INFO,
        size_dwords: 0,
        stack_inc: 0,
    },
    OpInfo {
        name: "LABEL",
        opcode: 255,
        fmt: BcType::INFO,
        size_dwords: 0,
        stack_inc: 0,
    },
];

/// Look up an opcode by its byte value. Returns `None` for the unused range
/// 213..=250 (`asBCINFO_DUMMY`). Covers both real opcodes and temp tokens.
pub fn op_info(opcode: u8) -> Option<&'static OpInfo> {
    OPCODES
        .iter()
        .chain(TEMP_TOKENS.iter())
        .find(|o| o.opcode == opcode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcodes_are_contiguous_0_to_212() {
        for (i, o) in OPCODES.iter().enumerate() {
            assert_eq!(o.opcode as usize, i, "opcode {} out of order", o.name);
        }
        assert_eq!(OPCODES.len(), 213);
        assert_eq!(OPCODES.last().unwrap().opcode, 212);
    }

    #[test]
    fn size_matches_fmt() {
        for o in OPCODES.iter().chain(TEMP_TOKENS.iter()) {
            assert_eq!(o.size_dwords, o.fmt.size_dwords(), "{}", o.name);
        }
    }

    #[test]
    fn spot_check_known_opcodes() {
        // RET = 10, W_ARG, 1 dword, variable stack.
        let ret = op_info(10).unwrap();
        assert_eq!(ret.name, "RET");
        assert_eq!(ret.fmt, BcType::W_ARG);
        assert_eq!(ret.size_dwords, 1);
        assert_eq!(ret.stack_inc, STACK_INC_VARIABLE);
        // SUSPEND = 63, NO_ARG, 1 dword.
        assert_eq!(op_info(63).unwrap().name, "SUSPEND");
        // PshC8 = 47, QW_ARG, 3 dwords (64-bit immediate).
        let p = op_info(47).unwrap();
        assert_eq!(p.fmt, BcType::QW_ARG);
        assert_eq!(p.size_dwords, 3);
        // ALLOC = 64, QW_DW_ARG (PTR_DW on 64-bit) = 4 dwords.
        let alloc = op_info(64).unwrap();
        assert_eq!(alloc.name, "ALLOC");
        assert_eq!(alloc.fmt, BcType::QW_DW_ARG);
        assert_eq!(alloc.size_dwords, 4);
        // 213..=250 are unused.
        assert!(op_info(213).is_none());
        assert!(op_info(250).is_none());
    }
}
