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



//
// as_module.cpp
//
// A class that holds a script module
//

#include "as_module.h"
#include "as_config.h"
#include "as_builder.h"
#include "as_context.h"
#include "as_texts.h"
#include "as_debug.h"
//WILL-EDIT
#include "as_scriptobject.h"

BEGIN_AS_NAMESPACE


// internal
asCModule::asCModule(const char *name, asCScriptEngine *engine)
{
	this->name     = name;
	this->engine   = engine;

	userData = 0;
	builder = 0;
	isGlobalVarInitialized = false;

	accessMask = 1;

	defaultNamespace = engine->nameSpaces[0];
}

// internal
asCModule::~asCModule()
{
	InternalReset();
	ClearImports();

	// The builder is not removed by InternalReset because it holds the script
	// sections that will be built, so we need to explictly remove it now if it exists
	if( builder )
	{
		asDELETE(builder,asCBuilder);
		builder = 0;
	}

	if( engine )
	{
		// Clean the user data
		for( asUINT n = 0; n < userData.GetLength(); n += 2 )
		{
			if( userData[n+1] )
			{
				for( asUINT c = 0; c < engine->cleanModuleFuncs.GetLength(); c++ )
					if( engine->cleanModuleFuncs[c].type == userData[n] )
						engine->cleanModuleFuncs[c].cleanFunc(this);
			}
		}
	}
}

// interface
void asCModule::Discard()
{
	// Reset the global variables already so that no object in the global variables keep the module alive forever.
	// If any live object tries to access the global variables during clean up they will fail with a script exception,
	// so the application must keep that in mind before discarding a module.
	CallExit();

	// Keep a local copy of the engine pointer, because once the module is moved do the discarded
	// pile, it is possible that another thread might discard it while we are still in here. So no
	// further access to members may be done after that
	asCScriptEngine *lEngine = engine;
	discarded = true;

	// Instead of deleting the module immediately, move it to the discarded pile
	// This will turn it invisible to the application, yet keep it alive until all
	// external references to its entities have been released.
	lEngine->scriptModulesByName.Remove(this);
}

// interface
void *asCModule::SetUserData(void *data, asPWORD type)
{
	// As a thread might add a new new user data at the same time as another
	// it is necessary to protect both read and write access to the userData member
	// It is not intended to store a lot of different types of userdata,
	// so a more complex structure like a associative map would just have
	// more overhead than a simple array.
	for( asUINT n = 0; n < userData.GetLength(); n += 2 )
	{
		if( userData[n] == type )
		{
			void *oldData = reinterpret_cast<void*>(userData[n+1]);
			userData[n+1] = reinterpret_cast<asPWORD>(data);

			RELEASEEXCLUSIVE(engine->engineRWLock);

			return oldData;
		}
	}

	userData.PushLast(type);
	userData.PushLast(reinterpret_cast<asPWORD>(data));

	return 0;
}

// interface
void *asCModule::GetUserData(asPWORD type) const
{
	// There may be multiple threads reading, but when
	// setting the user data nobody must be reading.
	for( asUINT n = 0; n < userData.GetLength(); n += 2 )
	{
		if( userData[n] == type )
		{
			void *ud = reinterpret_cast<void*>(userData[n+1]);
			return ud;
		}
	}

	return 0;
}

// interface
asIScriptEngine *asCModule::GetEngine() const
{
	return engine;
}

// interface
void asCModule::SetName(const char *in_name)
{
	engine->scriptModulesByName.Remove(this);
	name = in_name;
	engine->scriptModulesByName.Add(this);
}

// interface
const char *asCModule::GetName() const
{
	return name.AddressOf();
}

// interface
const char *asCModule::GetDefaultNamespace() const
{
	return defaultNamespace->name.AddressOf();
}

// interface
int asCModule::SetDefaultNamespace(const char *nameSpace)
{
	// TODO: cleanup: This function is similar to asCScriptEngine::SetDefaultNamespace. Can we reuse the code?
	if( nameSpace == 0 )
		return asINVALID_ARG;

	asCString ns = nameSpace;
	if( ns != "" )
	{
		// Make sure the namespace is composed of alternating identifier and ::
		size_t pos = 0;
		bool expectIdentifier = true;
		size_t len;
		eTokenType t = ttIdentifier;

		for( ; pos < ns.GetLength(); pos += len )
		{
			t = engine->tok.GetToken(ns.AddressOf() + pos, ns.GetLength() - pos, &len);
			if( (expectIdentifier && t != ttIdentifier) || (!expectIdentifier && t != ttScope) )
				return asINVALID_DECLARATION;

			expectIdentifier = !expectIdentifier;
		}

		// If the namespace ends with :: then strip it off
		if( t == ttScope )
			ns.SetLength(ns.GetLength()-2);
	}

	defaultNamespace = engine->AddNameSpace(ns.AddressOf());

	return 0;
}

// interface
int asCModule::AddScriptSection(const char *in_name, const char *in_code, size_t in_codeLength, int in_lineOffset)
{
#ifdef AS_NO_COMPILER
	UNUSED_VAR(in_name);
	UNUSED_VAR(in_code);
	UNUSED_VAR(in_codeLength);
	UNUSED_VAR(in_lineOffset);
	return asNOT_SUPPORTED;
#else
	if( !builder )
	{
		builder = asNEW(asCBuilder)(engine, this);
		if( builder == 0 )
			return asOUT_OF_MEMORY;
	}

	return builder->AddCode(in_name, in_code, (int)in_codeLength, in_lineOffset, (int)engine->GetScriptSectionNameIndex(in_name ? in_name : ""), engine->ep.copyScriptSections);
#endif
}

// internal
void asCModule::JITCompile()
{
	asIJITCompiler *jit = engine->GetJITCompiler();
	if( !jit )
		return;

	for (unsigned int i = 0; i < scriptFunctions.GetLength(); i++)
		scriptFunctions[i]->JITCompile();
}

// interface
int asCModule::Build()
{
#ifdef AS_NO_COMPILER
	return asNOT_SUPPORTED;
#else
	TimeIt("asCModule::Build");

	// Don't allow the module to be rebuilt if there are still
	// external references that will need the previous code
	// TODO: interface: The asIScriptModule must have a method for querying if the module is used
	if( HasExternalReferences(false) )
	{
		engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, TXT_MODULE_IS_IN_USE);
		return asMODULE_IS_IN_USE;
	}

	// Only one thread may build at one time
	// TODO: It should be possible to have multiple threads perform compilations
	int r = engine->RequestBuild();
	if( r < 0 )
		return r;

	engine->PrepareEngine();
	if( engine->configFailed )
	{
		engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, TXT_INVALID_CONFIGURATION);
		engine->BuildCompleted();
		return asINVALID_CONFIGURATION;
	}

	InternalReset();

	if( !builder )
	{
		engine->BuildCompleted();
		return asSUCCESS;
	}

	// Compile the script
	//r = builder->Build();
	asDELETE(builder,asCBuilder);
	builder = 0;

	if( r < 0 )
	{
		// Reset module again
		InternalReset();

		engine->BuildCompleted();
		return r;
	}

	JITCompile();

	engine->PrepareEngine();

#ifdef AS_DEBUG
	// Verify that there are no unwanted gaps in the scriptFunctions array.
	for( asUINT n = 1; n < engine->scriptFunctions.GetLength(); n++ )
	{
		int id = n;
		if( engine->scriptFunctions[n] == 0 && !engine->freeScriptFunctionIds.Exists(id) )
			asASSERT( false );
	}
#endif

	engine->BuildCompleted();

	// Initialize global variables
	if( r >= 0 && engine->ep.initGlobalVarsAfterBuild )
		r = ResetGlobalVars(0);

	return r;
#endif
}

// interface
int asCModule::ResetGlobalVars(asIScriptContext *ctx)
{
	if( isGlobalVarInitialized )
		CallExit();

	return CallInit(ctx);
}

// interface
asIScriptFunction *asCModule::GetFunctionByIndex(asUINT index) const
{
	if (index >= globalFunctionList.GetLength())
		return nullptr;
	return globalFunctionList[index];
}

asCGlobalProperty* asCModule::InitializingGlobalProperty = nullptr;

// internal
int asCModule::CallInit(asIScriptContext *myCtx)
{
	if( isGlobalVarInitialized )
		return asERROR;

	// Each global variable needs to be cleared individually
	for (asUINT n = 0; n < scriptGlobalsList.GetLength(); ++n)
	{
		asCGlobalProperty *desc = scriptGlobalsList[n];

		// Don't need to initialize pure constant globals, they already got the right value
		// when they were compiled.
		if (desc->isPureConstant)
			continue;

		memset(desc->GetAddressOfValue(), 0, sizeof(asDWORD)*desc->type.GetSizeOnStackDWords());
	}

	// Call the init function for each of the global variables
	asIScriptContext *ctx = myCtx;
	int r = asEXECUTION_FINISHED;
	for (asUINT n = 0; n < scriptGlobalsList.GetLength(); ++n)
	{
		asCGlobalProperty *desc = scriptGlobalsList[n];

		// Don't need to initialize pure constant globals, they already got the right value
		// when they were compiled.
		if (desc->isPureConstant)
			continue;

		// If the variable is default initialized, just call the default constructor
		if (desc->isDefaultInit)
		{
			if (desc->type.IsObject())
			{
				TGuardValue ScopeGlobalProperty(InitializingGlobalProperty, desc);

				void** Address = (void**)desc->GetAddressOfValue();
				*Address = engine->CallAlloc((asCObjectType*)desc->type.GetTypeInfo());
				ScriptObjectDefaultConstructor((asCObjectType*)desc->type.GetTypeInfo(), engine, *Address);
			}

			continue;
		}

		if( desc->GetInitFunc() )
		{
			TGuardValue ScopeGlobalProperty(InitializingGlobalProperty, desc);

			if( ctx == 0 )
			{
				ctx = engine->RequestContext();
				if( ctx == 0 )
					break;
			}

			r = ctx->Prepare(desc->GetInitFunc());
			if( r >= 0 )
			{
				r = ctx->Execute();
				if( r != asEXECUTION_FINISHED )
				{
					asCString msg;
					msg.Format(TXT_FAILED_TO_INITIALIZE_s, desc->name.AddressOf());
					asCScriptFunction *func = desc->GetInitFunc();

					engine->WriteMessage(func->scriptData->scriptSectionIdx >= 0 ? engine->scriptSectionNames[func->scriptData->scriptSectionIdx]->AddressOf() : "",
										 func->GetLineNumber(0, 0) & 0xFFFFF,
										 func->GetLineNumber(0, 0) >> 20,
										 asMSGTYPE_ERROR,
										 msg.AddressOf());

					if( r == asEXECUTION_EXCEPTION )
					{
						const asIScriptFunction *function = ctx->GetExceptionFunction();

						msg.Format(TXT_EXCEPTION_s_IN_s, ctx->GetExceptionString(), function->GetDeclaration());

						engine->WriteMessage(function->GetScriptSectionName(),
											 ctx->GetExceptionLineNumber(),
											 0,
											 asMSGTYPE_INFORMATION,
											 msg.AddressOf());
					}
				}
			}
		}
	}

	if( ctx && !myCtx )
	{
		engine->ReturnContext(ctx);
		ctx = 0;
	}

	// Even if the initialization failed we need to set the
	// flag that the variables have been initialized, otherwise
	// the module won't free those variables that really were
	// initialized.
	isGlobalVarInitialized = true;

	if( r != asEXECUTION_FINISHED )
		return asINIT_GLOBAL_VARS_FAILED;

	return asSUCCESS;
}

// internal
void asCModule::UninitializeGlobalProp(asCGlobalProperty *prop)
{
	if (prop == 0) 
		return;

	if (prop->type.IsObject())
	{
		void **obj = (void**)prop->GetAddressOfValue();
		if (*obj)
		{
			asCObjectType *ot = CastToObjectType(prop->type.GetTypeInfo());

			if (ot->flags & asOBJ_REF)
			{
				asASSERT((ot->flags & asOBJ_NOCOUNT) || ot->beh.release);
				if (ot->beh.release)
					engine->CallObjectMethod(*obj, ot->beh.release);
			}
			else if (ot->flags & asOBJ_SCRIPT_OBJECT)
			{
				(*(asIScriptObject**)obj)->CallDestructor(ot);
				engine->CallFree(*obj);
			}
			else
			{
				if (ot->beh.destruct)
					engine->CallObjectMethod(*obj, ot->beh.destruct);

				engine->CallFree(*obj);
			}

			// Set the address to 0 as someone might try to access the variable afterwards
			*obj = 0;
		}
	}
	else if (prop->type.IsFuncdef())
	{
		asCScriptFunction **func = (asCScriptFunction**)prop->GetAddressOfValue();
		if (*func)
		{
			(*func)->Release();
			*func = 0;
		}
	}
}

// internal
void asCModule::CallExit()
{
	if( !isGlobalVarInitialized ) return;

	for (asUINT n = 0; n < scriptGlobalsList.GetLength(); ++n)
	{
		asCGlobalProperty *desc = scriptGlobalsList[n];
		UninitializeGlobalProp(desc);
	}

	isGlobalVarInitialized = false;
}

// internal
bool asCModule::HasExternalReferences(bool shuttingDown)
{
	// Check all entities in the module for any external references.
	// If there are any external references the module cannot be deleted yet.

	for (asUINT n = 0; n < scriptGlobalsList.GetLength(); ++n)
	{
		asCGlobalProperty *desc = scriptGlobalsList[n];

		if (desc->GetInitFunc() && desc->GetInitFunc()->externalRefCount.get())
		{
			if( !shuttingDown )
				return true;
			else
			{
				asCString msg;
				msg.Format(TXT_EXTRNL_REF_TO_MODULE_s, name.AddressOf());
				engine->WriteMessage("", 0, 0, asMSGTYPE_WARNING, msg.AddressOf());

				// TODO: Use a better error message
				asCString tmpName = "init " + desc->name;
				msg.Format(TXT_PREV_FUNC_IS_NAMED_s_TYPE_IS_d, tmpName.AddressOf(), desc->GetInitFunc()->GetFuncType());
				engine->WriteMessage("", 0, 0, asMSGTYPE_INFORMATION, msg.AddressOf());
			}
		}
	}

	for (asUINT n = 0; n < scriptFunctions.GetLength(); n++)
	{
		asCScriptFunction *func = scriptFunctions[n];
		if (func && func->externalRefCount.get())
		{
			// If the func is shared and can be moved to another module then this is not a reason to keep the module alive
			if (func->IsShared() && engine->FindNewOwnerForSharedFunc(func, this) != this)
				continue;

			if (!shuttingDown)
				return true;
			else
			{
				asCString msg;
				msg.Format(TXT_EXTRNL_REF_TO_MODULE_s, name.AddressOf());
				engine->WriteMessage("", 0, 0, asMSGTYPE_WARNING, msg.AddressOf());

				msg.Format(TXT_PREV_FUNC_IS_NAMED_s_TYPE_IS_d, scriptFunctions[n]->GetName(), scriptFunctions[n]->GetFuncType());
				engine->WriteMessage("", 0, 0, asMSGTYPE_INFORMATION, msg.AddressOf());
			}
		}
	}

	for (asUINT n = 0; n < classTypes.GetLength(); n++)
	{
		asCObjectType *obj = classTypes[n];
		if (obj && obj->externalRefCount.get())
		{
			// If the obj is shared and can be moved to another module then this is not a reason to keep the module alive
			if (obj->IsShared() && engine->FindNewOwnerForSharedType(obj, this) != this)
				continue;

			if (!shuttingDown)
				return true;
			else
			{
				asCString msg;
				msg.Format(TXT_EXTRNL_REF_TO_MODULE_s, name.AddressOf());
				engine->WriteMessage("", 0, 0, asMSGTYPE_WARNING, msg.AddressOf());

				msg.Format(TXT_PREV_TYPE_IS_NAMED_s, classTypes[n]->GetName());
				engine->WriteMessage("", 0, 0, asMSGTYPE_INFORMATION, msg.AddressOf());
			}
		}
	}

	for (asUINT n = 0; n < funcDefs.GetLength(); n++)
	{
		asCFuncdefType *func = funcDefs[n];
		if (func && func->externalRefCount.get())
		{
			// If the funcdef is shared and can be moved to another module then this is not a reason to keep the module alive
			if (func->IsShared() && engine->FindNewOwnerForSharedType(func, this) != this)
				continue;

			if (!shuttingDown)
				return true;
			else
			{
				asCString msg;
				msg.Format(TXT_EXTRNL_REF_TO_MODULE_s, name.AddressOf());
				engine->WriteMessage("", 0, 0, asMSGTYPE_WARNING, msg.AddressOf());

				msg.Format(TXT_PREV_FUNC_IS_NAMED_s_TYPE_IS_d, funcDefs[n]->GetName(), funcDefs[n]->funcdef->GetFuncType());
				engine->WriteMessage("", 0, 0, asMSGTYPE_INFORMATION, msg.AddressOf());
			}
		}
	}

	return false;
}

// internal
void asCModule::InternalReset()
{
	CallExit();
	RemoveTypesAndGlobalsFromEngineAvailability();

	asUINT n;

	// Remove all global functions
	globalFunctions.EraseAll();
	globalFunctionList.SetLength(0);

	// Destroy the internals of the global properties here, but do not yet remove them from the
	// engine, because functions need the engine's varAddressMap to get to the property. If the
	// property is removed already, it may leak as the refCount doesn't reach 0.
	for( n = 0; n < scriptGlobalsList.GetLength(); n++ )
	{
		scriptGlobalsList[n]->DestroyInternal();
	}

	UnbindAllImportedFunctions();

	// Free bind information
	for( n = 0; n < bindInformations.GetLength(); n++ )
	{
		if( bindInformations[n] )
		{
			bindInformations[n]->importedFunctionSignature->ReleaseInternal();

			asDELETE(bindInformations[n], sBindInfo);
		}
	}
	bindInformations.SetLength(0);

	// Free declared types, including classes, typedefs, and enums
	TArray<asCObjectType*> oldTemplateInstances = MoveTemp(templateInstances);
	templateInstances.Empty();

	for(asCObjectType* TemplateInstance : oldTemplateInstances)
	{
		engine->DiscardTemplateInstance(TemplateInstance);
		TemplateInstance->ReleaseInternal();
	}

	for( n = 0; n < classTypes.GetLength(); n++ )
	{
		asCObjectType *type = classTypes[n];
		if( type->IsShared() )
		{
			// The type is shared, so transfer ownership to another module that also uses it
			if( engine->FindNewOwnerForSharedType(type, this) != this )
			{
				// The type is owned by another module, just release our reference
				type->ReleaseInternal();
				continue;
			}
		}

		// The type should be destroyed now
		type->DestroyInternal();

		// Remove the type from the engine
		if( type->IsShared() )
		{
			engine->sharedScriptTypes.RemoveValue(type);
			type->ReleaseInternal();
		}

		// Release it from the module
		type->module = 0;
		type->ReleaseInternal();
	}
	classTypes.SetLength(0);
	for( n = 0; n < enumTypes.GetLength(); n++ )
	{
		asCEnumType *type = enumTypes[n];
		if( type->IsShared() )
		{
			// The type is shared, so transfer ownership to another module that also uses it
			if( engine->FindNewOwnerForSharedType(type, this) != this )
			{
				// The type is owned by another module, just release our reference
				type->ReleaseInternal();
				continue;
			}
		}

		// Remove the type from the engine
		if( type->IsShared() )
		{
			engine->sharedScriptTypes.RemoveValue(type);
			type->ReleaseInternal();
		}

		// Release it from the module
		type->module = 0;
		type->ReleaseInternal();
	}
	enumTypes.SetLength(0);
	for( n = 0; n < typeDefs.GetLength(); n++ )
	{
		asCTypedefType *type = typeDefs[n];

		// The type should be destroyed now
		type->DestroyInternal();

		// Release it from the module
		type->module = 0;
		type->ReleaseInternal();
	}
	typeDefs.SetLength(0);

	// Free funcdefs
	for( n = 0; n < funcDefs.GetLength(); n++ )
	{
		asCFuncdefType *func = funcDefs[n];
		asASSERT(func);
		if( func->funcdef && func->funcdef->IsShared() )
		{
			// The funcdef is shared, so transfer ownership to another module that also uses it
			if( engine->FindNewOwnerForSharedType(func, this) != this )
			{
				// The funcdef is owned by another module, just release our reference
				func->ReleaseInternal();
				continue;
			}
		}

		func->DestroyInternal();
		engine->RemoveFuncdef(func);
		func->module = 0;
		func->ReleaseInternal();
	}
	funcDefs.SetLength(0);

	// Then release the functions
	for( n = 0; n < scriptFunctions.GetLength(); n++ )
	{
		asCScriptFunction *func = scriptFunctions[n];
		if( func->IsShared() )
		{
			// The func is shared, so transfer ownership to another module that also uses it
			if( engine->FindNewOwnerForSharedFunc(func, this) != this )
			{
				// The func is owned by another module, just release our reference
				func->ReleaseInternal();
				continue;
			}
		}

		func->DestroyInternal();
		func->module = 0;
		func->ReleaseInternal();
	}
	scriptFunctions.SetLength(0);

	// Now remove and release the global properties as there are no more references to them
	for( n = 0; n < scriptGlobalsList.GetLength(); n++ )
	{
		if (scriptGlobalsList[n]->refCount.get() == 1)
			engine->RemoveGlobalProperty(scriptGlobalsList[n]->id);
		scriptGlobalsList[n]->Release();
	}
	scriptGlobals.EraseAll();
	scriptGlobalsList.SetLength(0);

	asASSERT( IsEmpty() );
}

// interface
asIScriptFunction *asCModule::GetFunctionByName(const char *in_name) const
{
	asSNameSpace *ns = defaultNamespace;
	while( ns )
	{
		{
			auto* func = globalFunctions.FindFirst(in_name, ns);
			if( func )
				return func;
		}

		for (int i = 0, Count = importedModules.GetLength(); i < Count; ++i)
		{
			auto* importModule = importedModules[i];
			{
				auto* func = importModule->globalFunctions.FindFirst(in_name, ns);
				if( func )
					return func;
			}
		}

		// Recursively search parent namespaces
		ns = engine->GetParentNameSpace(ns);
	}

	return 0;
}

// interface
asUINT asCModule::GetImportedFunctionCount() const
{
	return (asUINT)bindInformations.GetLength();
}

// interface
int asCModule::GetImportedFunctionIndexByDecl(const char *decl) const
{
	asCBuilder bld(engine, const_cast<asCModule*>(this));

	// Don't write parser errors to the message callback
	bld.silent = true;

	asCScriptFunction func(engine, const_cast<asCModule*>(this), asFUNC_DUMMY);
	bld.ParseFunctionDeclaration(0, decl, &func, false, 0, 0, defaultNamespace);

	// TODO: optimize: Improve linear search
	// Search script functions for matching interface
	int id = -1;
	for( asUINT n = 0; n < bindInformations.GetLength(); ++n )
	{
		if( func.name == bindInformations[n]->importedFunctionSignature->name &&
			func.returnType == bindInformations[n]->importedFunctionSignature->returnType &&
			func.parameterTypes.GetLength() == bindInformations[n]->importedFunctionSignature->parameterTypes.GetLength() )
		{
			bool match = true;
			for( asUINT p = 0; p < func.parameterTypes.GetLength(); ++p )
			{
				if( func.parameterTypes[p] != bindInformations[n]->importedFunctionSignature->parameterTypes[p] )
				{
					match = false;
					break;
				}
			}

			if( match )
			{
				if( id == -1 )
					id = n;
				else
					return asMULTIPLE_FUNCTIONS;
			}
		}
	}

	if( id == -1 ) return asNO_FUNCTION;

	return id;
}

// interface
asUINT asCModule::GetFunctionCount() const
{
	return (asUINT)globalFunctionList.GetLength();
}

// interface
asIScriptFunction *asCModule::GetFunctionByDecl(const char *decl) const
{
	asCBuilder bld(engine, const_cast<asCModule*>(this));

	// Don't write parser errors to the message callback
	bld.silent = true;

	asCScriptFunction func(engine, const_cast<asCModule*>(this), asFUNC_DUMMY);
	int r = bld.ParseFunctionDeclaration(0, decl, &func, false, 0, 0, defaultNamespace);
	if( r < 0 )
	{
		// Invalid declaration
		// TODO: Write error to message stream
		return 0;
	}

	// Use the defaultNamespace implicitly unless an explicit namespace has been provided
	asSNameSpace *ns = func.nameSpace == engine->nameSpaces[0] ? defaultNamespace : func.nameSpace;

	if (auto* FoundFunction = GetFunctionByDecl(ns, func))
		return FoundFunction;

	for (int i = 0, Count = importedModules.GetLength(); i < Count; ++i)
	{
		auto* importModule = importedModules[i];
		if (auto* FoundFunction = importModule->GetFunctionByDecl(ns, func))
			return FoundFunction;
	}

	return 0;
}

asIScriptFunction *asCModule::GetFunctionByDecl(asSNameSpace* ns, asCScriptFunction& func) const
{
	// Search script functions for matching interface
	while( ns )
	{
		asIScriptFunction *f = 0;
		bool bMultiple = false;
		{
		globalFunctions.FindAllUntil(func.name.AddressOf(), ns, [&](asCScriptFunction* funcPtr)
		{
			if( funcPtr->objectType == 0 &&
				func.returnType                 == funcPtr->returnType &&
				func.parameterTypes.GetLength() == funcPtr->parameterTypes.GetLength()
				)
			{
				bool match = true;
				for( asUINT p = 0; p < func.parameterTypes.GetLength(); ++p )
				{
					if( func.parameterTypes[p] != funcPtr->parameterTypes[p] )
					{
						match = false;
						break;
					}
				}

				if( match )
				{
					if( f == 0 )
					{
						f = const_cast<asCScriptFunction*>(funcPtr);
					}
					else
					{
						bMultiple = true;
						return true;
					}
				}
			}

			return false;
		});

		if (bMultiple)
			return nullptr;
		if( f )
			return f;
		}
		
		// Search for matching functions in the parent namespace
		ns = engine->GetParentNameSpace(ns);
	}

	return nullptr;
}

asIScriptFunction* asCModule::GetFunctionByName(asSNameSpace* ns, const char* funcName) const
{
	// Search script functions for matching interface
	while( ns )
	{
		auto* func = globalFunctions.FindFirst(funcName, ns);
		if(func)
			return func;
		
		// Search for matching functions in the parent namespace
		ns = engine->GetParentNameSpace(ns);
	}

	return nullptr;
}

// interface
asUINT asCModule::GetGlobalVarCount() const
{
	return (asUINT)scriptGlobalsList.GetLength();
}

// interface
int asCModule::RemoveGlobalVar(asUINT index)
{
	if(index >= scriptGlobalsList.GetLength())
		return asINVALID_ARG;
	asCGlobalProperty *prop = scriptGlobalsList[index];
	if( !prop )
		return asINVALID_ARG;

	// Remove the global variable from the module
	scriptGlobals.Remove(prop);
	scriptGlobalsList.RemoveIndex(index);

	// If the global variables have already been initialized 
	// then uninitialize the variable before it is removed
	if (isGlobalVarInitialized)
		UninitializeGlobalProp(prop);

	// Destroy the internal of the global variable (removes the initialization function)
	prop->DestroyInternal();

	// Check if the module is the only one referring to the property, if so remove it from the engine too
	// If the property is not removed now, it will be removed later when the module is discarded
	if( prop->refCount.get() == 2 )
		engine->RemoveGlobalProperty(prop);

	prop->Release();

	return 0;
}

// interface
void *asCModule::GetAddressOfGlobalVar(asUINT index)
{
	if(index >= scriptGlobalsList.GetLength())
		return nullptr;
	asCGlobalProperty *prop = scriptGlobalsList[index];
	if( !prop )
		return 0;

	// For object variables it's necessary to dereference the pointer to get the address of the value
	if( prop->type.IsObject() &&
		!prop->type.IsObjectHandle() )
		return *(void**)(prop->GetAddressOfValue());

	return (void*)(prop->GetAddressOfValue());
}

// interface
const char *asCModule::GetGlobalVarDeclaration(asUINT index, bool includeNamespace) const
{
	if(index >= scriptGlobalsList.GetLength())
		return nullptr;
	const asCGlobalProperty *prop = scriptGlobalsList[index];
	if( !prop )
		return 0;

	asCString *tempString = &asCThreadManager::GetLocalData()->string;
	*tempString = prop->type.Format(defaultNamespace);
	*tempString += " ";
	if( includeNamespace && prop->nameSpace->name != "" )
		*tempString += prop->nameSpace->name + "::";
	*tempString += prop->name;

	return tempString->AddressOf();
}

// interface
int asCModule::GetGlobalVar(asUINT index, const char **out_name, const char **out_nameSpace, int *out_typeId, bool *out_isConst) const
{
	if(index >= scriptGlobalsList.GetLength())
		return 0;
	const asCGlobalProperty *prop = scriptGlobalsList[index];
	if (!prop) return 0;

	if( out_name )
		*out_name = prop->name.AddressOf();
	if( out_nameSpace )
		*out_nameSpace = prop->nameSpace->name.AddressOf();
	if( out_typeId )
		*out_typeId = engine->GetTypeIdFromDataType(prop->type);
	if( out_isConst )
		*out_isConst = prop->type.IsReadOnly();

	return asSUCCESS;
}

// interface
asUINT asCModule::GetObjectTypeCount() const
{
	return (asUINT)classTypes.GetLength();
}

// interface
asITypeInfo *asCModule::GetObjectTypeByIndex(asUINT index) const
{
	if( index >= classTypes.GetLength() )
		return 0;

	return classTypes[index];
}

// interface
asITypeInfo *asCModule::GetTypeInfoByName(const char *in_name) const
{
	asSNameSpace *ns = defaultNamespace;
	while (ns)
	{
		for (asUINT n = 0; n < classTypes.GetLength(); n++)
		{
			if (classTypes[n] &&
				classTypes[n]->name == in_name &&
				classTypes[n]->nameSpace == ns)
				return classTypes[n];
		}

		for (asUINT n = 0; n < enumTypes.GetLength(); n++)
		{
			if (enumTypes[n] &&
				enumTypes[n]->name == in_name &&
				enumTypes[n]->nameSpace == ns)
				return enumTypes[n];
		}

		for (asUINT n = 0; n < typeDefs.GetLength(); n++)
		{
			if (typeDefs[n] &&
				typeDefs[n]->name == in_name &&
				typeDefs[n]->nameSpace == ns)
				return typeDefs[n];
		}

		// Recursively search parent namespace
		ns = engine->GetParentNameSpace(ns);
	}

	return 0;
}

// interface
int asCModule::GetTypeIdByDecl(const char *decl) const
{
	asCDataType dt;

	// This const cast is safe since we know the engine won't be modified
	asCBuilder bld(engine, const_cast<asCModule*>(this));

	// Don't write parser errors to the message callback
	bld.silent = true;

	int r = bld.ParseDataType(decl, &dt, defaultNamespace);
	if( r < 0 )
		return asINVALID_TYPE;

	return engine->GetTypeIdFromDataType(dt);
}

// interface
asITypeInfo *asCModule::GetTypeInfoByDecl(const char *decl) const
{
	asCDataType dt;

	// This const cast is safe since we know the engine won't be modified
	asCBuilder bld(engine, const_cast<asCModule*>(this));

	// Don't write parser errors to the message callback
	bld.silent = true;

	int r = bld.ParseDataType(decl, &dt, defaultNamespace);
	if (r < 0)
		return 0;

	return dt.GetTypeInfo();
}

// interface
asUINT asCModule::GetEnumCount() const
{
	return enumTypes.GetLength();
}

// interface
asITypeInfo *asCModule::GetEnumByIndex(asUINT index) const
{
	if( index >= enumTypes.GetLength() )
		return 0;

	return enumTypes[index];
}

// interface
asUINT asCModule::GetTypedefCount() const
{
	return (asUINT)typeDefs.GetLength();
}

// interface
asITypeInfo *asCModule::GetTypedefByIndex(asUINT index) const
{
	if( index >= typeDefs.GetLength() )
		return 0;

	return typeDefs[index];
}

// internal
int asCModule::GetNextImportedFunctionId()
{
	// TODO: multithread: This will break if one thread if freeing a module, while another is being compiled
	if( engine->freeImportedFunctionIdxs.GetLength() )
		return FUNC_IMPORTED | (asUINT)engine->freeImportedFunctionIdxs[engine->freeImportedFunctionIdxs.GetLength()-1];

	return FUNC_IMPORTED | (asUINT)engine->importedFunctions.GetLength();
}

#ifndef AS_NO_COMPILER
// internal
int asCModule::AddScriptFunction(int sectionIdx, int declaredAt, int id, const asCString &funcName, const asCDataType &returnType, const asCArray<asCDataType> &params, const asCArray<asCString> &paramNames, const asCArray<asETypeModifiers> &inOutFlags, const asCArray<asCString *> &defaultArgs, bool isInterface, asCObjectType *objType, bool isGlobalFunction, asSFunctionTraits funcTraits, asSNameSpace *ns)
{
	asASSERT(id >= 0);

	// Store the function information
	asCScriptFunction *func = asNEW(asCScriptFunction)(engine, this, isInterface ? asFUNC_INTERFACE : asFUNC_SCRIPT);
	if( func == 0 )
	{
		// Free the default args
		for( asUINT n = 0; n < defaultArgs.GetLength(); n++ )
			if( defaultArgs[n] )
				asDELETE(defaultArgs[n], asCString);

		return asOUT_OF_MEMORY;
	}

	if( ns == 0 )
		ns = engine->nameSpaces[0];

	// All methods of shared objects are also shared
	if( objType && objType->IsShared() )
		funcTraits.SetTrait(asTRAIT_SHARED, true);

	func->name             = funcName;
	func->nameSpace        = ns;
	func->id               = id;
	func->returnType       = returnType;
	if( func->funcType == asFUNC_SCRIPT )
	{
		func->scriptData->scriptSectionIdx = sectionIdx;
		func->scriptData->declaredAt = declaredAt;
	}
	func->parameterTypes   = params;
	func->parameterNames   = paramNames;
	func->inOutFlags       = inOutFlags;
	func->defaultArgs      = defaultArgs;
	func->objectType       = objType;
	if( objType )
		objType->AddRefInternal();
	func->traits           = funcTraits;

	asASSERT( params.GetLength() == inOutFlags.GetLength() && params.GetLength() == defaultArgs.GetLength() );

	// Verify that we are not assigning either the final or override specifier(s) if we are registering a non-member function
	asASSERT( !(!objType && funcTraits.GetTrait(asTRAIT_FINAL)) );
	asASSERT( !(!objType && funcTraits.GetTrait(asTRAIT_OVERRIDE)) );

	// The internal ref count was already set by the constructor
	scriptFunctions.PushLast(func);
	engine->AddScriptFunction(func);


	// Add reference
	if( isGlobalFunction )
	{
		globalFunctions.Add(func);
		globalFunctionList.PushLast(func);
	}

	return 0;
}

// internal
int asCModule::AddScriptFunction(asCScriptFunction *func)
{
	scriptFunctions.PushLast(func);
	func->AddRefInternal();
	engine->AddScriptFunction(func);

	// If the function that is being added is an already compiled shared function
	// then it is necessary to look for anonymous functions that may be declared
	// within it and add those as well
	if( func->IsShared() && func->funcType == asFUNC_SCRIPT )
	{
		// Loop through the byte code and check all the
		// asBC_FuncPtr instructions for anonymous functions
		asDWORD *bc = func->scriptData->byteCode.AddressOf();
		asUINT bcLength = (asUINT)func->scriptData->byteCode.GetLength();
		for( asUINT n = 0; n < bcLength; )
		{
			int c = *(asBYTE*)&bc[n];
			if( c == asBC_FuncPtr )
			{
				asCScriptFunction *f = reinterpret_cast<asCScriptFunction*>(asBC_PTRARG(&bc[n]));
				// Anonymous functions start with $
				// There are never two equal anonymous functions so it is not necessary to look for duplicates
				if( f && f->name[0] == '$' )
				{
					AddScriptFunction(f);
					globalFunctions.Add(f);
					globalFunctionList.PushLast(f);
				}
			}
			n += asBCTypeSize[asBCInfo[c].type];
		}
	}

	return 0;
}

// internal
int asCModule::AddImportedFunction(int id, const asCString &funcName, const asCDataType &returnType, const asCArray<asCDataType> &params, const asCArray<asETypeModifiers> &inOutFlags, const asCArray<asCString *> &defaultArgs, asSNameSpace *ns, const asCString &moduleName)
{
	asASSERT(id >= 0);

	// Store the function information
	asCScriptFunction *func = asNEW(asCScriptFunction)(engine, this, asFUNC_IMPORTED);
	if( func == 0 )
	{
		// Free the default args
		for( asUINT n = 0; n < defaultArgs.GetLength(); n++ )
			if( defaultArgs[n] )
				asDELETE(defaultArgs[n], asCString);

		return asOUT_OF_MEMORY;
	}

	func->name           = funcName;
	func->id             = id;
	func->returnType     = returnType;
	func->nameSpace      = ns;
	func->parameterTypes = params;
	func->inOutFlags     = inOutFlags;
	func->defaultArgs    = defaultArgs;
	func->objectType     = 0;
	func->CalculateParameterOffsets();

	sBindInfo *info = asNEW(sBindInfo);
	if( info == 0 )
	{
		asDELETE(func, asCScriptFunction);
		return asOUT_OF_MEMORY;
	}

	info->importedFunctionSignature = func;
	info->boundFunctionId           = -1;
	info->importFromModule          = moduleName;
	bindInformations.PushLast(info);

	// Add the info to the array in the engine
	if( engine->freeImportedFunctionIdxs.GetLength() )
		engine->importedFunctions[engine->freeImportedFunctionIdxs.PopLast()] = info;
	else
		engine->importedFunctions.PushLast(info);

	return 0;
}
#endif

// internal
asCScriptFunction *asCModule::GetImportedFunction(int index) const
{
	return bindInformations[index]->importedFunctionSignature;
}

// interface
int asCModule::BindImportedFunction(asUINT index, asIScriptFunction *func)
{
	// First unbind the old function
	int r = UnbindImportedFunction(index);
	if( r < 0 ) return r;

	// Must verify that the interfaces are equal
	asCScriptFunction *dst = GetImportedFunction(index);
	if( dst == 0 ) return asNO_FUNCTION;

	if( func == 0 )
		return asINVALID_ARG;

	asCScriptFunction *src = engine->GetScriptFunction(func->GetId());
	if( src == 0 )
		return asNO_FUNCTION;

	// Verify return type
	if( dst->returnType != src->returnType )
		return asINVALID_INTERFACE;

	if( dst->parameterTypes.GetLength() != src->parameterTypes.GetLength() )
		return asINVALID_INTERFACE;

	for( asUINT n = 0; n < dst->parameterTypes.GetLength(); ++n )
	{
		if( dst->parameterTypes[n] != src->parameterTypes[n] )
			return asINVALID_INTERFACE;
	}

	bindInformations[index]->boundFunctionId = src->GetId();
	src->AddRefInternal();

	return asSUCCESS;
}

// interface
int asCModule::UnbindImportedFunction(asUINT index)
{
	if( index >= bindInformations.GetLength() )
		return asINVALID_ARG;

	// Remove reference to old module
	if( bindInformations[index] )
	{
		int oldFuncID = bindInformations[index]->boundFunctionId;
		if( oldFuncID != -1 )
		{
			bindInformations[index]->boundFunctionId = -1;
			engine->scriptFunctions[oldFuncID]->ReleaseInternal();
		}
	}

	return asSUCCESS;
}

// interface
const char *asCModule::GetImportedFunctionDeclaration(asUINT index) const
{
	asCScriptFunction *func = GetImportedFunction(index);
	if( func == 0 ) return 0;

	asCString *tempString = &asCThreadManager::GetLocalData()->string;
	*tempString = func->GetDeclarationStr();

	return tempString->AddressOf();
}

// interface
const char *asCModule::GetImportedFunctionSourceModule(asUINT index) const
{
	if( index >= bindInformations.GetLength() )
		return 0;

	return bindInformations[index]->importFromModule.AddressOf();
}

// inteface
int asCModule::BindAllImportedFunctions()
{
	bool notAllFunctionsWereBound = false;

	// Bind imported functions
	int c = GetImportedFunctionCount();
	for( int n = 0; n < c; ++n )
	{
		asCScriptFunction *importFunc = GetImportedFunction(n);
		if( importFunc == 0 ) return asERROR;

		asCString str = importFunc->GetDeclarationStr(false, true);

		// Get module name from where the function should be imported
		const char *moduleName = GetImportedFunctionSourceModule(n);
		if( moduleName == 0 ) return asERROR;

		asCModule *srcMod = engine->GetModule(moduleName, false);
		asIScriptFunction *func = 0;
		if( srcMod )
			func = srcMod->GetFunctionByDecl(str.AddressOf());

		if( func == 0 )
			notAllFunctionsWereBound = true;
		else
		{
			if( BindImportedFunction(n, func) < 0 )
				notAllFunctionsWereBound = true;
		}
	}

	if( notAllFunctionsWereBound )
		return asCANT_BIND_ALL_FUNCTIONS;

	return asSUCCESS;
}

// interface
int asCModule::UnbindAllImportedFunctions()
{
	asUINT c = GetImportedFunctionCount();
	for( asUINT n = 0; n < c; ++n )
		UnbindImportedFunction(n);

	return asSUCCESS;
}

//interface
void asCModule::ImportModule(asIScriptModule* in_module)
{
	asCModule* module = (asCModule*)in_module;
	if (module != nullptr)
	{
		// Add outer module to import list
		if (importedModules.IndexOf(module) == -1)
			importedModules.PushLast(module);

		// Flatten list of imports
		for (int i = 0, Count = module->importedModules.GetLength(); i < Count; ++i)
		{
			auto* subImport = module->importedModules[i];
			if (importedModules.IndexOf(subImport) == -1)
				importedModules.PushLast(subImport);
		}
	}
}

void asCModule::ClearImports()
{
}

// internal
asCTypeInfo *asCModule::GetType(const char *type, asSNameSpace *ns)
{
	asCTypeInfo* FoundType = nullptr;
	allLocalTypes.FindAllUntil(type, [&](asCTypeInfo* Type) -> bool
	{
		if (Type->nameSpace == ns || ns == nullptr)
		{
			FoundType = Type;
			return true;
		}
		return false;
	});

	if (FoundType != nullptr)
		return FoundType;

	for (int i = 0, Count = importedModules.GetLength(); i < Count; ++i)
	{
		auto* importModule = importedModules[i];

		importModule->allLocalTypes.FindAllUntil(type, [&](asCTypeInfo* Type) -> bool
		{
			if (Type->nameSpace == ns || ns == nullptr)
			{
				FoundType = Type;
				return true;
			}
			return false;
		});

		if (FoundType != nullptr)
			return FoundType;
	}

	return nullptr;
}

// internal
asCObjectType *asCModule::GetObjectType(const char *type, asSNameSpace *ns)
{
	asUINT n;

	// TODO: optimize: Improve linear search
	for( n = 0; n < classTypes.GetLength(); n++ )
		if( classTypes[n]->name == type &&
			classTypes[n]->nameSpace == ns )
			return classTypes[n];

	return 0;
}

// internal
asCGlobalProperty *asCModule::AllocateGlobalProperty(const char *propName, const asCDataType &dt, asSNameSpace *ns)
{
	asCGlobalProperty *prop = engine->AllocateGlobalProperty();
	prop->name = propName;
	prop->nameSpace = ns;
	prop->module = this;

	// Allocate the memory for this property based on its type
	prop->type = dt;
	prop->AllocateMemory();

	// Make an entry in the address to variable map
	engine->varAddressMap.Add(prop->GetAddressOfValue(), prop);

	// Store the variable in the module scope
	scriptGlobals.Add(prop);
	scriptGlobalsList.PushLast(prop);
	prop->AddRef();

	return prop;
}

// internal
bool asCModule::IsEmpty() const
{
	if( scriptFunctions.GetLength()  ) return false;
	if( globalFunctionList.GetLength()    ) return false;
	if( bindInformations.GetLength() ) return false;
	if( scriptGlobalsList.GetLength()      ) return false;
	if( classTypes.GetLength()       ) return false;
	if( enumTypes.GetLength()        ) return false;
	if( typeDefs.GetLength()         ) return false;
	if( funcDefs.GetLength()         ) return false;

	return true;
}

// interface
int asCModule::SaveByteCode(asIBinaryStream *out, bool stripDebugInfo) const
{
	return asNOT_SUPPORTED;
}

// interface
int asCModule::LoadByteCode(asIBinaryStream *in, bool *wasDebugInfoStripped)
{
	return asNOT_SUPPORTED;
}

// interface
int asCModule::CompileFunction(const char *sectionName, const char *code, int lineOffset, asDWORD compileFlags, asIScriptFunction **outFunc)
{
	// Make sure the outFunc is null if the function fails, so the
	// application doesn't attempt to release a non-existent function
	if( outFunc )
		*outFunc = 0;

#ifdef AS_NO_COMPILER
	UNUSED_VAR(sectionName);
	UNUSED_VAR(code);
	UNUSED_VAR(lineOffset);
	UNUSED_VAR(compileFlags);
	return asNOT_SUPPORTED;
#else
	// Validate arguments
	if( code == 0 ||
		(compileFlags != 0 && compileFlags != asCOMP_ADD_TO_MODULE) )
		return asINVALID_ARG;

	// Only one thread may build at one time
	// TODO: It should be possible to have multiple threads perform compilations
	int r = engine->RequestBuild();
	if( r < 0 )
		return r;

	// Prepare the engine
	engine->PrepareEngine();
	if( engine->configFailed )
	{
		engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, TXT_INVALID_CONFIGURATION);
		engine->BuildCompleted();
		return asINVALID_CONFIGURATION;
	}

	// Compile the single function
	asCBuilder funcBuilder(engine, this);
	asCString str = code;
	asCScriptFunction *func = 0;
	r = funcBuilder.CompileFunction(sectionName, str.AddressOf(), lineOffset, compileFlags, &func);

	engine->BuildCompleted();

	if( r >= 0 && outFunc && func )
	{
		// Return the function to the caller and add an external reference
		*outFunc = func;
		func->AddRef();
	}

	// Release our reference to the function
	if( func )
		func->ReleaseInternal();

	return r;
#endif
}

// interface
int asCModule::RemoveFunction(asIScriptFunction *func)
{
	// Find the global function
	asCScriptFunction *f = static_cast<asCScriptFunction*>(func);
	if(globalFunctions.Contains(f))
	{
		globalFunctions.Remove(f);
		globalFunctionList.RemoveValue(f);
		scriptFunctions.RemoveValue(f);
		f->ReleaseInternal();
		return 0;
	}

	return asNO_FUNCTION;
}

void asCModule::AddPreClassData(const char* preClassName, asPreClassData Data)
{
	PreClassData.Add(asCString(preClassName), Data);
}

#ifndef AS_NO_COMPILER
// internal
int asCModule::AddFuncDef(const asCString &funcName, asSNameSpace *ns, asCObjectType *parent)
{
	// namespace and parent are mutually exclusive
	asASSERT((ns == 0 && parent) || (ns && parent == 0));

	asCScriptFunction *func = asNEW(asCScriptFunction)(engine, 0, asFUNC_FUNCDEF);
	if (func == 0)
		return asOUT_OF_MEMORY;

	func->name      = funcName;
	func->nameSpace = ns;
	func->module    = this;

	asCFuncdefType *fdt = asNEW(asCFuncdefType)(engine, func);
	funcDefs.PushLast(fdt); // The constructor set the refcount to 1
	allLocalTypes.Add(fdt);

	engine->funcDefs.PushLast(fdt); // doesn't increase refcount
	func->id = engine->GetNextScriptFunctionId();
	engine->AddScriptFunction(func);

	if (parent)
	{
		parent->childFuncDefs.PushLast(fdt);
		fdt->parentClass = parent;
	}

	return (int)funcDefs.GetLength()-1;
}
#endif

// interface
asDWORD asCModule::SetAccessMask(asDWORD mask)
{
	asDWORD old = accessMask;
	accessMask = mask;
	return old;
}

asCGlobalProperty* asCModule::GetGlobalProperty(asSNameSpace* ns, const char* prop)
{
	auto* globProp = scriptGlobals.FindFirst(prop, ns);
	if( globProp )
		return globProp;

	for (int i = 0, Count = importedModules.GetLength(); i < Count; ++i)
	{
		auto* importModule = importedModules[i];
		auto* importProp = importModule->scriptGlobals.FindFirst(prop, ns);
		if( importProp )
			return importProp;
	}

	return nullptr;
}

void asCModule::FindGlobalFunctions(const char* funcName, asSNameSpace* ns, asCArray<int>& outFuncs, bool lookForMixins)
{
	// Get the script declared global functions
	{
		globalFunctions.FindAll(funcName, ns, [&](asCScriptFunction* f)
		{
			if (lookForMixins != f->IsMixin())
				return;
			asASSERT(f->objectType == 0);
			outFuncs.PushLast(f->id);
		});
	}

	for (int i = 0, Count = importedModules.GetLength(); i < Count; ++i)
	{
		auto* importModule = importedModules[i];
		importModule->globalFunctions.FindAll(funcName, ns, [&](asCScriptFunction* f)
		{
			if (lookForMixins != f->IsMixin())
				return;
			asASSERT( f->objectType == 0 );
			outFuncs.PushLast(f->id);
		});
	}
}

void asCModule::RemoveTypesAndGlobalsFromEngineAvailability()
{
	for (asUINT n = 0, count = classTypes.GetLength(); n < count; ++n)
	{
		if (classTypes[n])
			engine->allScriptDeclaredTypes.Remove(classTypes[n]);
	}

	for (asUINT n = 0, count = enumTypes.GetLength(); n < count; ++n)
	{
		if (enumTypes[n])
			engine->allScriptDeclaredTypes.Remove(enumTypes[n]);
	}

	{
		for (asUINT n = 0, count = scriptGlobalsList.GetLength(); n < count; ++n)
		{
			asCGlobalProperty *desc = scriptGlobalsList[n];
			if (desc)
				engine->allScriptGlobalVariables.Remove(desc);
		}
	}

	{
		for (asUINT n = 0, count = globalFunctionList.GetLength(); n < count; ++n)
		{
			asCScriptFunction *desc = globalFunctionList[n];
			if (desc)
				engine->allScriptGlobalFunctions.Remove(desc);
		}
	}
}

void asCModule::CollectUpdatedTypeReferences(asCModule* OldModule, asModuleReferenceUpdateMap& OutUpdateMap)
{
	OutUpdateMap.Modules.Add(OldModule, this);

	// Match each old class to a new class
	for (int ClassIndex = 0, ClassCount = OldModule->classTypes.GetLength(); ClassIndex < ClassCount; ++ClassIndex)
	{
		asCObjectType* OldClass = OldModule->classTypes[ClassIndex];
		asCObjectType* NewClass = nullptr;

		allLocalTypes.FindAllUntil(
			OldClass->name.AddressOf(),
			[&](asCTypeInfo* Type) -> bool
			{
				if ((Type->GetFlags() & asOBJ_SCRIPT_OBJECT) == 0)
					return false;
				if ((Type->GetFlags() & asOBJ_VALUE) != (OldClass->GetFlags() & asOBJ_VALUE))
					return false;

				if (Type->nameSpace == OldClass->nameSpace)
				{
					NewClass = (asCObjectType*)Type;
					return true;
				}

				return false;
			});

		if (NewClass != nullptr)
		{
			OutUpdateMap.Types.Add(OldClass, NewClass);
		}
	}

	// Match each old enum to a new enum
	for (int EnumIndex = 0, EnumCount = OldModule->enumTypes.GetLength(); EnumIndex < EnumCount; ++EnumIndex)
	{
		asCEnumType* OldEnum = OldModule->enumTypes[EnumIndex];
		asCEnumType* NewEnum = nullptr;

		allLocalTypes.FindAllUntil(
			OldEnum->name.AddressOf(),
			[&](asCTypeInfo* Type) -> bool
			{
				if ((Type->GetFlags() & asOBJ_ENUM) == 0)
					return false;

				if (Type->nameSpace == OldEnum->nameSpace)
				{
					NewEnum = (asCEnumType*)Type;
					return true;
				}

				return false;
			});

		if (NewEnum != nullptr)
		{
			OutUpdateMap.Types.Add(OldEnum, NewEnum);
		}
	}
}

static bool IsSameTypeInfoForReferenceUpdate(asModuleReferenceUpdateMap& OutUpdateMap, asCTypeInfo* Old, asCTypeInfo* New)
{
	if (Old == New)
		return true;
	if (Old == nullptr)
		return New == nullptr;
	else if (New == nullptr)
		return false;

	asCTypeInfo** Replacement = OutUpdateMap.Types.Find(Old);
	if (Replacement == nullptr)
	{
		// If this is a template type we still need to check if all template subtypes are the same,
		// because we haven't recreated a new template instance yet, but we probably will.
		if ((Old->flags & asOBJ_TEMPLATE) != 0 && (New->flags & asOBJ_TEMPLATE) != 0)
		{
			asCObjectType* OldTemplate = (asCObjectType*)Old;
			asCObjectType* NewTemplate = (asCObjectType*)New;

			if (OldTemplate->templateBaseType != NewTemplate->templateBaseType)
				return false;
			if (OldTemplate->templateSubTypes.GetLength() != NewTemplate->templateSubTypes.GetLength())
				return false;

			for (int i = 0, Count = OldTemplate->templateSubTypes.GetLength(); i < Count; ++i)
			{
				if (!OldTemplate->templateSubTypes[i].IsEqualExceptTypeInfo(NewTemplate->templateSubTypes[i]))
					return false;
				if (!IsSameTypeInfoForReferenceUpdate(OutUpdateMap, OldTemplate->templateSubTypes[i].GetTypeInfo(), NewTemplate->templateSubTypes[i].GetTypeInfo()))
					return false;
			}

			return true;
		}

		return false;
	}
	return *Replacement == New;
}

void asCModule::DiffForReferenceUpdate(asCModule* OldModule, asModuleReferenceUpdateMap& OutUpdateMap, bool& OutHadStructuralChanges)
{
	auto IsSameTypeInfo = [&](asCTypeInfo* Old, asCTypeInfo* New) -> bool
	{
		return IsSameTypeInfoForReferenceUpdate(OutUpdateMap, Old, New);
	};

	auto IsSameDataType = [&](const asCDataType& Old, const asCDataType& New) -> bool
	{
		if (!Old.IsEqualExceptTypeInfo(New))
			return false;

		if (!IsSameTypeInfoForReferenceUpdate(OutUpdateMap, Old.GetTypeInfo(), New.GetTypeInfo()))
			return false;

		return true;
	};

	auto IsSameFunctionSignature = [&](asCScriptFunction* Old, asCScriptFunction* New) -> bool
	{
		if (Old->parameterTypes.GetLength() != New->parameterTypes.GetLength())
			return false;
		if (Old->parameterNames.GetLength() != New->parameterNames.GetLength())
			return false;
		if (Old->defaultArgs.GetLength() != New->defaultArgs.GetLength())
			return false;
		if (Old->traits.traits != New->traits.traits)
			return false;
		if (!IsSameDataType(Old->returnType, New->returnType))
			return false;

		for (int i = 0, Count = Old->parameterTypes.GetLength(); i < Count; ++i)
		{
			if (!IsSameDataType(Old->parameterTypes[i], New->parameterTypes[i]))
				return false;
			if (Old->inOutFlags[i] != New->inOutFlags[i])
				return false;
		}

		for (int i = 0, Count = Old->parameterNames.GetLength(); i < Count; ++i)
		{
			if (Old->parameterNames[i] != New->parameterNames[i])
				return false;
		}

		for (int i = 0, Count = Old->defaultArgs.GetLength(); i < Count; ++i)
		{
			if (Old->defaultArgs[i] != nullptr)
			{
				if (New->defaultArgs[i] == nullptr)
					return false;
				if (*Old->defaultArgs[i] != *New->defaultArgs[i])
					return false;
			}
			else
			{
				if (New->defaultArgs[i] != nullptr)
					return false;
			}
		}

		if (Old->accessSpecifier != nullptr)
		{
			if (New->accessSpecifier == nullptr)
				return false;

			if (Old->accessSpecifier->name != New->accessSpecifier->name)
				return false;
		}
		else
		{
			if (New->accessSpecifier != nullptr)
				return false;
		}

		return true;
	};

	// Match each old class to a new class
	const uint32 StructuralChangeTypeFlagsMask = ~asOBJ_POD;
	for (int ClassIndex = 0, ClassCount = OldModule->classTypes.GetLength(); ClassIndex < ClassCount; ++ClassIndex)
	{
		asCObjectType* OldClass = OldModule->classTypes[ClassIndex];
		asCObjectType* NewClass = (asCObjectType*)OutUpdateMap.Types.FindRef(OldClass);

		if (NewClass == nullptr)
		{
			OutHadStructuralChanges = true;
			continue;
		}

		// If something about the class has changed, mark it as a structural change
		if (OldClass->shadowType != NewClass->shadowType
			|| !IsSameTypeInfo(OldClass->derivedFrom, NewClass->derivedFrom)
			|| (OldClass->flags & StructuralChangeTypeFlagsMask) != (NewClass->flags & StructuralChangeTypeFlagsMask)
		)
		{
			OutHadStructuralChanges = true;
		}

		// Match all functions in the methods list, this will be either non-virtuals (structs), or virtual stubs
		int OldLocalMethodCount = 0;
		auto MatchFunctionId = [&](int FunctionId)
		{
			if (FunctionId == 0)
				return;
			asCScriptFunction* OldFunction = engine->scriptFunctions[FunctionId];
			if (OldFunction->objectType != OldClass)
				return;

			OldLocalMethodCount += 1;

			check(OldFunction->funcType == asEFuncType::asFUNC_SCRIPT);
			asCScriptFunction* NewFunction = nullptr;

			NewClass->methodTable.FindAllUntil(
				OldFunction->name.AddressOf(),
				[&](asCScriptFunction* Function) -> bool
				{
					if (Function->objectType != NewClass)
						return false;

					if (!IsSameFunctionSignature(OldFunction, Function))
						return false;

					NewFunction = Function;
					return true;
				});

			if (NewFunction != nullptr)
			{
				OutUpdateMap.Functions.Add(OldFunction, NewFunction);
			}
			else
			{
				OutHadStructuralChanges = true;
			}
		};

		for (int FunctionIndex = 0, FunctionCount = OldClass->methods.GetLength(); FunctionIndex < FunctionCount; ++FunctionIndex)
		{
			MatchFunctionId(OldClass->methods[FunctionIndex]);
		}

		if (NewClass->methods.GetLength() != OldLocalMethodCount)
			OutHadStructuralChanges = true;

		// Match constructors
		if (OldClass->beh.constructors.GetLength() == NewClass->beh.constructors.GetLength())
		{
			for (int i = 0, Count = OldClass->beh.constructors.GetLength(); i < Count; ++i)
			{
				asCScriptFunction* OldFunction = engine->scriptFunctions[OldClass->beh.constructors[i]];
				asCScriptFunction* NewFunction = engine->scriptFunctions[NewClass->beh.constructors[i]];

				if (IsSameFunctionSignature(OldFunction, NewFunction))
				{
					OutUpdateMap.Functions.Add(OldFunction, NewFunction);
				}
				else
				{
					OutHadStructuralChanges = true;
				}
			}
		}
		else
		{
			OutHadStructuralChanges = true;
		}

		// Match factories
		if (OldClass->beh.factories.GetLength() == NewClass->beh.factories.GetLength())
		{
			for (int i = 0, Count = OldClass->beh.factories.GetLength(); i < Count; ++i)
			{
				asCScriptFunction* OldFunction = engine->scriptFunctions[OldClass->beh.factories[i]];
				asCScriptFunction* NewFunction = engine->scriptFunctions[NewClass->beh.factories[i]];

				if (IsSameFunctionSignature(OldFunction, NewFunction))
				{
					OutUpdateMap.Functions.Add(OldFunction, NewFunction);
				}
				else
				{
					OutHadStructuralChanges = true;
				}
			}
		}
		else
		{
			OutHadStructuralChanges = true;
		}

		// Match destructor
		if (OldClass->beh.destruct != 0)
		{
			if (NewClass->beh.destruct == 0)
			{
				// If the old class had a destructor, we _must_ also have a destructor, or
				// all our replacement stuff goes out the window. We don't have the information about
				// parent properties to fix this yet, so we just add it always, right now
				sClassDeclaration* classDecl = nullptr;
				for (asUINT n = 0; n < builder->classDeclarations.GetLength(); n++)
				{
					if (builder->classDeclarations[n]->typeInfo == NewClass)
					{
						classDecl = builder->classDeclarations[n];
						break;
					}
				}

				builder->AddDefaultDestructor(NewClass, classDecl->script);
				check(NewClass->beh.destruct != 0)
			}

			asCScriptFunction* OldFunction = engine->scriptFunctions[OldClass->beh.destruct];
			asCScriptFunction* NewFunction = engine->scriptFunctions[NewClass->beh.destruct];

			OutUpdateMap.Functions.Add(OldFunction, NewFunction);
		}
		else
		{
			if (NewClass->beh.destruct != 0)
				OutHadStructuralChanges = true;
		}

		// Match all the properties
		if (OldClass->localProperties.GetLength() != NewClass->localProperties.GetLength())
			OutHadStructuralChanges = true;

		for (int PropertyIndex = 0, PropertyCount = OldClass->localProperties.GetLength(); PropertyIndex < PropertyCount; ++PropertyIndex)
		{
			asCObjectProperty* OldProperty = OldClass->localProperties[PropertyIndex];

			asCObjectProperty* NewProperty = nullptr;
			NewClass->propertyTable.FindAllUntil(
				OldProperty->name.AddressOf(),
				[&](asCObjectProperty* Property) -> bool
				{
					if (OldProperty->isPrivate != Property->isPrivate)
						return false;
					if (OldProperty->isProtected != Property->isProtected)
						return false;

					if (!IsSameDataType(OldProperty->type, Property->type))
						return false;

					if (OldProperty->accessSpecifier != nullptr)
					{
						if (Property->accessSpecifier == nullptr)
							return false;

						if (OldProperty->accessSpecifier->name != Property->accessSpecifier->name)
							return false;
					}
					else
					{
						if (Property->accessSpecifier != nullptr)
							return false;
					}

					NewProperty = Property;
					return true;
				});

			if (NewProperty != nullptr)
			{
				OutUpdateMap.ObjectProperties.Add(OldProperty, NewProperty);
				OutUpdateMap.NewObjectPropertiesByOldTypeIdAndOldOffset.Add(
					TPair<int, int>(OldClass->GetTypeId(), OldProperty->byteOffset), NewProperty);
			}
			else
			{
				OutHadStructuralChanges = true;
			}
		}

		// Check if any access specifiers have changed
		for (int AccessSpecifierIndex = 0, AccessSpecifierCount = OldClass->accessSpecifiers.GetLength(); AccessSpecifierIndex < AccessSpecifierCount; ++AccessSpecifierIndex)
		{
			asSAccessSpecifier* OldAccessSpecifier = OldClass->accessSpecifiers[AccessSpecifierIndex];
			asSAccessSpecifier* NewAccessSpecifier = NewClass->GetAccessSpecifier(OldAccessSpecifier->name.AddressOf());

			if (NewAccessSpecifier != nullptr)
			{
				if (OldAccessSpecifier->bIsPrivate != NewAccessSpecifier->bIsPrivate)
					OutHadStructuralChanges = true;
				if (OldAccessSpecifier->bIsProtected != NewAccessSpecifier->bIsProtected)
					OutHadStructuralChanges = true;
				if (OldAccessSpecifier->bAnyReadOnly != NewAccessSpecifier->bAnyReadOnly)
					OutHadStructuralChanges = true;
				if (OldAccessSpecifier->bAnyEditDefaults != NewAccessSpecifier->bAnyEditDefaults)
					OutHadStructuralChanges = true;

				if (OldAccessSpecifier->permissions.GetLength() != NewAccessSpecifier->permissions.GetLength())
				{
					OutHadStructuralChanges = true;
				}
				else
				{
					for (int i = 0, Count = OldAccessSpecifier->permissions.GetLength(); i < Count; ++i)
					{
						auto& OldPermission = OldAccessSpecifier->permissions[i];
						auto& NewPermission = NewAccessSpecifier->permissions[i];

						if (OldPermission.accessName != NewPermission.accessName
							|| OldPermission.bInherited != NewPermission.bInherited
							|| OldPermission.bReadOnly != NewPermission.bReadOnly
							|| OldPermission.bEditDefaults != NewPermission.bEditDefaults)
						{
							OutHadStructuralChanges = true;
							break;
						}
					}
				}
			}
			else
			{
				OutHadStructuralChanges = true;
			}
		}
	}

	// Match each old enum to a new enum
	for (int EnumIndex = 0, EnumCount = OldModule->enumTypes.GetLength(); EnumIndex < EnumCount; ++EnumIndex)
	{
		asCEnumType* OldEnum = OldModule->enumTypes[EnumIndex];
		asCEnumType* NewEnum = (asCEnumType*)OutUpdateMap.Types.FindRef(OldEnum);

		if (NewEnum == nullptr)
		{
			OutHadStructuralChanges = true;
			continue;
		}

		// If the enum has changed, this is a structural change
		// TODO: This runs _before_ the enum receives its values, so we need to fix that!
		if (OldEnum->enumValues.GetLength() == NewEnum->enumValues.GetLength())
		{
			for (int i = 0, Count = OldEnum->enumValues.GetLength(); i < Count; ++i)
			{
				if (OldEnum->enumValues[i]->value != NewEnum->enumValues[i]->value
					|| OldEnum->enumValues[i]->name != NewEnum->enumValues[i]->name)
				{
					OutHadStructuralChanges = true;
					break;
				}
			}
		}
		else
		{
			OutHadStructuralChanges = true;
		}
	}

	// Match global functions
	for (int FunctionIndex = 0, FunctionCount = OldModule->globalFunctionList.GetLength(); FunctionIndex < FunctionCount; ++FunctionIndex)
	{
		asCScriptFunction* OldFunction = OldModule->globalFunctionList[FunctionIndex];
		if (OldFunction == nullptr)
			continue;

		asCScriptFunction* NewFunction = nullptr; 
		globalFunctions.FindAllUntil(OldFunction->name.AddressOf(), OldFunction->nameSpace, [&](asCScriptFunction* Function)
		{
			if (Function == nullptr)
				return false;

			if (!IsSameFunctionSignature(OldFunction, Function))
				return false;

			NewFunction = Function;
			return true;
		});

		if (NewFunction != nullptr)
		{
			OutUpdateMap.Functions.Add(OldFunction, NewFunction);
		}
		else
		{
			OutHadStructuralChanges = true;
		}
	}

	// Match global properties
	for (int GlobalIndex = 0, GlobalCount = OldModule->scriptGlobalsList.GetLength(); GlobalIndex < GlobalCount; ++GlobalIndex)
	{
		asCGlobalProperty* OldGlobal = OldModule->scriptGlobalsList[GlobalIndex];
		if (OldGlobal == nullptr)
			continue;

		asCGlobalProperty* NewGlobal = nullptr; 
		scriptGlobals.FindAllUntil(OldGlobal->name.AddressOf(), OldGlobal->nameSpace, [&](asCGlobalProperty* Global)
		{
			if (Global == nullptr)
				return false;
			if (!IsSameDataType(OldGlobal->type, Global->type))
				return false;

			NewGlobal = Global;
			return true;
		});

		if (NewGlobal != nullptr)
		{
			OutUpdateMap.GlobalProperties.Add(OldGlobal, NewGlobal);
		}
		else
		{
			OutHadStructuralChanges = true;
		}
	}
}

static void UpdateReferenceInTypeInfo(const asModuleReferenceUpdateMap& UpdateMap, asCTypeInfo*& TypeInfo, bool bHoldsRefCount)
{
	if (TypeInfo == nullptr)
		return;
	if (TypeInfo->module == nullptr)
		return;
	asCTypeInfo* Replacement = UpdateMap.Types.FindRef(TypeInfo);
	if (Replacement == nullptr)
	{
		check(TypeInfo->module->ReloadNewModule == nullptr);
		return;
	}
	if (bHoldsRefCount)
	{
		Replacement->AddRefInternal();
		if (TypeInfo != nullptr)
			TypeInfo->ReleaseInternal();
	}
	TypeInfo = Replacement;
}

static void UpdateReferenceInDataType(const asModuleReferenceUpdateMap& UpdateMap, asCDataType& DataType, bool bHoldsRefCount)
{
	if (asCTypeInfo* TypeInfo = DataType.GetTypeInfo())
	{
		if (TypeInfo->module == nullptr)
			return;
		asCTypeInfo* Replacement = UpdateMap.Types.FindRef(TypeInfo);
		if (Replacement == nullptr)
		{
			check(TypeInfo->module->ReloadNewModule == nullptr);
			return;
		}

		if (bHoldsRefCount)
		{
			Replacement->AddRefInternal();
			if (TypeInfo != nullptr)
				TypeInfo->ReleaseInternal();
		}

		DataType.SetTypeInfo(Replacement);
	}
}

static void UpdateReferenceInFunctionPointer(const asModuleReferenceUpdateMap& UpdateMap, asCScriptFunction*& Function, bool bHoldsRefCount)
{
	if (Function == nullptr)
		return;
	if (Function->module == nullptr)
	{
		if (Function->objectType == nullptr)
			return;
		if ((Function->objectType->flags & asOBJ_TEMPLATE) == 0)
			return;
	}

	asCScriptFunction* Replacement = UpdateMap.Functions.FindRef(Function);
	if (Replacement == nullptr)
	{
		checkSlow(Function->module == nullptr || Function->module->ReloadNewModule == nullptr);
		return;
	}

	if (bHoldsRefCount)
	{
		Replacement->AddRefInternal();
		if (Function != nullptr)
			Function->ReleaseInternal();
	}

	Function = Replacement;
}

void asCModule::UpdateReferencesInReflectionDataOnly(const asModuleReferenceUpdateMap& UpdateMap)
{
	auto UpdateTypeInfo = [&](asCTypeInfo*& TypeInfo, bool bHoldsRefCount)
	{
		UpdateReferenceInTypeInfo(UpdateMap, TypeInfo, bHoldsRefCount);
	};

	auto UpdateDataType = [&](asCDataType& DataType, bool bHoldsRefCount)
	{
		UpdateReferenceInDataType(UpdateMap, DataType, bHoldsRefCount);
	};

	auto UpdateModule = [&](asCModule*& Module)
	{
		asCModule* Replacement = UpdateMap.Modules.FindRef(Module);
		if (Replacement == nullptr)
		{
			check(Module->ReloadNewModule == nullptr);
			return;
		}

		Module = Replacement;
	};

	auto UpdateFunction = [&](asCScriptFunction*& Function, bool bHoldsRefCount)
	{
		UpdateReferenceInFunctionPointer(UpdateMap, Function, bHoldsRefCount);
	};

	auto UpdateFunctionId = [&](int FunctionId, bool bHoldsRefCount)
	{
		asCScriptFunction* Function = engine->scriptFunctions[FunctionId];
		if (Function == nullptr)
			return;
		if (Function->module == nullptr)
		{
			if (Function->objectType == nullptr)
				return;
			if ((Function->objectType->flags & asOBJ_TEMPLATE) == 0)
				return;
		}

		asCScriptFunction* Replacement = UpdateMap.Functions.FindRef(Function);
		if (Replacement == nullptr)
		{
			checkSlow(Function->module == nullptr || Function->module->ReloadNewModule == nullptr);
			return;
		}

		if (bHoldsRefCount)
		{
			Replacement->AddRefInternal();
			if (Function != nullptr)
				Function->ReleaseInternal();
		}

		Function = Replacement;
	};

	auto UpdateFunctionSignature = [&](asCScriptFunction* Function)
	{
		bool bHoldingReferences = (Function->scriptData != nullptr && Function->scriptData->byteCode.GetLength() != 0);
		UpdateDataType(Function->returnType, bHoldingReferences);
		for (int i = 0, Count = Function->parameterTypes.GetLength(); i < Count; ++i)
			UpdateDataType(Function->parameterTypes[i], bHoldingReferences);
	};

	for (int ClassIndex = 0, ClassCount = classTypes.GetLength(); ClassIndex < ClassCount; ++ClassIndex)
	{
		asCObjectType* Class = classTypes[ClassIndex];
		if (Class->derivedFrom != nullptr)
			UpdateTypeInfo((asCTypeInfo*&)Class->derivedFrom, true);

		for (int FunctionIndex = 0, FunctionCount = Class->methods.GetLength(); FunctionIndex < FunctionCount; ++FunctionIndex)
		{
			UpdateFunctionId(Class->methods[FunctionIndex], true);

			asCScriptFunction* Function = engine->scriptFunctions[Class->methods[FunctionIndex]];
			if (Function->objectType == Class)
				UpdateFunctionSignature(Function);
		}

		// Update constructors
		for (int i = 0, Count = Class->beh.constructors.GetLength(); i < Count; ++i)
		{
			asCScriptFunction* Function = engine->scriptFunctions[Class->beh.constructors[i]];
			UpdateFunctionSignature(Function);
		}

		// Update factories
		for (int i = 0, Count = Class->beh.factories.GetLength(); i < Count; ++i)
		{
			asCScriptFunction* Function = engine->scriptFunctions[Class->beh.factories[i]];
			UpdateFunctionSignature(Function);
		}

		// Update destructor
		if (Class->beh.destruct != 0)
		{
			asCScriptFunction* Function = engine->scriptFunctions[Class->beh.destruct];
			UpdateFunctionSignature(Function);
		}
		
		UpdateFunctionId(Class->beh.construct, true);
		UpdateFunctionId(Class->beh.copyconstruct, true);
		UpdateFunctionId(Class->beh.destruct, true);
		UpdateFunctionId(Class->beh.copy, true);
		UpdateFunctionId(Class->beh.factory, true);
		UpdateFunctionId(Class->beh.copyfactory, true);

		// Local properties should replace their type
		for (int PropertyIndex = 0, PropertyCount = Class->localProperties.GetLength(); PropertyIndex < PropertyCount; ++PropertyIndex)
		{
			asCObjectProperty* Property = Class->localProperties[PropertyIndex];
			UpdateDataType(Property->type, true);
		}
	}

	// Update global functions
	for (int FunctionIndex = 0, FunctionCount = globalFunctionList.GetLength(); FunctionIndex < FunctionCount; ++FunctionIndex)
	{
		asCScriptFunction* Function = globalFunctionList[FunctionIndex];
		if (Function == nullptr)
			continue;

		UpdateFunctionSignature(Function);
	}

	// Update global properties
	for (int GlobalIndex = 0, GlobalCount = scriptGlobalsList.GetLength(); GlobalIndex < GlobalCount; ++GlobalIndex)
	{
		asCGlobalProperty* Global = scriptGlobalsList[GlobalIndex];
		if (Global == nullptr)
			continue;

		UpdateDataType(Global->type, false);
	}

	// Global properties that are currently being compiled need to be updated too
	if (builder != nullptr)
	{
		builder->globVariables.IterateAll([&](sGlobalVariableDescription* gvar)
		{
			UpdateDataType(gvar->datatype, false);
		});
	}

	// Update module references
	auto OldDependencies = MoveTemp(moduleDependencies);
	moduleDependencies.Reset();
	moduleDependencies.Reserve(OldDependencies.Num());

	for (const auto& DependencyElem : OldDependencies)
	{
		asCModule* DependencyModule = DependencyElem.Key;
		UpdateModule(DependencyModule);

		moduleDependencies.Add(DependencyModule, DependencyElem.Value);
	}

	for (int i = 0, Count = importedModules.GetLength(); i < Count; ++i)
		UpdateModule(importedModules[i]);
}

void asCModule::UpdateReferencesInScriptBytecode(const asModuleReferenceUpdateMap& UpdateMap)
{
	auto UpdateTypeId = [&](int& TypeId)
	{
		if (TypeId == 0)
			return;
		if (TypeId <= asTYPEID_LAST_PRIMITIVE)
			return;

		asCTypeInfo* TypeInfo = (asCTypeInfo*)engine->GetTypeInfoById(TypeId);
		asDWORD Flags = TypeId & ~(asTYPEID_MASK_SEQNBR | asTYPEID_MASK_OBJECT);

		UpdateReferenceInTypeInfo(UpdateMap, TypeInfo, false);
		TypeId = TypeInfo->GetTypeId() | Flags;
	};

	auto UpdateFunctionId = [&](int& FunctionId, bool bHoldsReference)
	{
		asCScriptFunction* Function = engine->scriptFunctions[FunctionId];
		UpdateReferenceInFunctionPointer(UpdateMap, Function, bHoldsReference);
		FunctionId = Function->GetId();
	};

	auto UpdateGlobalVariablePointer = [&](void*& GlobalPointer)
	{
		void* Replacement = UpdateMap.GlobalVariablePointers.FindRef(GlobalPointer);
		if (Replacement != nullptr)
			GlobalPointer = Replacement;
	};

	for (int FunctionIndex = 0, FunctionCount = scriptFunctions.GetLength(); FunctionIndex < FunctionCount; ++FunctionIndex)
	{
		asCScriptFunction* Function = scriptFunctions[FunctionIndex];
		if (Function == nullptr)
			continue;

		asCScriptFunction::ScriptFunctionData* scriptData = Function->scriptData;
		if (scriptData == nullptr)
			continue;

		for (int n = 0, Count = scriptData->objVariableTypes.GetLength(); n < Count; ++n)
		{
			UpdateReferenceInTypeInfo(UpdateMap, scriptData->objVariableTypes[n], true);
		}

		for (int n = 0, Count = scriptData->variables.GetLength(); n < Count; ++n)
		{
			UpdateReferenceInDataType(UpdateMap, scriptData->variables[n]->type, false);
		}

		// Go through the byte code and replace references to all resources used by the function
		asCArray<asDWORD>& bc = scriptData->byteCode;
		for( asUINT n = 0; n < bc.GetLength(); n += asBCTypeSize[asBCInfo[*(asBYTE*)&bc[n]].type] )
		{
			switch( *(asBYTE*)&bc[n] )
			{
			case asBC_TYPEID:
			case asBC_Cast:
				UpdateTypeId((int&)asBC_DWORDARG(&bc[n]));
				break;

			case asBC_ADDSi:
			case asBC_LoadThisR:
			case asBC_LoadRObjR:
			case asBC_LoadVObjR:
			{
				// For ADDSi, we should also make sure we update the offset being used if the type has changed,
				// because the property might be somewhere else now!
				int& TypeId = (int&)asBC_INTARG(&bc[n]);
				short& Offset = (short&)asBC_SWORDARG0(&bc[n]);

				asCTypeInfo* OldTypeInfo = (asCTypeInfo*)engine->GetTypeInfoById(TypeId);
				if (OldTypeInfo != nullptr && OldTypeInfo->module != nullptr)
				{
					asDWORD Flags = TypeId & ~(asTYPEID_MASK_SEQNBR | asTYPEID_MASK_OBJECT);
					asCTypeInfo* NewTypeInfo = UpdateMap.Types.FindRef(OldTypeInfo);

					if (NewTypeInfo != OldTypeInfo)
					{
						asCObjectProperty* NewProperty = UpdateMap.NewObjectPropertiesByOldTypeIdAndOldOffset.FindRef(
							TPair<int,int>(OldTypeInfo->GetTypeId(), Offset)
						);
						if (NewProperty != nullptr)
						{
							Offset = NewProperty->byteOffset;
							TypeId = NewTypeInfo->GetTypeId() | Flags;
						}
					}
				}
			}
			break;

			// Object types
			case asBC_OBJTYPE:
			case asBC_FREE:
				UpdateReferenceInTypeInfo(UpdateMap, (asCTypeInfo*&)asBC_PTRARG(&bc[n]), true);
			break;

			case asBC_CopyScript:
			case asBC_FinConstruct:
				UpdateReferenceInTypeInfo(UpdateMap, (asCTypeInfo*&)asBC_PTRARG(&bc[n]), false);
			break;

			// Object type and function
			case asBC_ALLOC:
			{
				UpdateReferenceInTypeInfo(UpdateMap, (asCTypeInfo*&)asBC_PTRARG(&bc[n]), true);

				int& funcId = (int&)asBC_INTARG(&bc[n]+AS_PTR_SIZE);
				UpdateFunctionId(funcId, true);
			}
			break;

			// Global variables
			case asBC_PGA:
			case asBC_PshGPtr:
			case asBC_LDG:
			case asBC_PshG4:
			case asBC_LdGRdR4:
			case asBC_CpyGtoV4:
			case asBC_CpyVtoG4:
			case asBC_SetG4:
			{
				void*& gvarPtr = (void*&)asBC_PTRARG(&bc[n]);
				UpdateGlobalVariablePointer(gvarPtr);
			}
			break;

			// System functions
			case asBC_CALLSYS:
			case asBC_FuncPtr:
				UpdateReferenceInFunctionPointer(UpdateMap, (asCScriptFunction*&)asBC_PTRARG(&bc[n]), true);
			break;

			case asBC_Thiscall1:
				UpdateReferenceInFunctionPointer(UpdateMap, (asCScriptFunction*&)asBC_PTRARG(&bc[n]), false);
			break;

			// Functions
			case asBC_CALL:
			case asBC_CALLINTF:
			{
				int& funcId = (int&)asBC_INTARG(&bc[n]);
				UpdateFunctionId(funcId, true);
			}
			break;
			}
		}
	}
}

void asModuleReferenceUpdateMap::BuildReverseMap(asModuleReferenceUpdateMap& OutReverseMap)
{
	TMap<int, int> OldToNewTypeId;
	OldToNewTypeId.Reserve(Types.Num());

	for (auto Elem : Modules)
	{
		OutReverseMap.Modules.Add(Elem.Value, Elem.Key);
	}

	for (auto Elem : Types)
	{
		OldToNewTypeId.Add(Elem.Key->GetTypeId(), Elem.Value->GetTypeId());
		OutReverseMap.Types.Add(Elem.Value, Elem.Key);
	}

	for (auto Elem : Functions)
	{
		OutReverseMap.Functions.Add(Elem.Value, Elem.Key);
	}

	for (auto Elem : ObjectProperties)
	{
		OutReverseMap.ObjectProperties.Add(Elem.Value, Elem.Key);
	}

	for (auto Elem : GlobalProperties)
	{
		OutReverseMap.GlobalProperties.Add(Elem.Value, Elem.Key);
	}

	for (auto Elem : GlobalVariablePointers)
	{
		OutReverseMap.GlobalVariablePointers.Add(Elem.Value, Elem.Key);
	}

	for (auto Elem : NewObjectPropertiesByOldTypeIdAndOldOffset)
	{
		OutReverseMap.NewObjectPropertiesByOldTypeIdAndOldOffset.Add(
			TPair<int, int>(
				OldToNewTypeId.FindChecked(Elem.Key.Key),
				Elem.Value->byteOffset
			),
			OutReverseMap.ObjectProperties.FindChecked(Elem.Value)
		);
	}
}

END_AS_NAMESPACE
