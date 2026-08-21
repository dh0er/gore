/*
   AngelCode Scripting Library
   Copyright (c) 2003-2017 Andreas Jonsson

   This software is provided 'as-is', without any express or implied
   warranty. In no event will the authors be held liable for any
   damages arising from the use of this software.

   Permission is granted to anyone to use this software for any
   purpose, including commercial applications, and to alter it and
   redistribute it freely, subject to the following restrictions:

   1. The origin of this software must not be misrepresented; you
      must not claim that you wrote the original software. If you use
      this software in a product, an acknowledgment in the product
      documentation would be appreciated but is not required.

   2. Altered source versions must be plainly marked as such, and
      must not be misrepresented as being the original software.

   3. This notice may not be removed or altered from any source
      distribution.

   The original version of this library can be located at:
   http://www.angelcode.com/angelscript/

   Andreas Jonsson
   andreas@angelcode.com
*/


//
// as_tokendef.h
//
// Definitions for tokens identifiable by the tokenizer
//


#ifndef AS_TOKENDEF_H
#define AS_TOKENDEF_H

#include "as_config.h"

BEGIN_AS_NAMESPACE

enum eTokenType
{
	ttUnrecognizedToken,

	ttEnd,                 // End of file

	// White space and comments
	ttWhiteSpace,          // ' ', '\t', '\r', '\n', UTF8 byte-order-mark
	ttOnelineComment,      // // \n
	ttMultilineComment,    // /* */

	// Atoms
	ttIdentifier,                  // abc123
	ttIntConstant,                 // 1234
	ttFloat32Constant,               // 12.34e56f
	ttFloat64Constant,              // 12.34e56
	ttStringConstant,              // "123"
	ttMultilineStringConstant,     //
	ttHeredocStringConstant,       // """text"""
	ttNonTerminatedStringConstant, // "123
	ttBitsConstant,                // 0xFFFF

	// Math operators
	ttPlus,                // +
	ttMinus,               // -
	ttStar,                // *
	ttSlash,               // /
	ttPercent,             // %
	ttStarStar,            // **

	ttHandle,              // @

	ttAddAssign,           // +=
	ttSubAssign,           // -=
	ttMulAssign,           // *=
	ttDivAssign,           // /=
	ttModAssign,           // %=
	ttPowAssign,           // **=

	ttOrAssign,            // |=
	ttAndAssign,           // &=
	ttXorAssign,           // ^=
	ttShiftLeftAssign,     // <<=
	ttShiftRightLAssign,   // >>=
	ttShiftRightAAssign,   // >>>=

	ttInc,                 // ++
	ttDec,                 // --

	ttDot,                 // .
	ttScope,               // ::

	// Statement tokens
	ttAssignment,          // =
	ttEndStatement,        // ;
	ttListSeparator,       // ,
	ttStartStatementBlock, // {
	ttEndStatementBlock,   // }
	ttOpenParanthesis,     // (
	ttCloseParanthesis,    // )
	ttOpenBracket,         // [
	ttCloseBracket,        // ]
	ttAmp,                 // &

	// Bitwise operators
	ttBitOr,               // |
	ttBitNot,              // ~
	ttBitXor,              // ^
	ttBitShiftLeft,        // <<
	ttBitShiftRight,       // >>     // TODO: In Java this is the arithmetical shift
	ttBitShiftRightArith,  // >>>    // TODO: In Java this is the logical shift

	// Compare operators
	ttEqual,               // ==
	ttNotEqual,            // !=
	ttLessThan,            // <
	ttGreaterThan,         // >
	ttLessThanOrEqual,     // <=
	ttGreaterThanOrEqual,  // >=

	ttQuestion,            // ?
	ttColon,               // :

	// Reserved keywords
	ttIf,                  // if
	ttElse,                // else
	ttFor,                 // for
	ttWhile,               // while
	ttBool,                // bool
	ttFuncDef,             // funcdef
	ttImport,              // import
	ttInt,                 // int
	ttInt8,                // int8
	ttInt16,               // int16
	ttInt64,               // int64
	ttInterface,           // interface
	ttIs,                  // is
	ttNotIs,               // !is
	ttUInt,                // uint
	ttUInt8,               // uint8
	ttUInt16,              // uint16
	ttUInt64,              // uint64
	ttFloat,               // float
	ttFloat32,             // float32
	ttFloat64,             // float64
	ttVoid,                // void
	ttTrue,                // true
	ttFalse,               // false
	ttReturn,              // return
	ttNot,                 // not
	ttAnd,                 // and, &&
	ttOr,                  // or, ||
	ttXor,                 // xor, ^^
	ttBreak,               // break
	ttContinue,            // continue
	ttConst,               // const
	ttDo,                  // do
	ttDouble,              // double
	ttSwitch,              // switch
	ttCase,                // case
	ttDefault,             // default
	ttIn,                  // in
	ttOut,                 // out
	ttInOut,               // inout
	ttNull,                // null
	ttClass,               // class
	ttTypedef,             // typedef
	ttEnum,                // enum
	ttCast,                // cast
	ttPrivate,             // private
	ttProtected,           // protected
	ttNamespace,           // namespace
	ttMixin,               // mixin
	ttAuto,                // auto
	ttStruct,              // struct
	ttLocal,               // local
	ttFallthrough,         // fallthrough

	ttAccess,
	ttUnresolvedObject,
};

#define ttHandleSuffix ttHandle

struct sTokenWord
{
	const char *word;
	size_t      wordLength;
	eTokenType  tokenType;
};

#define asTokenDef(str, tok) {str, sizeof(str)-1, tok}

sTokenWord const tokenWords[] =
{
	asTokenDef("+"         , ttPlus),
	asTokenDef("+="        , ttAddAssign),
	asTokenDef("++"        , ttInc),
	asTokenDef("-"         , ttMinus),
	asTokenDef("-="        , ttSubAssign),
	asTokenDef("--"        , ttDec),
	asTokenDef("*"         , ttStar),
	asTokenDef("*="        , ttMulAssign),
	asTokenDef("/"         , ttSlash),
	asTokenDef("/="        , ttDivAssign),
	asTokenDef("%"         , ttPercent),
	asTokenDef("%="        , ttModAssign),
	asTokenDef("**"        , ttStarStar),
	asTokenDef("**="       , ttPowAssign),
	asTokenDef("="         , ttAssignment),
	asTokenDef("=="        , ttEqual),
	asTokenDef("."         , ttDot),
	asTokenDef("|"         , ttBitOr),
	asTokenDef("|="        , ttOrAssign),
	asTokenDef("||"        , ttOr),
	asTokenDef("&"         , ttAmp),
	asTokenDef("&="        , ttAndAssign),
	asTokenDef("&&"        , ttAnd),
	asTokenDef("^"         , ttBitXor),
	asTokenDef("^="        , ttXorAssign),
	asTokenDef("^^"        , ttXor),
	asTokenDef("<"         , ttLessThan),
	asTokenDef("<="        , ttLessThanOrEqual),
	asTokenDef("<<"        , ttBitShiftLeft),
	asTokenDef("<<="       , ttShiftLeftAssign),
	asTokenDef(">"         , ttGreaterThan),
	asTokenDef(">="        , ttGreaterThanOrEqual),
	asTokenDef(">>"        , ttBitShiftRight),
	asTokenDef(">>="       , ttShiftRightLAssign),
	asTokenDef(">>>"       , ttBitShiftRightArith),
	asTokenDef(">>>="      , ttShiftRightAAssign),
	asTokenDef("~"         , ttBitNot),
	asTokenDef(";"         , ttEndStatement),
	asTokenDef(","         , ttListSeparator),
	asTokenDef("{"         , ttStartStatementBlock),
	asTokenDef("}"         , ttEndStatementBlock),
	asTokenDef("("         , ttOpenParanthesis),
	asTokenDef(")"         , ttCloseParanthesis),
	asTokenDef("["         , ttOpenBracket),
	asTokenDef("]"         , ttCloseBracket),
	asTokenDef("?"         , ttQuestion),
	asTokenDef(":"         , ttColon),
	asTokenDef("::"        , ttScope),
	asTokenDef("!"         , ttNot),
	asTokenDef("!="        , ttNotEqual),
	//asTokenDef("!is"       , ttNotIs),
	//asTokenDef("@"         , ttHandle),
	//asTokenDef("and"       , ttAnd),
	asTokenDef("access"    , ttAccess),
	asTokenDef("auto"      , ttAuto),
	asTokenDef("bool"      , ttBool),
	asTokenDef("break"     , ttBreak),
	asTokenDef("case"      , ttCase), 
	asTokenDef("Cast"      , ttCast),
	asTokenDef("class"     , ttClass),
	asTokenDef("const"     , ttConst),
	asTokenDef("continue"  , ttContinue),
	asTokenDef("default"   , ttDefault),
	asTokenDef("do"        , ttDo),
	asTokenDef("double"    , ttDouble),
	asTokenDef("else"      , ttElse),
	asTokenDef("enum"      , ttEnum),
	asTokenDef("false"     , ttFalse),
	asTokenDef("float"     , ttFloat),
	asTokenDef("float32"   , ttFloat32),
	asTokenDef("float64"   , ttFloat64),
	asTokenDef("for"       , ttFor),
	//asTokenDef("funcdef"   , ttFuncDef),
	asTokenDef("if"        , ttIf),
	asTokenDef("import"    , ttImport),
	asTokenDef("in"        , ttIn),
	asTokenDef("inout"     , ttInOut),
	asTokenDef("int"       , ttInt),
	asTokenDef("int8"      , ttInt8),
	asTokenDef("int16"     , ttInt16),
	asTokenDef("int32"     , ttInt),
	asTokenDef("int64"     , ttInt64),
	//asTokenDef("interface" , ttInterface),
	//asTokenDef("is"        , ttIs),
	asTokenDef("local"     , ttLocal),
	asTokenDef("mixin"     , ttMixin),
	asTokenDef("namespace" , ttNamespace),
	//asTokenDef("not"       , ttNot),
	asTokenDef("nullptr"   , ttNull),
	//asTokenDef("or"        , ttOr),
	asTokenDef("out"       , ttOut),
	asTokenDef("private"   , ttPrivate),
	asTokenDef("protected" , ttProtected),
	asTokenDef("return"    , ttReturn),
	asTokenDef("struct"    , ttStruct),
	asTokenDef("switch"    , ttSwitch),
	asTokenDef("true"      , ttTrue),
	//asTokenDef("typedef"   , ttTypedef),
	asTokenDef("uint"      , ttUInt),
	asTokenDef("uint8"     , ttUInt8),
	asTokenDef("uint16"    , ttUInt16),
	asTokenDef("uint32"    , ttUInt),
	asTokenDef("uint64"    , ttUInt64),
	asTokenDef("unresolved_object" , ttUnresolvedObject),
	asTokenDef("void"      , ttVoid),
	asTokenDef("while"     , ttWhile),
	//asTokenDef("xor"       , ttXor),
	asTokenDef("fallthrough", ttFallthrough),
};

const unsigned int numTokenWords = sizeof(tokenWords)/sizeof(sTokenWord);

// Some keywords that are not considered tokens by the parser
// These only have meaning in specific situations. Outside these
// situations they are treated as normal identifiers.
const char * const THIS_TOKEN      = "this";
const char * const FROM_TOKEN      = "from";
const char * const SUPER_TOKEN     = "super";
const char * const SHARED_TOKEN    = "shared";
const char * const FINAL_TOKEN     = "final";
const char * const TIDY_TOKEN      = "tidy";
const char * const OVERRIDE_TOKEN  = "override";
const char * const GET_TOKEN       = "get";
const char * const SET_TOKEN       = "set";
const char * const ABSTRACT_TOKEN  = "abstract";
const char * const FUNCTION_TOKEN  = "function";
const char * const IF_HANDLE_TOKEN = "if_handle_then_const";
const char * const HANDLE_ONLY_TOKEN = "handle_only";
const char * const AUTO_CONSTREF_TOKEN = "__auto_constref_type";
const char * const ANY_IMPLICIT_INTEGER_TOKEN = "__any_implicit_integer";
const char * const EXTERNAL_TOKEN  = "external";
const char * const PROPERTY_TOKEN  = "property";
const char * const MIXIN_TOKEN  = "mixin";
const char * const READONLY_TOKEN  = "readonly";
const char * const EDITDEFAULTS_TOKEN  = "editdefaults";
const char * const INHERITED_TOKEN  = "inherited";
const char * const ACCEPT_TEMPORARY_TOKEN     = "accept_temporary_this";
const char * const EXTERNAL_IMPLICIT_THIS_TOKEN     = "external_implicit_this";
const char * const NO_DISCARD_TOKEN     = "no_discard";
const char * const ALLOW_DISCARD_TOKEN     = "allow_discard";
const char * const GENERATED_FUNCTION_TOKEN     = "__generated";
const char * const DEPRECATED_TOKEN     = "deprecated";

END_AS_NAMESPACE

#endif
