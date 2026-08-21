/*
   AngelCode Scripting Library
   Copyright (c) 2003-2018 Andreas Jonsson

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


#include "as_scriptobject.h"
#include "as_config.h"
#include "as_scriptengine.h"
#include "as_texts.h"
#include <new>

BEGIN_AS_NAMESPACE

// This helper function will call the default factory, that is a script function
asIScriptObject *ScriptObjectFactory(const asCObjectType *objType, asCScriptEngine *engine)
{
	asIScriptContext *ctx = 0;
	int r = 0;
	bool isNested = false;

	// Use nested call in the context if there is an active context
	ctx = asGetUsableContext();
	if( ctx )
	{
		// It may not always be possible to reuse the current context, 
		// in which case we'll have to create a new one any way.
		if( ctx->GetEngine() == objType->GetEngine() && ctx->PushState() == asSUCCESS )
			isNested = true;
		else
			ctx = 0;
	}
	
	if( ctx == 0 )
	{
		// Request a context from the engine
		ctx = engine->RequestContext();
		if( ctx == 0 )
		{
			// TODO: How to best report this failure?
			return 0;
		}
	}

	r = ctx->Prepare(engine->scriptFunctions[objType->beh.factory]);
	if( r < 0 )
	{
		if( isNested )
			ctx->PopState();
		else
			engine->ReturnContext(ctx);
		return 0;
	}

	for(;;)
	{
		r = ctx->Execute();

		// We can't allow this execution to be suspended 
		// so resume the execution immediately
		if( r != asEXECUTION_SUSPENDED )
			break;
	}

	if( r != asEXECUTION_FINISHED )
	{
		if( isNested )
		{
			ctx->PopState();

			// If the execution was aborted or an exception occurred,
			// then we should forward that to the outer execution.
			if( r == asEXECUTION_EXCEPTION )
			{
				// TODO: How to improve this exception
				ctx->SetException(TXT_EXCEPTION_IN_NESTED_CALL);
			}
			else if( r == asEXECUTION_ABORTED )
				ctx->Abort();
		}
		else
			engine->ReturnContext(ctx);
		return 0;
	}

	asIScriptObject *ptr = (asIScriptObject*)ctx->GetReturnAddress();

	// Increase the reference, because the context will release its pointer
	//ptr->AddRef();

	if( isNested )
		ctx->PopState();
	else
		engine->ReturnContext(ctx);

	return ptr;
}

void ScriptObjectDefaultConstructor(const asCObjectType *objType, asCScriptEngine *engine, void* ObjectPtr)
{
	if (objType->beh.construct == 0)
		return;

	asCScriptFunction* func = engine->scriptFunctions[objType->beh.construct];
	if (func->funcType == asEFuncType::asFUNC_SYSTEM)
	{
		engine->CallObjectMethod(ObjectPtr, objType->beh.construct);
		return;
	}

	asIScriptContext *ctx = 0;
	int r = 0;
	bool isNested = false;

	// Use nested call in the context if there is an active context
	ctx = asGetUsableContext();
	if( ctx )
	{
		// It may not always be possible to reuse the current context, 
		// in which case we'll have to create a new one any way.
		if( ctx->GetEngine() == objType->GetEngine() && ctx->PushState() == asSUCCESS )
			isNested = true;
		else
			ctx = 0;
	}
	
	if( ctx == 0 )
	{
		// Request a context from the engine
		ctx = engine->RequestContext();
		if( ctx == 0 )
		{
			// TODO: How to best report this failure?
			return;
		}
	}

	r = ctx->Prepare(func);
	if( r < 0 )
	{
		if( isNested )
			ctx->PopState();
		else
			engine->ReturnContext(ctx);
		return;
	}

	ctx->SetObject(ObjectPtr);

	for(;;)
	{
		r = ctx->Execute();

		// We can't allow this execution to be suspended 
		// so resume the execution immediately
		if( r != asEXECUTION_SUSPENDED )
			break;
	}

	if( r != asEXECUTION_FINISHED )
	{
		if( isNested )
		{
			ctx->PopState();

			// If the execution was aborted or an exception occurred,
			// then we should forward that to the outer execution.
			if( r == asEXECUTION_EXCEPTION )
			{
				// TODO: How to improve this exception
				ctx->SetException(TXT_EXCEPTION_IN_NESTED_CALL);
			}
			else if( r == asEXECUTION_ABORTED )
				ctx->Abort();
		}
		else
			engine->ReturnContext(ctx);

		return;
	}

	if( isNested )
		ctx->PopState();
	else
		engine->ReturnContext(ctx);
}

#ifdef AS_MAX_PORTABILITY

static void ScriptObject_AddRef_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	self->AddRef();
}

static void ScriptObject_Release_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	self->Release();
}

static void ScriptObject_GetRefCount_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	*(int*)gen->GetAddressOfReturnLocation() = self->GetRefCount();
}

static void ScriptObject_SetFlag_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	self->SetFlag();
}

static void ScriptObject_GetFlag_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	*(bool*)gen->GetAddressOfReturnLocation() = self->GetFlag();
}

static void ScriptObject_GetWeakRefFlag_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	*(asILockableSharedBool**)gen->GetAddressOfReturnLocation() = self->GetWeakRefFlag();
}

static void ScriptObject_EnumReferences_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	asIScriptEngine *engine = *(asIScriptEngine**)gen->GetAddressOfArg(0);
	self->EnumReferences(engine);
}

static void ScriptObject_ReleaseAllHandles_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();
	asIScriptEngine *engine = *(asIScriptEngine**)gen->GetAddressOfArg(0);
	self->ReleaseAllHandles(engine);
}

static void ScriptObject_Assignment_Generic(asIScriptGeneric *gen)
{
	asCScriptObject *other = *(asCScriptObject**)gen->GetAddressOfArg(0);
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();

	*self = *other;

	*(asCScriptObject**)gen->GetAddressOfReturnLocation() = self;
}

static void ScriptObject_Construct_Generic(asIScriptGeneric *gen)
{
	asCObjectType *objType = *(asCObjectType**)gen->GetAddressOfArg(0);
	asCScriptObject *self = (asCScriptObject*)gen->GetObject();

	ScriptObject_Construct(objType, self);
}

#endif

void Caller_ScriptObject_Construct(asFUNCTION_t FunctionPtr, void** Arguments, void* ReturnValue)
{
	//ScriptObject_Construct(Arguments[0]);
}

void Caller_ScriptObject_Assignment(asFUNCTION_t FunctionPtr, void** Arguments, void* ReturnValue)
{
}

void RegisterScriptObject(asCScriptEngine *engine)
{
	// Register the default script class behaviours
	int r = 0;
	UNUSED_VAR(r); // It is only used in debug mode
	engine->scriptTypeBehaviours.engine = engine;
	engine->scriptTypeBehaviours.flags = asOBJ_SCRIPT_OBJECT | asOBJ_REF | asOBJ_NOCOUNT;
	engine->scriptTypeBehaviours.name = "$obj";
#ifndef AS_MAX_PORTABILITY
	r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_CONSTRUCT, "void f()", asFUNCTION(ScriptObject_Construct), asCALL_CDECL_OBJLAST, Caller_ScriptObject_Construct, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_ADDREF, "void f()", asMETHOD(asCScriptObject,AddRef), asCALL_THISCALL, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_RELEASE, "void f()", asMETHOD(asCScriptObject,Release), asCALL_THISCALL, 0); asASSERT( r >= 0 );
	r = engine->RegisterMethodToObjectType(&engine->scriptTypeBehaviours, "int &opAssign(int &in Other)", asFUNCTION(ScriptObject_Assignment), asCALL_CDECL_OBJLAST, Caller_ScriptObject_Assignment, nullptr); asASSERT( r >= 0 );

	// Weakref behaviours
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_GET_WEAKREF_FLAG, "int &f()", asMETHOD(asCScriptObject,GetWeakRefFlag), asCALL_THISCALL, 0); asASSERT( r >= 0 );
	
	// Register GC behaviours
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_GETREFCOUNT, "int f()", asMETHOD(asCScriptObject,GetRefCount), asCALL_THISCALL, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_SETGCFLAG, "void f()", asMETHOD(asCScriptObject,SetFlag), asCALL_THISCALL, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_GETGCFLAG, "bool f()", asMETHOD(asCScriptObject,GetFlag), asCALL_THISCALL, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_ENUMREFS, "void f(int&in)", asMETHOD(asCScriptObject,EnumReferences), asCALL_THISCALL, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_RELEASEREFS, "void f(int&in)", asMETHOD(asCScriptObject,ReleaseAllHandles), asCALL_THISCALL, 0); asASSERT( r >= 0 );
#else
	r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_CONSTRUCT, "void f(int&in Other)", asFUNCTION(ScriptObject_Construct_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_ADDREF, "void f()", asFUNCTION(ScriptObject_AddRef_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_RELEASE, "void f()", asFUNCTION(ScriptObject_Release_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
	r = engine->RegisterMethodToObjectType(&engine->scriptTypeBehaviours, "int &opAssign(int &in Other)", asFUNCTION(ScriptObject_Assignment_Generic), asCALL_GENERIC); asASSERT( r >= 0 );

	// Weakref behaviours
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_GET_WEAKREF_FLAG, "int &f()", asFUNCTION(ScriptObject_GetWeakRefFlag_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );

	// Register GC behaviours
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_GETREFCOUNT, "int f()", asFUNCTION(ScriptObject_GetRefCount_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_SETGCFLAG, "void f()", asFUNCTION(ScriptObject_SetFlag_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_GETGCFLAG, "bool f()", asFUNCTION(ScriptObject_GetFlag_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_ENUMREFS, "void f(int&in)", asFUNCTION(ScriptObject_EnumReferences_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
	//r = engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, asBEHAVE_RELEASEREFS, "void f(int&in)", asFUNCTION(ScriptObject_ReleaseAllHandles_Generic), asCALL_GENERIC, 0); asASSERT( r >= 0 );
#endif
}

void ScriptObject_Construct(asCObjectType *objType, asCScriptObject *self)
{
	new(self) asCScriptObject(objType);
}

void ScriptObject_ConstructUnitialized(asCObjectType *objType, asCScriptObject *self)
{
	new(self) asCScriptObject(objType, false);
}

asCScriptObject::asCScriptObject(asCObjectType *ot, bool doInitialize)
{
	/*auto* objType = ot;
	objType->AddRef();

	// Notify the garbage collector of this object
	if( objType->flags & asOBJ_GC )
		objType->engine->gc.AddScriptObjectToGC(this, objType);

	// Initialize members to zero. Technically we only need to zero the pointer
	// members, but just the memset is faster than having to loop and check the datatypes
	// Don't need to zero, unreal does it for us
	//memset((void*)((size_t)this+(size_t)objType->basePropertyOffset), 0, objType->size - objType->basePropertyOffset);

	if( doInitialize )
	{
#ifdef AS_NO_MEMBER_INIT
		// When member initialization is disabled the constructor must make sure
		// to allocate and initialize all members with the default constructor
		for( asUINT n = 0; n < objType->properties.GetLength(); n++ )
		{
			asCObjectProperty *prop = objType->properties[n];
			if( prop->type.IsObject() && !prop->type.IsObjectHandle() )
			{
				if( prop->type.IsReference() || prop->type.GetTypeInfo()->flags & asOBJ_REF )
				{
					asPWORD *ptr = reinterpret_cast<asPWORD*>(reinterpret_cast<asBYTE*>(this) + prop->byteOffset);
					if( prop->type.GetTypeInfo()->flags & asOBJ_SCRIPT_OBJECT )
						*ptr = (asPWORD)ScriptObjectFactory(prop->type.GetTypeInfo(), ot->engine);
					else
						*ptr = (asPWORD)AllocateUninitializedObject(prop->type.GetTypeInfo(), ot->engine);
				}
			}
		}
#endif
	}
	else
	{
		// When the object is created without initialization, all non-handle members must be allocated, but not initialized
		asCScriptEngine *engine = objType->engine;
		for( asUINT n = 0; n < objType->properties.GetLength(); n++ )
		{
			asCObjectProperty *prop = objType->properties[n];
			if( prop->type.IsObject() && !prop->type.IsObjectHandle() )
			{
				if( prop->type.IsReference() || (prop->type.GetTypeInfo()->flags & asOBJ_REF) )
				{
					asPWORD *ptr = reinterpret_cast<asPWORD*>(reinterpret_cast<asBYTE*>(this) + prop->byteOffset);
					*ptr = (asPWORD)AllocateUninitializedObject(CastToObjectType(prop->type.GetTypeInfo()), engine);
				}
			}
		}
	}*/
}

/*void asCScriptObject::Destruct()
{
	// Call the destructor, which will also call the GCObject's destructor
	this->~asCScriptObject();

	// Free the memory
#ifndef WIP_16BYTE_ALIGN
	userFree(this);
#else
	// Script object memory is allocated through asCScriptEngine::CallAlloc()
	// This free call must match the allocator used in CallAlloc().
	userFreeAligned(this);
#endif
}*/

/*asILockableSharedBool *asCScriptObject::GetWeakRefFlag() const
{
	return nullptr;
}*/

void *asIScriptObject::GetUserData(asPWORD type) const
{
	return 0;
}

void *asIScriptObject::SetUserData(void *data, asPWORD type)
{
	return 0;
}

asIScriptEngine *asIScriptObject::GetEngine() const
{
	auto* objType = (asCObjectType*)GetObjectType();
	return objType->engine;
}

/*int asCScriptObject::AddRef() const
{
	// Warn in case the application tries to increase the refCount after it has reached zero.
	// This may happen for example if the application calls a method on the class while it is
	// being destroyed. The application shouldn't do this because it may cause application
	// crashes if members that have already been destroyed are accessed accidentally.
	if( hasRefCountReachedZero )
	{
		if( objType && objType->engine )
		{
			asCString msg;
			msg.Format(TXT_RESURRECTING_SCRIPTOBJECT_s, objType->name.AddressOf());
			objType->engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, msg.AddressOf());
		}
	}

	// Increase counter and clear flag set by GC
	gcFlag = false;
	return refCount.atomicInc();
}

int asCScriptObject::Release() const
{
	// Clear the flag set by the GC
	gcFlag = false;

	// Call the script destructor behaviour if the reference counter is 1.
	if( refCount.get() == 1 && !isDestructCalled )
	{
		// This cast is OK since we are the last reference
		const_cast<asCScriptObject*>(this)->CallDestructor();
	}

	// Now do the actual releasing
	int r = refCount.atomicDec();
	if( r == 0 )
	{
		// Flag this object as being destroyed so the application
		// can be warned if the code attempts to resurrect the object
		// during the destructor. This also avoids a recursive call
		// to the destructor which would crash the application if it
		// really does resurrect the object.
		if( !hasRefCountReachedZero )
		{
			hasRefCountReachedZero = true;

			// This cast is OK since we are the last reference
			const_cast<asCScriptObject*>(this)->Destruct();
		}
		return 0;
	}

	return r;
}*/

void asIScriptObject::CallDestructor(class asCObjectType* objType)
{
	asIScriptContext *ctx = 0;
	bool isNested = false;
	bool doAbort = false;

	// Call the destructor for this class and all the super classes
	asCObjectType *ot = objType;
	int funcIndex = ot->beh.destruct;
	if( funcIndex )
	{
		if( ctx == 0 )
		{
			// Check for active context first as it is quicker
			// to reuse than to set up a new one.
			ctx = asGetUsableContext();
			if( ctx )
			{
				if( ctx->GetEngine() == objType->GetEngine() && ctx->PushState() == asSUCCESS )
					isNested = true;
				else
					ctx = 0;
			}

			if( ctx == 0 )
			{
				// Request a context from the engine
				ctx = objType->engine->RequestContext();
				if( ctx == 0 )
				{
					// TODO: How to best report this failure?
					return;
				}
			}
		}

		int r = ctx->Prepare(objType->engine->scriptFunctions[funcIndex]);
		if( r >= 0 )
		{
			ctx->SetObject(this);

			for(;;)
			{
				r = ctx->Execute();

				// If the script tries to suspend itself just restart it
				if( r != asEXECUTION_SUSPENDED )
					break;
			}

			// Exceptions in the destructor will be ignored, as there is not much
			// that can be done about them. However a request to abort the execution
			// will be forwarded to the outer execution, in case of a nested call.
			if( r == asEXECUTION_ABORTED )
				doAbort = true;

			// Observe, even though the current destructor was aborted or an exception
			// occurred, we still try to execute the base class' destructor if available
			// in order to free as many resources as possible.
		}
	}

	if( ctx )
	{
		if( isNested )
		{
			ctx->PopState();

			// Forward any request to abort the execution to the outer call
			if( doAbort )
				ctx->Abort();
		}
		else
		{
			// Return the context to engine
			objType->engine->ReturnContext(ctx);
		}
	}

	//objType->Release();
}

/*int asCScriptObject::GetRefCount()
{
	return refCount.get();
}

void asCScriptObject::SetFlag()
{
	gcFlag = true;
}

bool asCScriptObject::GetFlag()
{
	return gcFlag;
}*/

// interface
int asIScriptObject::GetTypeId() const
{
	auto* objType = (asCObjectType*)GetObjectType();
	asCDataType dt = asCDataType::CreateType(objType, false);
	return objType->engine->GetTypeIdFromDataType(dt);
}

asUINT asIScriptObject::GetPropertyCount() const
{
	auto* objType = (asCObjectType*)GetObjectType();
	return asUINT(objType->properties.GetLength());
}

int asIScriptObject::GetPropertyTypeId(asUINT prop) const
{
	auto* objType = (asCObjectType*)GetObjectType();
	if( prop >= objType->properties.GetLength() )
		return asINVALID_ARG;

	return objType->engine->GetTypeIdFromDataType(objType->properties[prop]->type);
}

const char *asIScriptObject::GetPropertyName(asUINT prop) const
{
	auto* objType = (asCObjectType*)GetObjectType();
	if( prop >= objType->properties.GetLength() )
		return 0;

	return objType->properties[prop]->name.AddressOf();
}

void *asIScriptObject::GetAddressOfProperty(asUINT prop)
{
	auto* objType = (asCObjectType*)GetObjectType();
	if( prop >= objType->properties.GetLength() )
		return 0;

	// Objects are stored by reference, so this must be dereferenced
	asCDataType *dt = &objType->properties[prop]->type;
	if( dt->IsObject() && !dt->IsObjectHandle() &&
		(dt->IsReference() || dt->GetTypeInfo()->flags & asOBJ_REF) )
		return *(void**)(((char*)this) + objType->properties[prop]->byteOffset);

	return (void*)(((char*)this) + objType->properties[prop]->byteOffset);
}

/*void asCScriptObject::EnumReferences(asIScriptEngine *engine)
{
	// We'll notify the GC of all object handles that we're holding
	for( asUINT n = 0; n < objType->properties.GetLength(); n++ )
	{
		asCObjectProperty *prop = objType->properties[n];
		void *ptr = 0;
		if (prop->type.IsObject())
		{
			if (prop->type.IsReference() || (prop->type.GetTypeInfo()->flags & asOBJ_REF))
				ptr = *(void**)(((char*)this) + prop->byteOffset);
			else
				ptr = (void*)(((char*)this) + prop->byteOffset);

			// The members of the value type needs to be enumerated
			// too, since the value type may be holding a reference.
			if ((prop->type.GetTypeInfo()->flags & asOBJ_VALUE) && (prop->type.GetTypeInfo()->flags & asOBJ_GC))
			{
				reinterpret_cast<asCScriptEngine*>(engine)->CallObjectMethod(ptr, engine, CastToObjectType(prop->type.GetTypeInfo())->beh.gcEnumReferences);
			}
		}
		else if (prop->type.IsFuncdef())
			ptr = *(void**)(((char*)this) + prop->byteOffset);

		if (ptr)
			((asCScriptEngine*)engine)->GCEnumCallback(ptr);
	}
}

void asCScriptObject::ReleaseAllHandles(asIScriptEngine *engine)
{
	for( asUINT n = 0; n < objType->properties.GetLength(); n++ )
	{
		asCObjectProperty *prop = objType->properties[n];

		if (prop->type.IsObject())
		{
			if (prop->type.IsObjectHandle())
			{
				void **ptr = (void**)(((char*)this) + prop->byteOffset);
				if (*ptr)
				{
					asASSERT((prop->type.GetTypeInfo()->flags & asOBJ_NOCOUNT) || prop->type.GetBehaviour()->release);
					if (prop->type.GetBehaviour()->release)
						((asCScriptEngine*)engine)->CallObjectMethod(*ptr, prop->type.GetBehaviour()->release);
					*ptr = 0;
				}
			}
			else if ((prop->type.GetTypeInfo()->flags & asOBJ_VALUE) && (prop->type.GetTypeInfo()->flags & asOBJ_GC))
			{
				// The members of the members needs to be released
				// too, since they may be holding a reference. Even
				// if the member is a value type.
				void *ptr = 0;
				if (prop->type.IsReference())
					ptr = *(void**)(((char*)this) + prop->byteOffset);
				else
					ptr = (void*)(((char*)this) + prop->byteOffset);

				reinterpret_cast<asCScriptEngine*>(engine)->CallObjectMethod(ptr, engine, CastToObjectType(prop->type.GetTypeInfo())->beh.gcReleaseAllReferences);
			}
		}
		else if (prop->type.IsFuncdef())
		{
			asCScriptFunction **ptr = (asCScriptFunction**)(((char*)this) + prop->byteOffset);
			if (*ptr)
			{
				(*ptr)->Release();
				*ptr = 0;
			}
		}
	}
}*/

asCScriptObject &ScriptObject_Assignment(asCScriptObject *other, asCScriptObject *self)
{
	return (*self = *other);
}

asIScriptObject &asIScriptObject::operator=(const asIScriptObject &other)
{
	auto* objType = (asCObjectType*)GetObjectType();
	auto* otherObjType = (asCObjectType*)other.GetObjectType();
	return ((asCScriptObject*)this)->PerformCopy((asCScriptObject*)&other, objType, otherObjType);
}

void asCScriptObject::CopyStruct(asCScriptObject* SourceObject, asCObjectType* ObjectType)
{
#if DO_CHECK
	asCScriptEngine* Engine = ObjectType->engine;
	check(ObjectType->beh.copy == Engine->scriptTypeBehaviours.beh.copy);
	check((ObjectType->flags & asOBJ_POD) == 0);
#endif

	// Copy all properties
	for( asUINT n = 0; n < ObjectType->properties.GetLength(); n++ )
	{
		asCObjectProperty *prop = ObjectType->properties[n];

		void **dst = (void**)(((char*)this) + prop->byteOffset);
		void **src = (void**)(((char*)SourceObject) + prop->byteOffset);

		if (auto* TypeInfo = prop->type.GetTypeInfo())
		{
			if (TypeInfo->flags & (asOBJ_ENUM | asOBJ_POD))
			{
				FMemory::Memcpy(dst, src, TypeInfo->size);
			}
			else if (TypeInfo->flags & asOBJ_REF)
			{
				*dst = *src;
			}
			else
			{
				CopyObject(src, dst, (asCObjectType*)TypeInfo, ObjectType->engine);
			}
		}
		else
		{
			FMemory::Memcpy(dst, src, prop->type.GetSizeInMemoryBytes());
		}
	}
}

asCScriptObject& asCScriptObject::PerformCopy(asCScriptObject* OtherObject, asCObjectType* objType, asCObjectType* otherObjType)
{
	asCScriptObject& other = *OtherObject;
	if( &other != this )
	{
		if( !otherObjType->DerivesFrom(objType) )
		{
			// We cannot allow a value assignment from a type that isn't the same or 
			// derives from this type as the member properties may not have the same layout
			asIScriptContext *ctx = asGetActiveContext();
			ctx->SetException(TXT_MISMATCH_IN_VALUE_ASSIGN);
			return *this;
		}

		// If the script class implements the opAssign method, it should be called
		asCScriptEngine *engine = objType->engine;
		asCScriptFunction *func = engine->scriptFunctions[objType->beh.copy];
		if( func->funcType == asFUNC_SYSTEM )
		{
			// Copy all properties
			for( asUINT n = 0; n < objType->properties.GetLength(); n++ )
			{
				asCObjectProperty *prop = objType->properties[n];
				if (prop->byteOffset < objType->basePropertyOffset)
					continue;
				if( prop->type.IsObject() )
				{
					void **dst = (void**)(((char*)this) + prop->byteOffset);
					void **src = (void**)(((char*)&other) + prop->byteOffset);
					if( !prop->type.IsObjectHandle() )
					{
						if( prop->type.IsReference() || (prop->type.GetTypeInfo()->flags & asOBJ_REF) )
							CopyObject(*src, *dst, CastToObjectType(prop->type.GetTypeInfo()), engine);
						else
							CopyObject(src, dst, CastToObjectType(prop->type.GetTypeInfo()), engine);
					}
					else
						CopyHandle((asPWORD*)src, (asPWORD*)dst, CastToObjectType(prop->type.GetTypeInfo()), engine);
				}
				else
				{
					void *dst = ((char*)this) + prop->byteOffset;
					void *src = ((char*)&other) + prop->byteOffset;
					memcpy(dst, src, prop->type.GetSizeInMemoryBytes());
				}
			}
		}
		else
		{
			// Reuse the active context or create a new one to call the script class' opAssign method
			asIScriptContext *ctx = 0;
			int r = 0;
			bool isNested = false;

			ctx = asGetUsableContext();
			if( ctx )
			{
				if( ctx->GetEngine() == engine && ctx->PushState() == asSUCCESS )
					isNested = true;
				else
					ctx = 0;
			}

			if( ctx == 0 )
			{
				// Request a context from the engine
				ctx = engine->RequestContext();
				if( ctx == 0 )
				{
					// TODO: How to best report this failure?
					return *this;
				}
			}

			r = ctx->Prepare(engine->scriptFunctions[objType->beh.copy]);
			if( r < 0 )
			{
				if( isNested )
					ctx->PopState();
				else
					engine->ReturnContext(ctx);
				// TODO: How to best report this failure?
				return *this;
			}

			r = ctx->SetArgAddress(0, (asCScriptObject*)(&other));
			asASSERT( r >= 0 );
			r = ctx->SetObject(this);
			asASSERT( r >= 0 );

			for(;;)
			{
				r = ctx->Execute();

				// We can't allow this execution to be suspended 
				// so resume the execution immediately
				if( r != asEXECUTION_SUSPENDED )
					break;
			}

			if( r != asEXECUTION_FINISHED )
			{
				if( isNested )
				{
					ctx->PopState();

					// If the execution was aborted or an exception occurred,
					// then we should forward that to the outer execution.
					if( r == asEXECUTION_EXCEPTION )
					{
						// TODO: How to improve this exception
						ctx->SetException(TXT_EXCEPTION_IN_NESTED_CALL);
					}
					else if( r == asEXECUTION_ABORTED )
						ctx->Abort();
				}
				else
				{
					// Return the context to the engine
					engine->ReturnContext(ctx);
				}
				return *this;
			}

			if( isNested )
				ctx->PopState();
			else
			{
				// Return the context to the engine
				engine->ReturnContext(ctx);
			}
		}
	}

	return *this;
}

int asIScriptObject::CopyFrom(asIScriptObject *other)
{
	if( other == 0 ) return asINVALID_ARG;

	if( GetTypeId() != other->GetTypeId() )
		return asINVALID_TYPE;

	*this = *(asCScriptObject*)other;

	return 0;
}

void *asIScriptObject::AllocateUninitializedObject(asCObjectType *in_objType, asCScriptEngine *engine)
{
	void *ptr = 0;

	if( (in_objType->flags & asOBJ_SCRIPT_OBJECT) && (in_objType->flags & asOBJ_REF) )
	{
		ptr = nullptr;
		//ptr = engine->AllocScriptObject(in_objType);
		//ScriptObject_ConstructUnitialized(in_objType, reinterpret_cast<asCScriptObject*>(ptr));
	}
	else if( in_objType->flags & asOBJ_TEMPLATE )
	{
		// Templates store the original factory that takes the object
		// type as a hidden parameter in the construct behaviour
		ptr = engine->CallGlobalFunctionRetPtr(in_objType->beh.construct, in_objType);
	}
	else if( in_objType->flags & asOBJ_REF )
	{
		ptr = engine->CallGlobalFunctionRetPtr(in_objType->beh.factory);
	}
	else
	{
		ptr = engine->CallAlloc(in_objType);
		int funcIndex = in_objType->beh.construct;
		if( funcIndex )
			engine->CallObjectMethod(ptr, funcIndex);
	}

	return ptr;
}

/*void asCScriptObject::FreeObject(void *ptr, asCObjectType *in_objType, asCScriptEngine *engine)
{
	if( in_objType->flags & asOBJ_REF )
	{
		asASSERT( (in_objType->flags & asOBJ_NOCOUNT) || in_objType->beh.release );
		if(in_objType->beh.release )
			engine->CallObjectMethod(ptr, in_objType->beh.release);
	}
	else
	{
		if( in_objType->beh.destruct )
			engine->CallObjectMethod(ptr, in_objType->beh.destruct);

		engine->CallFree(ptr);
	}
}*/

void asIScriptObject::CopyObject(void *src, void *dst, asCObjectType *in_objType, asCScriptEngine *engine)
{
	int funcIndex = in_objType->beh.copy;
	if( funcIndex )
	{
		asCScriptFunction *func = engine->scriptFunctions[in_objType->beh.copy];
		if(in_objType->flags & asOBJ_SCRIPT_OBJECT)
			reinterpret_cast<asCScriptObject*>(dst)->PerformCopy(reinterpret_cast<asCScriptObject*>(src), in_objType, in_objType);
		else //if( func->funcType == asFUNC_SYSTEM )
			engine->CallObjectMethod(dst, src, funcIndex);
	}
	else if( in_objType->size && (in_objType->flags & asOBJ_POD) )
	{
		memcpy(dst, src, in_objType->size);
	}
	else
	{
		ensureMsgf(false, TEXT("Attempted to copy script object type %s which does not have an opAssign"), ANSI_TO_TCHAR(in_objType->name.AddressOf()));
	}
}

void asIScriptObject::CopyHandle(asPWORD *src, asPWORD *dst, asCObjectType *in_objType, asCScriptEngine *engine)
{
	*dst = *src;
}

// TODO: weak: Should move to its own file
asCLockableSharedBool::asCLockableSharedBool() : value(false) 
{
	refCount.set(1);
}

int asCLockableSharedBool::AddRef() const
{
	return refCount.atomicInc();
}

int asCLockableSharedBool::Release() const
{
	int r = refCount.atomicDec();
	if( r == 0 )
		asDELETE(const_cast<asCLockableSharedBool*>(this), asCLockableSharedBool);
	return r;
}

bool asCLockableSharedBool::Get() const
{
	return value;
}

void asCLockableSharedBool::Set(bool v)
{
	// Make sure the value is not changed while another thread  
	// is inspecting it and taking a decision on what to do.
	Lock();
	value = v;
	Unlock();
}

void asCLockableSharedBool::Lock() const
{
	ENTERCRITICALSECTION(lock);
}

void asCLockableSharedBool::Unlock() const
{
	LEAVECRITICALSECTION(lock);
}

// Interface
// Auxiliary function to allow applications to create shared 
// booleans without having to implement the logic for them
AS_API asILockableSharedBool *asCreateLockableSharedBool()
{
	return asNEW(asCLockableSharedBool);
}

END_AS_NAMESPACE

