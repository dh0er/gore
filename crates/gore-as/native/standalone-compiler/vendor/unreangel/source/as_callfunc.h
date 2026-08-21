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
// as_callfunc.h
//
// These functions handle the actual calling of system functions
//


#ifndef AS_CALLFUNC_H
#define AS_CALLFUNC_H

#include "as_array.h"

BEGIN_AS_NAMESPACE

class asCContext;
class asCScriptEngine;
class asCScriptFunction;
class asCObjectType;
struct asSSystemFunctionInterface;

int DetectCallingConvention(bool isMethod, const asSFuncPtr &ptr, int callConv, void *auxiliary, asFunctionCaller caller, asSSystemFunctionInterface *internal);

int PrepareSystemFunctionGeneric(asCScriptFunction *func, asSSystemFunctionInterface *internal, asCScriptEngine *engine);

int PrepareSystemFunction(asCScriptFunction *func, asSSystemFunctionInterface *internal, asCScriptEngine *engine);

int CallSystemFunction(int id, asCContext *context);

inline asPWORD FuncPtrToUInt(asFUNCTION_t func)
{
	// A little trickery as the C++ standard doesn't allow direct
	// conversion between function pointer and data pointer
	union { asFUNCTION_t func; asPWORD idx; } u;
	u.func = func;

	return u.idx;
}

enum internalCallConv
{
	ICC_GENERIC_FUNC,
	ICC_GENERIC_FUNC_RETURNINMEM, // never used
	ICC_CDECL,
	ICC_CDECL_RETURNINMEM,
	ICC_STDCALL,
	ICC_STDCALL_RETURNINMEM,
	ICC_THISCALL,
	ICC_THISCALL_RETURNINMEM,
	ICC_VIRTUAL_THISCALL,
	ICC_VIRTUAL_THISCALL_RETURNINMEM,
	ICC_CDECL_OBJLAST,
	ICC_CDECL_OBJLAST_RETURNINMEM,
	ICC_CDECL_OBJFIRST,
	ICC_CDECL_OBJFIRST_RETURNINMEM,
	ICC_GENERIC_METHOD,
	ICC_GENERIC_METHOD_RETURNINMEM, // never used
	ICC_THISCALL_OBJLAST,
	ICC_THISCALL_OBJLAST_RETURNINMEM,
	ICC_VIRTUAL_THISCALL_OBJLAST,
	ICC_VIRTUAL_THISCALL_OBJLAST_RETURNINMEM,
	ICC_THISCALL_OBJFIRST,
	ICC_THISCALL_OBJFIRST_RETURNINMEM,
	ICC_VIRTUAL_THISCALL_OBJFIRST,
	ICC_VIRTUAL_THISCALL_OBJFIRST_RETURNINMEM
};

enum class asEFirstParamMetaData : uint8
{
	None = 0,
	ScriptFunction,
	ScriptObjectType,
};

struct asSSystemFunctionInterface
{
	asFUNCTION_t         func;
	asMETHOD_t           method;
	int                  baseOffset;
	internalCallConv     callConv;
	int                  scriptReturnSize;
	int                  paramSize;
	asEFirstParamMetaData passFirstParamMetaData;
	asFunctionCaller	 caller;

	struct SClean
	{
		asCObjectType *ot; // argument type for clean up
		short op;          // clean up operation: 0 = release, 1 = free, 2 = destruct then free
		short off;         // argument offset on the stack
	};

	asSSystemFunctionInterface() : func(0), method(0), baseOffset(0), callConv(ICC_GENERIC_FUNC), scriptReturnSize(0), paramSize(0), passFirstParamMetaData(asEFirstParamMetaData::None) {}

	asSSystemFunctionInterface(const asSSystemFunctionInterface &in)
	{
		*this = in;
	}

	asSSystemFunctionInterface &operator=(const asSSystemFunctionInterface &in)
	{
		func                = in.func;
		method              = in.method;
		baseOffset          = in.baseOffset;
		callConv            = in.callConv;
		scriptReturnSize    = in.scriptReturnSize;
		paramSize           = in.paramSize;
		passFirstParamMetaData = in.passFirstParamMetaData;
		caller = in.caller;
		return *this;
	}
};

END_AS_NAMESPACE

#endif

