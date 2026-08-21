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
// as_scriptobject.h
//
// A generic class for handling script declared structures
//



#ifndef AS_SCRIPTOBJECT_H
#define AS_SCRIPTOBJECT_H

#include "as_config.h"
#include "as_atomic.h"

BEGIN_AS_NAMESPACE

class ANGELSCRIPTCODE_API asCScriptObject : public asIScriptObject
{
public:
	asCScriptObject(class asCObjectType *objType, bool doInitialize = true);

	asCScriptObject& PerformCopy(asCScriptObject* OtherObject, asCObjectType* objType, asCObjectType* otherObjType);
	void CopyStruct(asCScriptObject* SourceObject, asCObjectType* ObjectType);
};

// TODO: Add const overload for GetAddressOfProperty

// TODO: weak: Should move to its own file
class asCLockableSharedBool : public asILockableSharedBool
{
public:
	asCLockableSharedBool();
	int AddRef() const;
	int Release() const;

	bool Get() const;
	void Set(bool);

	void Lock() const;
	void Unlock() const;

protected:
	mutable asCAtomic refCount;
	bool      value;
	DECLARECRITICALSECTION(mutable lock)
};

void ScriptObject_Construct(asCObjectType *objType, asCScriptObject *self);
asCScriptObject &ScriptObject_Assignment(asCScriptObject *other, asCScriptObject *self);

void ScriptObject_ConstructUnitialized(asCObjectType *objType, asCScriptObject *self);

void RegisterScriptObject(asCScriptEngine *engine);

asIScriptObject *ScriptObjectFactory(const asCObjectType *objType, asCScriptEngine *engine);
void ScriptObjectDefaultConstructor(const asCObjectType *objType, asCScriptEngine *engine, void* ObjectPtr);

END_AS_NAMESPACE

#endif
