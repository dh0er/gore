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
// as_configgroup.cpp
//
// This class holds configuration groups for the engine
//



#include "as_configgroup.h"
#include "as_config.h"
#include "as_scriptengine.h"
#include "as_texts.h"

BEGIN_AS_NAMESPACE

asCConfigGroup::asCConfigGroup()
{
	refCount = 0;
}

asCConfigGroup::~asCConfigGroup()
{
}

int asCConfigGroup::AddRef()
{
	refCount++;
	return refCount;
}

int asCConfigGroup::Release()
{
	// Don't delete the object here, the engine will delete the object when ready
	refCount--;
	return refCount;
}

asCTypeInfo *asCConfigGroup::FindType(const char *obj)
{
	return typesByName.FindFirst(obj);
}

void asCConfigGroup::AddType(asCTypeInfo* type)
{
	types.PushLast(type);
	typesByName.Add(type);
}

void asCConfigGroup::RefConfigGroup(asCConfigGroup *group)
{
	if( group == this || group == 0 ) return;

	// Verify if the group is already referenced
	for( asUINT n = 0; n < referencedConfigGroups.GetLength(); n++ )
		if( referencedConfigGroups[n] == group )
			return;

	referencedConfigGroups.PushLast(group);
	group->AddRef();
}

void asCConfigGroup::AddReferencesForFunc(asCScriptEngine *engine, asCScriptFunction *func)
{
	AddReferencesForType(engine, func->returnType.GetTypeInfo());
	for( asUINT n = 0; n < func->parameterTypes.GetLength(); n++ )
		AddReferencesForType(engine, func->parameterTypes[n].GetTypeInfo());
}

void asCConfigGroup::AddReferencesForType(asCScriptEngine *engine, asCTypeInfo *type)
{
	if( type == 0 ) return;

	// Keep reference to other groups
	RefConfigGroup(engine->FindConfigGroupForTypeInfo(type));
}

bool asCConfigGroup::HasLiveObjects()
{
	for( asUINT n = 0; n < types.GetLength(); n++ )
		if( types[n]->externalRefCount.get() != 0 )
			return true;

	return false;
}

void asCConfigGroup::RemoveConfiguration(asCScriptEngine *engine, bool notUsed)
{
}

END_AS_NAMESPACE
