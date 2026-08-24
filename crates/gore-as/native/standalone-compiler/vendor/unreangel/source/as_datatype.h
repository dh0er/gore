/*
   AngelCode Scripting Library
   Copyright (c) 2003-2016 Andreas Jonsson

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
// as_datatype.h
//
// This class describes the datatype for expressions during compilation
//



#ifndef AS_DATATYPE_H
#define AS_DATATYPE_H

#include "as_tokendef.h"
#include "as_string.h"

BEGIN_AS_NAMESPACE

struct asSTypeBehaviour;
class asCScriptEngine;
class asCTypeInfo;
class asCScriptFunction;
class asCModule;
class asCObjectType;
class asCEnumType;
struct asSNameSpace;

// TODO: refactor: Reference should not be part of the datatype. This should be stored separately, e.g. in asCExprValue
//                 MakeReference, MakeReadOnly, IsReference, IsReadOnly should be removed

class ANGELSCRIPTCODE_API asCDataType
{
public:
	asCDataType();
	asCDataType(const asCDataType &);
	~asCDataType();

	bool IsValid() const;

	asCString Format(asSNameSpace *currNs, bool includeNamespace = false, bool forDisplay = false) const;

	static asCDataType CreatePrimitive(eTokenType tt, bool isConst);
	static asCDataType CreateType(asCTypeInfo *ti, bool isConst);
	static asCDataType CreateAuto(bool isConst);
	static asCDataType CreateObjectHandle(asCTypeInfo *ot, bool isConst);
	static asCDataType CreateNullHandle();

	int MakeHandle(bool b, bool acceptHandleForScope = false);
	int MakeArray(asCScriptEngine *engine, asCModule *requestingModule);
	int MakeReference(bool b);
	int MakeReadOnly(bool b);
	int MakeHandleToConst(bool b);
	void MakeUnresolvedObject(bool b) { isUnresolvedObject = b; }
	void SetIfHandleThenConst(bool b) { ifHandleThenConst = b; }
	bool HasIfHandleThenConst() const { return ifHandleThenConst; }
	void SetAutoConstRef(bool b) { autoConstRef = b; }
	bool HasAutoConstRef() const { return autoConstRef; }

	void MakeConstValueOrConstObject(bool b);

	bool IsTemplate()             const;
	bool IsScriptObject()         const;
	bool IsPrimitive()            const;
	bool IsMathType()             const;
	bool IsObject()               const;
	bool IsReference()            const {return isReference;}
	bool IsAuto()                 const {return isAuto;}
	bool IsReadOnly()             const;
	bool IsIntegerType()          const;
	bool IsUnsignedType()         const;
	bool IsFloat32Type()            const;
	bool IsFloat64Type()           const;
	bool IsBooleanType()          const;
	bool IsObjectHandle()         const {return isObjectHandle;}
	bool IsHandleToAuto()         const {return isAuto && isObjectHandle;}
	bool IsHandleToConst()        const;
	bool IsArrayType()            const;
	bool IsEnumType()             const;
	bool IsAnyType()              const {return tokenType == ttQuestion;}
	bool IsHandleToAsHandleType() const {return isHandleToAsHandleType;}
	bool IsAbstractClass()        const;
	bool IsInterface()            const;
	bool IsFuncdef()              const;
	bool IsUnresolvedObject()     const { return isUnresolvedObject; }

	void SetFloatExtendedToDouble(bool b) { isFloatExtendedToDouble = b; }
	bool IsFloatExtendedToDouble()     const { return isFloatExtendedToDouble; }

	bool IsReferenceType() const;
	bool IsNonPrimitiveValueType() const;

	bool IsObjectConst()    const;

	bool IsEqualExceptRef(const asCDataType &)             const;
	bool IsEqualExceptTypeInfo(const asCDataType &)             const;
	bool IsEqualExceptRefAndConst(const asCDataType &)     const;
	bool IsEqualExceptConst(const asCDataType &)           const;
	bool IsNullHandle()                                    const;

	bool SupportHandles() const;
	bool CanBeInstantiated() const;
	bool CanBeCopied() const;

	bool operator ==(const asCDataType &) const;
	bool operator !=(const asCDataType &) const;

	asCDataType        GetSubType(asUINT subtypeIndex = 0)    const;
	eTokenType         GetTokenType()  const {return tokenType;}
	asCTypeInfo       *GetTypeInfo() const { return typeInfo; }

	int  GetSizeOnStackDWords()  const;
	int  GetSizeInMemoryBytes()  const;
	int  GetSizeInMemoryDWords() const;
	int  GetSizeOfVariableBytes()  const;
	int  GetAlignment()          const;

	void SetTokenType(eTokenType tt)         {tokenType = tt;}
	void SetTypeInfo(asCTypeInfo *ti)       {typeInfo = ti;}

	asCDataType &operator =(const asCDataType &);

	asSTypeBehaviour *GetBehaviour() const;

	void SetIsErrorDataType(bool b) { isErrorDataType = b; }
	bool IsErrorDataType() const { return isErrorDataType; }

	void SetAnyImplicitIntegerType(bool b) { anyImplicitIntegerType = b; }
	bool IsAnyImplicitIntegerType() const { return anyImplicitIntegerType; }

	static bool floatIsFloat64;

protected:
	// Base object type
	eTokenType tokenType;

	// Behaviour type
	asCTypeInfo *typeInfo;

	// Top level
	bool isReference:1;
	bool isReadOnly:1;
	bool isObjectHandle:1;
	bool isConstHandle:1;
	bool isAuto:1;
	bool isHandleToAsHandleType:1; // Used by the compiler to know how to initialize the object
	bool ifHandleThenConst:1; // Used when creating template instances to determine if a handle should be const or not
	bool autoConstRef:1;
	bool isUnresolvedObject:1; // Used when object pointers need to be resolved before use
	bool isErrorDataType:1; // Used when the datatype was replaced because of a lookup error
	bool isFloatExtendedToDouble : 1;
	bool anyImplicitIntegerType : 1;
};

struct asSAccessPermission
{
	asCString accessName;
	bool bInherited = false;
	bool bReadOnly = false;
	bool bEditDefaults = false;
};

struct asSAccessSpecifier
{
	asCString name;

	bool bIsPrivate = false;
	bool bIsProtected = false;

	bool bAnyReadOnly = false;
	bool bAnyEditDefaults = false;

	asCArray<asSAccessPermission> permissions;

	void GetAllowedAccess(bool bAccessedFromDerivedClass, class asCScriptFunction* accessFromFunction, bool& Read, bool& Write, bool& Edit);
};

END_AS_NAMESPACE

#endif
