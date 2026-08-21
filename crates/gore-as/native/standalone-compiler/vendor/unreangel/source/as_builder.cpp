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
// as_builder.cpp
//
// This is the class that manages the compilation of the scripts
//


#include "as_builder.h"
#include "as_config.h"
#include "as_parser.h"
#include "as_compiler.h"
#include "as_tokendef.h"
#include "as_string_util.h"
#include "as_outputbuffer.h"
#include "as_texts.h"
#include "as_scriptobject.h"
#include "as_debug.h"
#include <functional>
#include "AngelscriptManager.h"
#include "AngelscriptSettings.h"

BEGIN_AS_NAMESPACE

#ifndef AS_NO_COMPILER

// asCSymbolTable template specializations for sGlobalVariableDescription entries
template<>
void asCSymbolTable<sGlobalVariableDescription>::GetKey(const sGlobalVariableDescription *entry, asSNameSpaceNamePair &key) const
{
	asSNameSpace *ns = entry->nameSpace;
	asCString name = entry->name;
	key = asSNameSpaceNamePair(ns, name);
}

// Comparator for exact variable search
class asCCompGlobVarType : public asIFilter
{
public:
	const asCDataType &m_type;
	asCCompGlobVarType(const asCDataType &type) : m_type(type) {}

	bool operator()(const void *p) const
	{
		const sGlobalVariableDescription* desc = reinterpret_cast<const sGlobalVariableDescription*>(p);
		return desc->datatype == m_type;
	}

private:
	// The assignment operator is required for MSVC9, otherwise it will complain that it is not possible to auto generate the operator
	asCCompGlobVarType &operator=(const asCCompGlobVarType &) {return *this;}
};

#endif

asCBuilder::asCBuilder(asCScriptEngine *_engine, asCModule *_module)
{
	this->engine = _engine;
	this->module = _module;
	silent = false;
}

asCBuilder::~asCBuilder()
{
#ifndef AS_NO_COMPILER
	asUINT n;

	// Free all functions
	for( n = 0; n < functions.GetLength(); n++ )
	{
		if( functions[n] )
		{
			asDELETE(functions[n],sFunctionDescription);
		}

		functions[n] = 0;
	}

	// Free all global variables
	globVariables.IterateAll([&](sGlobalVariableDescription* globVar)
	{
		asDELETE(globVar,sGlobalVariableDescription);
	});
	globVariables.EraseAll();
	globVariableList.SetLength(0);

	// Free all the loaded files
	for( n = 0; n < scripts.GetLength(); n++ )
	{
		if( scripts[n] )
			asDELETE(scripts[n],asCScriptCode);

		scripts[n] = 0;
	}

	// Free all class declarations
	for( n = 0; n < classDeclarations.GetLength(); n++ )
	{
		if( classDeclarations[n] )
		{
			asDELETE(classDeclarations[n],sClassDeclaration);
			classDeclarations[n] = 0;
		}
	}

	for( n = 0; n < interfaceDeclarations.GetLength(); n++ )
	{
		if( interfaceDeclarations[n] )
		{
			asDELETE(interfaceDeclarations[n],sClassDeclaration);
			interfaceDeclarations[n] = 0;
		}
	}

	for( n = 0; n < namedTypeDeclarations.GetLength(); n++ )
	{
		if( namedTypeDeclarations[n] )
		{
			asDELETE(namedTypeDeclarations[n],sClassDeclaration);
			namedTypeDeclarations[n] = 0;
		}
	}

	for( n = 0; n < funcDefs.GetLength(); n++ )
	{
		if( funcDefs[n] )
		{
			asDELETE(funcDefs[n],sFuncDef);
			funcDefs[n] = 0;
		}
	}

	for( n = 0; n < mixinClasses.GetLength(); n++ )
	{
		if( mixinClasses[n] )
		{
			asDELETE(mixinClasses[n],sMixinClass);
			mixinClasses[n] = 0;
		}
	}

#endif // AS_NO_COMPILER
}

void asCBuilder::Reset()
{
	scriptsParsed = false;
	numErrors = 0;
	numWarnings = 0;
	engine->preMessage.isSet = false;
}

#ifndef AS_NO_COMPILER
int asCBuilder::AddCode(const char *name, const char *code, int codeLength, int lineOffset, int sectionIdx, bool makeCopy)
{
	asCScriptCode *script = asNEW(asCScriptCode);
	if( script == 0 )
		return asOUT_OF_MEMORY;

	int r = script->SetCode(name, code, codeLength, makeCopy);
	if( r < 0 )
	{
		asDELETE(script, asCScriptCode);
		return r;
	}

	script->lineOffset = lineOffset;
	script->idx = sectionIdx;
	scripts.PushLast(script);

	return 0;
}

void asCBuilder::EvaluateTemplateInstances(bool keepSilent)
{
	// Backup the original message stream
	bool                       msgCallback     = engine->msgCallback;
	asSSystemFunctionInterface msgCallbackFunc = engine->msgCallbackFunc;
	void                      *msgCallbackObj  = engine->msgCallbackObj;

	// Set the new temporary message stream
	asCOutputBuffer outBuffer;
	if( keepSilent )
		engine->SetMessageCallback(asMETHOD(asCOutputBuffer, Callback), &outBuffer, asCALL_THISCALL);

	// Evaluate each of the template instances that have been created since the start of the build
	// TODO: This is not exactly correct, since another thread may have created template instances in parallel
	TArray<asCObjectType*> TypesToValidate = MoveTemp(engine->unvalidatedTemplateInstances);
	engine->unvalidatedTemplateInstances.Reset();

	for(asCObjectType* tmpl : TypesToValidate)
	{
		asCScriptFunction *callback = engine->scriptFunctions[tmpl->beh.templateCallback];
		asCString ErrorMessage;
		if( callback && !engine->CallGlobalFunctionRetBool(tmpl, &ErrorMessage, callback->sysFuncIntf, callback) )
		{
			asCString sub = tmpl->templateSubTypes[0].Format(engine->nameSpaces[0]);
			for( asUINT m = 1; m < tmpl->templateSubTypes.GetLength(); m++ )
			{
				sub += ",";
				sub += tmpl->templateSubTypes[m].Format(engine->nameSpaces[0]);
			}
			asCString str;
			str.Format(TXT_INSTANCING_INVLD_TMPL_TYPE_s_s, tmpl->name.AddressOf(), sub.AddressOf());
			if (ErrorMessage.GetLength() != 0)
			{
				str += ": ";
				str += ErrorMessage;
			}
			WriteError(tmpl->scriptSectionIdx >= 0 ? engine->scriptSectionNames[tmpl->scriptSectionIdx]->AddressOf() : "", str, tmpl->declaredAt&0xFFFFF, (tmpl->declaredAt>>20)&0xFFF);
			tmpl->isInvalidGeneratedType = true;
			engine->DiscardTemplateInstance(tmpl);
		}
		else
		{
			// If the callback said this template instance won't be garbage collected then remove the flag
			tmpl->flags &= ~asOBJ_GC;
		}

		if ((tmpl->flags & asOBJ_TEMPLATE_SUBTYPE_DETERMINES_SIZE) != 0)
			tmpl->CalculateTemplateSize();
	}

	// Restore message callback
	if( keepSilent )
	{
		engine->msgCallback     = msgCallback;
		engine->msgCallbackFunc = msgCallbackFunc;
		engine->msgCallbackObj  = msgCallbackObj;
	}
}

int asCBuilder::BuildParallelParseScripts()
{
	Reset();

	// Parse all the files as if they were one
	asUINT n = 0;
	for( n = 0; n < scripts.GetLength(); n++ )
	{
		asCParser *parser = asNEW(asCParser)(this);
		if( parser != 0 )
		{
			parsers.PushLast(parser);

			// Parse the script file
			parser->ParseScript(scripts[n]);
		}
	}

	if( numErrors > 0 )
		return asERROR;
	return asSUCCESS;
}

int asCBuilder::BuildGenerateTypes()
{
	asUINT n = 0;

	// Find all type declarations
	if (numErrors == 0)
	{
		for (n = 0; n < scripts.GetLength(); n++)
		{
			asCScriptNode *node = parsers[n]->GetScriptNode();
			RegisterTypesFromScript(node, scripts[n], engine->nameSpaces[0]);
		}
	}

	if( numErrors > 0 )
		return asERROR;
	return asSUCCESS;
}

int asCBuilder::BuildGenerateFunctions()
{
	ParseScripts();

	// Compile the types first
	CompileInterfaces();
	CompileClasses(0);

	RegisterGlobalVariables();

	if( numErrors > 0 )
		return asERROR;

	return asSUCCESS;
}

int asCBuilder::BuildLayoutClasses()
{
	for (asUINT n = 0; n < classDeclarations.GetLength(); n++)
		LayoutClass(classDeclarations[n]);

	CreateDefaultDestructors();

	if( numErrors > 0 )
		return asERROR;

	return asSUCCESS;
}

int asCBuilder::BuildLayoutFunctions()
{
	for( asUINT n = 0; n < functions.GetLength(); n++ )
	{
		sFunctionDescription *current = functions[n];
		if (current != nullptr)
			LayoutFunction(current);
	}

	for( asUINT n = 0; n < factories.GetLength(); n++ )
	{
		const sFactoryDescription& factoryDesc = factories[n];
		asCScriptFunction* func = engine->scriptFunctions[factoryDesc.funcId];
		if (func != nullptr)
			func->CalculateParameterOffsets();
	}

	// Then the global variables. Here the variables declared with auto
	// will be resolved, so they can be accessed properly in the functions
	CompileGlobalVariables();

	if( numErrors > 0 )
		return asERROR;

	return asSUCCESS;
}

int asCBuilder::BuildCompileCode()
{
	// Compile all the factories we've been putting off compiling
	for( asUINT n = 0; n < factories.GetLength(); n++ )
	{
		const sFactoryDescription& factoryDesc = factories[n];
		asCScriptFunction* func = engine->scriptFunctions[factoryDesc.funcId];
		if (func != nullptr)
		{
			asCCompiler compiler(this);
			compiler.CompileFactory(this, factoryDesc.script, func);
		}
	}

	// Finally the global functions and class methods
	CompileFunctions();

	// Remove class declarations from classes
	for( asUINT n = 0; n < classDeclarations.GetLength(); n++ )
	{
		if (classDeclarations[n]->typeInfo != nullptr)
		{
			asCObjectType *ot = CastToObjectType(classDeclarations[n]->typeInfo);
			ot->compilingDeclaration = nullptr;
		}
	}

	// Clean up parsers
	for(asUINT n = 0; n < parsers.GetLength(); n++ )
	{
		asDELETE(parsers[n],asCParser);
	}
	parsers.SetLength(0);

	// TODO: Attempt to reorder the initialization of global variables so that
	//       they do not access other uninitialized global variables out-of-order
	//       The builder needs to check for each of the global variable, what functions
	//       that are accessed, and what global variables are access by these functions.

	if( numWarnings > 0 && engine->ep.compilerWarnings == 2 )
		WriteError(TXT_WARNINGS_TREATED_AS_ERROR, 0, 0);

	if( numErrors > 0 )
		return asERROR;

	// Make sure something was compiled, otherwise return an error
	//if( module->IsEmpty() )
	//{
	//	WriteError(TXT_NOTHING_WAS_BUILT, 0, 0);
	//	return asERROR;
	//}

	return asSUCCESS;
}
#endif

int asCBuilder::ValidateDefaultArgs(asCScriptCode *script, asCScriptNode *node, asCScriptFunction *func)
{
	int firstArgWithDefaultValue = -1;
	for( asUINT n = 0; n < func->defaultArgs.GetLength(); n++ )
	{
		if( func->defaultArgs[n] )
			firstArgWithDefaultValue = n;
		else if( firstArgWithDefaultValue >= 0 )
		{
			asCString str;
			str.Format(TXT_DEF_ARG_MISSING_IN_FUNC_s, func->GetDeclaration(true, false, false, true));
			WriteError(str, script, node);
			return asINVALID_DECLARATION;
		}
	}

	return 0;
}

#ifndef AS_NO_COMPILER
// This function will verify if the newly created function will conflict another overload due to having
// identical function arguments that are not default args, e.g: foo(int) and foo(int, int=0)
int asCBuilder::CheckForConflictsDueToDefaultArgs(asCScriptCode *script, asCScriptNode *node, asCScriptFunction *func, asCObjectType *objType)
{
	// TODO: Implement for global functions too
	if( func->objectType == 0 || objType == 0 ) return 0;

	asCArray<int> funcs;
	GetObjectMethodDescriptions(func->name.AddressOf(), objType, funcs, false);
	for( asUINT n = 0; n < funcs.GetLength(); n++ )
	{
		asCScriptFunction *func2 = engine->scriptFunctions[funcs[n]];
		if( func == func2 )
			continue;

		if( func->IsReadOnly() != func2->IsReadOnly() )
			continue;

		bool match = true;
		asUINT p = 0;
		for( ; p < func->parameterTypes.GetLength() && p < func2->parameterTypes.GetLength(); p++ )
		{
			// Only verify until the first argument with default args
			if( (func->defaultArgs.GetLength() > p && func->defaultArgs[p]) ||
				(func2->defaultArgs.GetLength() > p && func2->defaultArgs[p]) )
				break;

			if( func->parameterTypes[p] != func2->parameterTypes[p] ||
				func->inOutFlags[p] != func2->inOutFlags[p] )
			{
				match = false;
				break;
			}
		}

		if( match )
		{
			if( !((p >= func->parameterTypes.GetLength() && p < func2->defaultArgs.GetLength() && func2->defaultArgs[p]) ||
				  (p >= func2->parameterTypes.GetLength() && p < func->defaultArgs.GetLength() && func->defaultArgs[p])) )
			{
				// The argument lists match for the full length of the shorter, but the next
				// argument on the longer does not have a default arg so there is no conflict
				match = false;
			}
		}

		if( match )
		{
			WriteWarning(TXT_OVERLOAD_CONFLICTS_DUE_TO_DEFAULT_ARGS, script, node);
			WriteInfo(func->GetDeclaration(true, false, false, true), script, node);
			WriteInfo(func2->GetDeclaration(true, false, false, true), script, node);
			break;
		}
	}

	return 0;
}

int asCBuilder::CompileFunction(const char *sectionName, const char *code, int lineOffset, asDWORD compileFlags, asCScriptFunction **outFunc)
{
	asASSERT(outFunc != 0);

	Reset();

	// Add the string to the script code
	asCScriptCode *script = asNEW(asCScriptCode);
	if( script == 0 )
		return asOUT_OF_MEMORY;

	script->SetCode(sectionName, code, true);
	script->lineOffset = lineOffset;
	script->idx = engine->GetScriptSectionNameIndex(sectionName ? sectionName : "");
	scripts.PushLast(script);

	// Parse the string
	asCParser parser(this);
	if( parser.ParseScript(scripts[0]) < 0 )
		return asERROR;

	asCScriptNode *node = parser.GetScriptNode();

	// Make sure there is nothing else than the function in the script code
	if( node == 0 ||
		node->firstChild == 0 ||
		node->firstChild != node->lastChild ||
		node->firstChild->nodeType != snFunction )
	{
		WriteError(TXT_ONLY_ONE_FUNCTION_ALLOWED, script, 0);
		return asERROR;
	}

	// Find the function node
	node = node->firstChild;

	// Create the function
	asSFunctionTraits funcTraits;
	asCScriptFunction *func = asNEW(asCScriptFunction)(engine, compileFlags & asCOMP_ADD_TO_MODULE ? module : 0, asFUNC_SCRIPT);
	if( func == 0 )
		return asOUT_OF_MEMORY;

	GetParsedFunctionDetails(node, scripts[0], 0, func->name, func->returnType, func->parameterNames, func->parameterTypes, func->inOutFlags, func->defaultArgs, funcTraits, module->defaultNamespace);
	func->id                           = engine->GetNextScriptFunctionId();
	func->scriptData->scriptSectionIdx = engine->GetScriptSectionNameIndex(sectionName ? sectionName : "");
	int row, col;
	scripts[0]->ConvertPosToRowCol(node->tokenPos, &row, &col);
	func->scriptData->declaredAt       = (row & 0xFFFFF)|((col & 0xFFF)<<20);
	func->nameSpace                    = module->defaultNamespace;

	// Make sure the default args are declared correctly
	int r = ValidateDefaultArgs(script, node, func);
	if( r < 0 )
	{
		func->ReleaseInternal();
		return asERROR;
	}

	// Tell the engine that the function exists already so the compiler can access it
	if( compileFlags & asCOMP_ADD_TO_MODULE )
	{
		r = CheckNameConflict(func->name.AddressOf(), node, scripts[0], module->defaultNamespace);
		if( r < 0 )
		{
			func->ReleaseInternal();
			return asERROR;
		}

		module->globalFunctions.Add(func);
		module->globalFunctionList.PushLast(func);
		module->AddScriptFunction(func);
	}
	else
		engine->AddScriptFunction(func);

	// Fill in the function info for the builder too
	node->DisconnectParent();
	sFunctionDescription *funcDesc = asNEW(sFunctionDescription);
	if( funcDesc == 0 )
	{
		func->ReleaseInternal();
		return asOUT_OF_MEMORY;
	}

	functions.PushLast(funcDesc);
	funcDesc->script            = scripts[0];
	funcDesc->node              = node;
	funcDesc->name              = func->name;
	funcDesc->funcId            = func->id;
	funcDesc->paramNames        = func->parameterNames;
	funcDesc->isExistingShared  = false;

	// This must be done in a loop, as it is possible that additional functions get declared as lambda's in the code
	for( asUINT n = 0; n < functions.GetLength(); n++ )
	{
		asCCompiler compiler(this);
		asCScriptFunction *f = engine->scriptFunctions[functions[n]->funcId];
		r = compiler.CompileFunction(this, functions[n]->script, f->parameterNames, functions[n]->node, f, 0);
		if( r < 0 )
			break;
	}

	if( numWarnings > 0 && engine->ep.compilerWarnings == 2 )
		WriteError(TXT_WARNINGS_TREATED_AS_ERROR, 0, 0);

	// None of the functions should be added to the module if any error occurred,
	// or it was requested that the functions wouldn't be added to the scope
	if( !(compileFlags & asCOMP_ADD_TO_MODULE) || numErrors > 0 )
	{
		for( asUINT n = 0; n < functions.GetLength(); n++ )
		{
			asCScriptFunction *f = engine->scriptFunctions[functions[n]->funcId];
			if( module->globalFunctions.Contains(f) )
			{
				module->globalFunctions.Remove(f);
				module->globalFunctionList.RemoveValue(f);
				module->scriptFunctions.RemoveValue(f);
				f->ReleaseInternal();
			}
		}
	}

	if( numErrors > 0 )
	{
		// Release the function pointer that would otherwise be returned if no errors occured
		func->ReleaseInternal();

		return asERROR;
	}

	// Return the function
	*outFunc = func;

	return asSUCCESS;
}

void asCBuilder::ParseScripts()
{
	if (scriptsParsed)
		return;

	TimeIt("asCBuilder::ParseScripts");

	scriptsParsed = true;
	if (numErrors == 0)
	{
		// Find all type declarations
		asUINT n = 0;

		// Before moving forward the builder must establish the relationship between types
		// so that a derived type can see the child types of the parent type.
		DetermineTypeRelations();

		// Complete function definitions (defining returntype and parameters)
		for( n = 0; n < funcDefs.GetLength(); n++ )
			CompleteFuncDef(funcDefs[n]);

		// Register script methods found in the interfaces
		for( n = 0; n < interfaceDeclarations.GetLength(); n++ )
		{
			sClassDeclaration *decl = interfaceDeclarations[n];
			asCScriptNode *node = decl->node->firstChild->next;

			// Skip list of inherited interfaces
			while( node && node->nodeType == snIdentifier )
				node = node->next;

			while( node )
			{
				asCScriptNode *next = node->next;
				if( node->nodeType == snFunction )
				{
					node->DisconnectParent();
					RegisterScriptFunctionFromNode(node, decl->script, CastToObjectType(decl->typeInfo), true, false, 0, decl->isExistingShared);
				}
				else if( node->nodeType == snVirtualProperty )
				{
					node->DisconnectParent();
					RegisterVirtualProperty(node, decl->script, CastToObjectType(decl->typeInfo), true, false, 0, decl->isExistingShared);
				}

				node = next;
			}
		}

		// Register script methods found in the classes
		for( n = 0; n < classDeclarations.GetLength(); n++ )
		{
			sClassDeclaration *decl = classDeclarations[n];

			asCScriptNode *node = decl->node->firstChild->next;

			// Skip list of classes and interfaces
			while( node && node->nodeType == snIdentifier )
				node = node->next;

			while( node )
			{
				asCScriptNode *next = node->next;
				if( node->nodeType == snFunction )
				{
					node->DisconnectParent();
					RegisterScriptFunctionFromNode(node, decl->script, CastToObjectType(decl->typeInfo), false, false, 0, decl->isExistingShared);
				}
				else if( node->nodeType == snVirtualProperty )
				{
					node->DisconnectParent();
					RegisterVirtualProperty(node, decl->script, CastToObjectType(decl->typeInfo), false, false, 0, decl->isExistingShared);
				}
				else if( node->nodeType == snAccessDeclaration )
				{
					node->DisconnectParent();
					RegisterAccessSpecifier(node, decl->script, CastToObjectType(decl->typeInfo));
				}
				else if( node->nodeType == snClassDefaultStatement )
				{
					node->DisconnectParent();
					decl->defaultStatements.PushLast(node);
				}

				node = next;
			}

			// Make sure the default factory & constructor exists for classes
			asCObjectType *ot = CastToObjectType(decl->typeInfo);
			if( ot->beh.construct == engine->scriptTypeBehaviours.beh.construct )
			{
				if( ot->beh.constructors.GetLength() == 1 || engine->ep.alwaysImplDefaultConstruct )
				{
					AddDefaultConstructor(ot, decl->script);
				}
				else
				{
					// As the class has another constructor we shouldn't provide the default constructor
					if( ot->beh.construct )
					{
						engine->scriptFunctions[ot->beh.construct]->ReleaseInternal();
						ot->beh.construct = 0;
						ot->beh.constructors.RemoveIndex(0);
					}
					if( ot->beh.factory )
					{
						engine->scriptFunctions[ot->beh.factory]->ReleaseInternal();
						ot->beh.factory = 0;
						ot->beh.factories.RemoveIndex(0);
					}
					// Only remove the opAssign method if the script hasn't provided one
					if( ot->beh.copy == engine->scriptTypeBehaviours.beh.copy )
					{
						engine->scriptFunctions[ot->beh.copy]->ReleaseInternal();
						ot->beh.copy = 0;
					}
				}
			}

			// Add an initDefaults function if we have default statements
			if (decl->defaultStatements.GetLength() != 0)
				AddInitDefaultsFunction(ot, decl->script);
		}

		// Find other global nodes
		for( n = 0; n < scripts.GetLength(); n++ )
		{
			// Find other global nodes
			asCScriptNode *node = parsers[n]->GetScriptNode();
			RegisterNonTypesFromScript(node, scripts[n], engine->nameSpaces[0]);
		}
	}
}

void asCBuilder::RegisterTypesFromScript(asCScriptNode *node, asCScriptCode *script, asSNameSpace *ns)
{
	asASSERT(node->nodeType == snScript);

	// Find structure definitions first
	node = node->firstChild;
	while( node )
	{
		asCScriptNode *next = node->next;
		if( node->nodeType == snNamespace )
		{
			// Determine the name of the namespace
			asCString nsName;

			auto* nameNode = node->firstChild;
			while (nameNode && nameNode->nodeType == snIdentifier)
			{
				if (nsName.GetLength() != 0)
				{
					// Ensure that the parent namespace has also been created
					engine->AddNameSpace(nsName.AddressOf());
					nsName += "::";
				}

				nsName.Concatenate(&script->code[nameNode->tokenPos], nameNode->tokenLength);
				nameNode = nameNode->next;
			}

			if( ns->name != "" )
				nsName = ns->name + "::" + nsName;

			// Recursively register the entities defined in the namespace
			asSNameSpace *nsChild = engine->AddNameSpace(nsName.AddressOf());
			RegisterTypesFromScript(node->lastChild, script, nsChild);
		}
		else
		{
			if( node->nodeType == snClass )
			{
				node->DisconnectParent();
				RegisterClass(node, script, ns);
			}
			else if( node->nodeType == snInterface )
			{
				node->DisconnectParent();
				RegisterInterface(node, script, ns);
			}
			else if( node->nodeType == snEnum )
			{
				node->DisconnectParent();
				RegisterEnum(node, script, ns);
			}
			else if( node->nodeType == snTypedef )
			{
				node->DisconnectParent();
				RegisterTypedef(node, script, ns);
			}
			else if( node->nodeType == snFuncDef )
			{
				node->DisconnectParent();
				RegisterFuncDef(node, script, ns, 0);
			}
		}

		node = next;
	}
}

void asCBuilder::RegisterNonTypesFromScript(asCScriptNode *node, asCScriptCode *script, asSNameSpace *ns)
{
	node = node->firstChild;
	while( node )
	{
		asCScriptNode *next = node->next;
		if( node->nodeType == snNamespace )
		{
			// Determine the name of the namespace
			asCString nsName;

			auto* nameNode = node->firstChild;
			while (nameNode && nameNode->nodeType == snIdentifier)
			{
				if (nsName.GetLength() != 0)
					nsName += "::";
				nsName.Concatenate(&script->code[nameNode->tokenPos], nameNode->tokenLength);
				nameNode = nameNode->next;
			}

			if( ns->name != "" )
				nsName = ns->name + "::" + nsName;

			// Declare the namespace, then add the entities
			asSNameSpace *nsChild = engine->AddNameSpace(nsName.AddressOf());
			RegisterNonTypesFromScript(node->lastChild, script, nsChild);
		}
		else
		{
			node->DisconnectParent();
			if( node->nodeType == snFunction )
				RegisterScriptFunctionFromNode(node, script, 0, false, true, ns);
			else if( node->nodeType == snDeclaration )
				RegisterGlobalVar(node, script, ns);
			else if( node->nodeType == snVirtualProperty )
				RegisterVirtualProperty(node, script, 0, false, true, ns);
			else if( node->nodeType == snImport )
				RegisterImportedFunction(module->GetNextImportedFunctionId(), node, script, ns);
			else
			{
				// Unused script node
				int r, c;
				script->ConvertPosToRowCol(node->tokenPos, &r, &c);

				WriteWarning(script->name, TXT_UNUSED_SCRIPT_NODE, r, c);
			}
		}

		node = next;
	}
}

void asCBuilder::CreateDefaultDestructors()
{
	struct FCheckConstructor
	{
		static void ConditionalAddDefaultDestructor(asCBuilder* builder, asCObjectType* objType)
		{
			if (objType->beh.destruct != 0)
				return;
			if (objType->compilingDeclaration == nullptr)
				return;

			if (objType->compilingDeclaration->evaluatedDestructor)
				return;
			objType->compilingDeclaration->evaluatedDestructor = true;

			bool bNeedDestruct = false;
			if (objType->derivedFrom != nullptr)
			{
				if (objType->module != nullptr && objType->module->builder != nullptr)
					ConditionalAddDefaultDestructor(objType->derivedFrom->module->builder, (asCObjectType*)objType->derivedFrom);

				if (((asCObjectType*)objType->derivedFrom)->beh.destruct != 0)
					bNeedDestruct = true;
			}

			// Initialize each member in the order they were declared
			int ParentOffset = 0;
			if (objType->derivedFrom != nullptr)
				ParentOffset = objType->derivedFrom->size;
			else if (objType->shadowType != nullptr)
				ParentOffset = objType->basePropertyOffset;

			for( asUINT n = 0; n < objType->properties.GetLength(); n++ )
			{
				asCObjectProperty *prop = objType->properties[n];
				check(prop->byteOffset >= 0);

				if (prop->byteOffset < ParentOffset)
					continue;

				if (!prop->type.IsObject() || prop->type.IsReference() || prop->type.IsObjectHandle())
					continue;

				auto* typeInfo = (asCObjectType*)prop->type.GetTypeInfo();
				if (typeInfo->module != nullptr && typeInfo->module->builder != nullptr)
					ConditionalAddDefaultDestructor(typeInfo->module->builder, typeInfo);

				if( typeInfo->beh.destruct != 0 )
				{
					bNeedDestruct = true;
					break;
				}
			}

			if (bNeedDestruct)
			{
				builder->AddDefaultDestructor(objType, objType->compilingDeclaration->script);
			}
		}
	};

	for( asUINT n = 0; n < classDeclarations.GetLength(); n++ )
	{
		auto* classDecl = classDeclarations[n];
		FCheckConstructor::ConditionalAddDefaultDestructor(this, (asCObjectType*)classDecl->typeInfo);
	}
}

void asCBuilder::CompileFunctions()
{
	// Compile each function
	for( asUINT n = 0; n < functions.GetLength(); n++ )
	{
		sFunctionDescription *current = functions[n];
		if( current == 0 ) continue;

		// Don't compile the function again if it was an existing shared function
		if( current->isExistingShared ) continue;

		// Don't compile if there is no statement block
		if (current->node && !(current->node->nodeType == snStatementBlock || current->node->lastChild->nodeType == snStatementBlock))
			continue;

		asCCompiler compiler(this);
		asCScriptFunction *func = engine->scriptFunctions[current->funcId];

		// Find the class declaration for constructors
		sClassDeclaration *classDecl = 0;
		if( current->objType && (current->name == current->objType->name || current->name[0] == '~' || current->name == "__InitDefaults"))
		{
			for( asUINT c = 0; c < classDeclarations.GetLength(); c++ )
			{
				if( classDeclarations[c]->typeInfo == current->objType )
				{
					classDecl = classDeclarations[c];
					break;
				}
			}

			asASSERT( classDecl );
		}

		if( current->node )
		{
			int r, c;
			current->script->ConvertPosToRowCol(current->node->tokenPos, &r, &c);

			asCString str = func->GetDeclarationStr(true, false, false, true);
			str.Format(TXT_COMPILING_s, str.AddressOf());
			WriteInfo(current->script->name, str, r, c, true);

			// When compiling a constructor need to pass the class declaration for member initializations
			compiler.CompileFunction(this, current->script, current->paramNames, current->node, func, classDecl);

			engine->preMessage.isSet = false;
		}
		else if( current->objType && current->name == current->objType->name )
		{
			asCScriptNode *node = classDecl->node;

			int r = 0, c = 0;
			if( node )
				current->script->ConvertPosToRowCol(node->tokenPos, &r, &c);

			asCString str = func->GetDeclarationStr(true, false, false, true);
			str.Format(TXT_COMPILING_s, str.AddressOf());
			WriteInfo(current->script->name, str, r, c, true);

			// This is the default constructor that is generated
			// automatically if not implemented by the user.
			compiler.CompileDefaultConstructor(this, current->script, node, func, classDecl);

			engine->preMessage.isSet = false;
		}
		else if( current->objType && current->name[0] == '~' )
		{
			asCScriptNode *node = classDecl->node;

			int r = 0, c = 0;
			if( node )
				current->script->ConvertPosToRowCol(node->tokenPos, &r, &c);

			asCString str = func->GetDeclarationStr(true, false, false, true);
			str.Format(TXT_COMPILING_s, str.AddressOf());
			WriteInfo(current->script->name, str, r, c, true);

			// This is the default constructor that is generated
			// automatically if not implemented by the user.
			compiler.CompileDefaultDestructor(this, current->script, node, func, classDecl);

			engine->preMessage.isSet = false;
		}
		else if( current->objType && current->name == "__InitDefaults" )
		{
			asCScriptNode *node = classDecl->node;

			int r = 0, c = 0;
			if( node )
				current->script->ConvertPosToRowCol(node->tokenPos, &r, &c);

			asCString str = func->GetDeclarationStr(true, false, false, true);
			str.Format(TXT_COMPILING_s, str.AddressOf());
			WriteInfo(current->script->name, str, r, c, true);

			// When compiling a constructor need to pass the class declaration for member initializations
			compiler.CompileFunction(this, current->script, current->paramNames, node, func, classDecl);

			engine->preMessage.isSet = false;
		}
		else
		{
			asASSERT( false );
		}
	}
}
#endif

// Called from module and engine
int asCBuilder::ParseDataType(const char *datatype, asCDataType *result, asSNameSpace *implicitNamespace, bool isReturnType, bool isAppInterface)
{
	Reset();

	asCScriptCode source;
	source.SetCode("", datatype, true);

	asCParser parser(this);
	int r = parser.ParseDataType(&source, isReturnType, isAppInterface);
	if( r < 0 )
		return asINVALID_TYPE;

	// Get data type and property name
	asCScriptNode *dataType = parser.GetScriptNode()->firstChild;

	*result = CreateDataTypeFromNode(dataType, &source, implicitNamespace, true);
	if( isReturnType )
		*result = ModifyDataTypeFromNode(*result, dataType->next, &source, 0, 0);

	if( numErrors > 0 )
		return asINVALID_TYPE;

	return asSUCCESS;
}

int asCBuilder::ParseTemplateDecl(const char *decl, asCString *name, asCArray<asCString> &subtypeNames)
{
	Reset();

	asCScriptCode source;
	source.SetCode("", decl, true);

	asCParser parser(this);
	int r = parser.ParseTemplateDecl(&source);
	if( r < 0 )
		return asINVALID_TYPE;

	// Get the template name and subtype names
	asCScriptNode *node = parser.GetScriptNode()->firstChild;

	name->Assign(&decl[node->tokenPos], node->tokenLength);
	while( (node = node->next) != 0 )
	{
		asCString subtypeName;
		subtypeName.Assign(&decl[node->tokenPos], node->tokenLength);
		subtypeNames.PushLast(subtypeName);
	}

	// TODO: template: check for name conflicts

	if( numErrors > 0 )
		return asINVALID_DECLARATION;

	return asSUCCESS;
}

int asCBuilder::VerifyProperty(asCDataType *dt, const char *decl, asCString &name, asCDataType &type, asSNameSpace *ns)
{
	// Either datatype or namespace must be informed
	asASSERT( dt || ns );

	Reset();

	if( dt )
	{
		// Verify that the object type exist
		if( CastToObjectType(dt->GetTypeInfo()) == 0 )
			return asINVALID_OBJECT;
	}

	// Check property declaration and type
	asCScriptCode source;
	source.SetCode(TXT_PROPERTY, decl, true);

	asCParser parser(this);
	int r = parser.ParsePropertyDeclaration(&source, true);
	if( r < 0 )
		return asINVALID_DECLARATION;

	// Get data type
	asCScriptNode *dataType = parser.GetScriptNode()->firstChild;

	// Check if the property is declared 'by reference'
	bool isReference = (dataType->next->tokenType == ttAmp);

	// Get the name of the property
	asCScriptNode *nameNode = isReference ? dataType->next->next : dataType->next;

	// If an object property is registered, then use the
	// object's namespace, otherwise use the specified namespace
	type = CreateDataTypeFromNode(dataType, &source, dt ? dt->GetTypeInfo()->nameSpace : ns);
	name.Assign(&decl[nameNode->tokenPos], nameNode->tokenLength);
	type.MakeReference(isReference);

	// Validate that the type really can be a registered property
	// We cannot use CanBeInstantiated, as it is allowed to register
	// properties of type that cannot otherwise be instantiated
	if( type.IsFuncdef() && !type.IsObjectHandle() )
	{
		// Function definitions must always be handles
		return asINVALID_DECLARATION;
	}

	// Verify property name
	if( dt )
	{
		if( CheckNameConflictMember(dt->GetTypeInfo(), name.AddressOf(), nameNode, &source, true) < 0 )
			return asNAME_TAKEN;
	}
	else
	{
		if( CheckNameConflict(name.AddressOf(), nameNode, &source, ns) < 0 )
			return asNAME_TAKEN;
	}

	if( numErrors > 0 )
		return asINVALID_DECLARATION;

	return asSUCCESS;
}

#ifndef AS_NO_COMPILER
asCObjectProperty *asCBuilder::GetObjectProperty(asCDataType &obj, const char *prop)
{
	asASSERT(CastToObjectType(obj.GetTypeInfo()) != 0);

	auto* objectType = CastToObjectType(obj.GetTypeInfo());
	asCObjectProperty* foundProp = nullptr;

	objectType->FindPropertyUntil(prop, [&](asCObjectProperty* checkProp) -> bool
	{
		foundProp = checkProp;
		return true;
	});


	return foundProp;
}
#endif

bool asCBuilder::DoesGlobalPropertyExist(const char *prop, asSNameSpace *ns, asCGlobalProperty **outProp, sGlobalVariableDescription **outDesc, bool *isAppProp, bool bAllowAnyModule)
{
	if( outProp )   *outProp = 0;
	if( outDesc )   *outDesc = 0;
	if( isAppProp ) *isAppProp = false;

	// Check application registered properties
	asCGlobalProperty *globProp = engine->registeredGlobalPropTable.FindFirst(prop, ns);
	if( globProp )
	{
		if( isAppProp ) *isAppProp = true;
		if( outProp )   *outProp   = globProp;
		return true;
	}

#ifndef AS_NO_COMPILER
	// Check properties being compiled now
	sGlobalVariableDescription* desc = globVariables.FindFirst(prop, ns);
	if( desc && !desc->isEnumValue )
	{
		if( outProp ) *outProp = desc->property;
		if( outDesc ) *outDesc = desc;
		return true;
	}
#endif

	// Check previously compiled global variables
	if( module )
	{
		if (engine->ep.automaticImports && ((ns && ns != module->defaultNamespace) || bAllowAnyModule))
		{
			globProp = engine->allScriptGlobalVariables.FindFirst(prop, ns);
			if( globProp )
			{
				if( outProp ) *outProp = globProp;
				return true;
			}
		}
		else
		{
			globProp = module->GetGlobalProperty(ns, prop);
			if( globProp )
			{
				if( outProp ) *outProp = globProp;
				return true;
			}
		}
	}

	return false;
}

asCGlobalProperty *asCBuilder::GetGlobalProperty(const char *prop, asSNameSpace *ns, bool *isCompiled, bool *isPureConstant, asQWORD *constantValue, bool *isAppProp, bool bAllowAnyModule)
{
	if( isCompiled )     *isCompiled     = true;
	if( isPureConstant ) *isPureConstant = false;
	if( isAppProp )      *isAppProp      = false;
	if( constantValue )  *constantValue  = 0;

	asCGlobalProperty          *globProp = 0;
	sGlobalVariableDescription *globDesc = 0;
	if( DoesGlobalPropertyExist(prop, ns, &globProp, &globDesc, isAppProp, bAllowAnyModule) )
	{
#ifndef AS_NO_COMPILER
		if( globDesc )
		{
			// The property was declared in this build call, check if it has been compiled successfully already
			if( isCompiled )     *isCompiled     = globDesc->isCompiled;
			if( isPureConstant ) *isPureConstant = globDesc->isPureConstant;
			if( constantValue  ) *constantValue  = globDesc->constantValue;
		}
		else if (globProp->isPureConstant
			// In editor, pure constants declared in script are not hardcoded into sets,
			// because we may hotreload the file with the global variable in it and we want
			// to always use the updated value. In cooked we _do_ consider them pure constant,
			// that way we get more optimized code.
			&& (!GIsEditor || globProp->module == nullptr))
		{
			if( isPureConstant ) *isPureConstant = true;
			if( constantValue  ) *constantValue  = globProp->storage;
		}
#endif

		return globProp;
	}

	return 0;
}

int asCBuilder::ParseFunctionDeclaration(asCObjectType *objType, const char *decl, asCScriptFunction *func, bool isSystemFunction, asCArray<bool> *paramAutoHandles, bool *returnAutoHandle, asSNameSpace *ns, asCScriptNode **listPattern, asCObjectType **outParentClass)
{
	asASSERT( objType || ns );

	if (listPattern)
		*listPattern = 0;
	if (outParentClass)
		*outParentClass = 0;

	// TODO: Can't we use GetParsedFunctionDetails to do most of what is done in this function?

	Reset();

	asCScriptCode source;
	source.SetCode(TXT_SYSTEM_FUNCTION, decl, true);

	asCParser parser(this);
	int r = parser.ParseFunctionDefinition(&source, listPattern != 0);
	if( r < 0 )
		return asINVALID_DECLARATION;

	asCScriptNode *node = parser.GetScriptNode();

	// Determine scope
	asCScriptNode *n = node->firstChild->next->next;
	asCObjectType *parentClass = 0;
	func->nameSpace = GetNameSpaceFromNode(n, &source, ns, &n, &parentClass);
	if( func->nameSpace == 0 && parentClass == 0 )
		return asINVALID_DECLARATION;
	if (parentClass && func->funcType != asFUNC_FUNCDEF)
		return asINVALID_DECLARATION;

	if (outParentClass)
		*outParentClass = parentClass;

	// Find name
	func->name.Assign(&source.code[n->tokenPos], n->tokenLength);

	// Initialize a script function object for registration
	bool autoHandle;

	// Scoped reference types are allowed to use handle when returned from application functions
	func->returnType = CreateDataTypeFromNode(node->firstChild, &source, objType ? objType->nameSpace : ns, true, parentClass ? parentClass : objType);
	func->returnType = ModifyDataTypeFromNode(func->returnType, node->firstChild->next, &source, 0, &autoHandle);
	if( autoHandle && (!func->returnType.IsObjectHandle() || func->returnType.IsReference()) )
		return asINVALID_DECLARATION;
	if( returnAutoHandle ) *returnAutoHandle = autoHandle;

	// Reference types cannot be returned by value from system functions
	if( isSystemFunction &&
		(func->returnType.GetTypeInfo() &&
		 (func->returnType.GetTypeInfo()->flags & asOBJ_REF)) &&
		!(func->returnType.IsReference() ||
		  func->returnType.IsObjectHandle()) )
		return asINVALID_DECLARATION;

	// Count number of parameters
	int paramCount = 0;
	asCScriptNode *paramList = n->next;
	n = paramList->firstChild;
	while( n )
	{
		paramCount++;
		n = n->next->next;
		if( n && n->nodeType == snIdentifier )
			n = n->next;

		if( n && n->nodeType == snExpression )
			n = n->next;
	}

	// Preallocate memory
	func->parameterTypes.Allocate(paramCount, false);
	func->parameterNames.SetLength(paramCount);
	func->inOutFlags.Allocate(paramCount, false);
	func->defaultArgs.Allocate(paramCount, false);
	if( paramAutoHandles ) paramAutoHandles->Allocate(paramCount, false);

	n = paramList->firstChild;
	asUINT index = 0;
	while( n )
	{
		asETypeModifiers inOutFlags;
		asCDataType type = CreateDataTypeFromNode(n, &source, objType ? objType->nameSpace : ns, false, parentClass ? parentClass : objType);
		type = ModifyDataTypeFromNode(type, n->next, &source, &inOutFlags, &autoHandle);

		// Reference types cannot be passed by value to system functions
		if( isSystemFunction &&
			(type.GetTypeInfo() &&
			 (type.GetTypeInfo()->flags & asOBJ_REF)) &&
			!(type.IsReference() ||
			  type.IsObjectHandle()) )
			return asINVALID_DECLARATION;

		if (!isSystemFunction && !type.IsReference())
		{
			// Script functions that take a value type by value should always take a const ref instead
			auto* typeInfo = type.GetTypeInfo();
			if (typeInfo != nullptr && typeInfo->GetFlags() & asOBJ_VALUE)
			{
				type.MakeReference(true);
				type.MakeReadOnly(true);
				inOutFlags = asETypeModifiers::asTM_INOUTREF;
			}

			// Script functions that take a primitive type by value should make it const, for consistency
			// so parameters can never be changed.
			if (type.IsPrimitive() && !type.IsReadOnly())
			{
				type.MakeReadOnly(true);
			}
		}

		// Store the parameter type
		func->parameterTypes.PushLast(type);
		func->inOutFlags.PushLast(inOutFlags);

		// Don't permit void parameters
		if( type.GetTokenType() == ttVoid )
			return asINVALID_DECLARATION;

		if( autoHandle && (!type.IsObjectHandle() || type.IsReference()) )
			return asINVALID_DECLARATION;

		if( paramAutoHandles ) paramAutoHandles->PushLast(autoHandle);

		// Make sure that var type parameters are references
		if( type.GetTokenType() == ttQuestion &&
			!type.IsReference() )
			return asINVALID_DECLARATION;

		// Move to next parameter
		n = n->next->next;
		if( n && n->nodeType == snIdentifier )
		{
			func->parameterNames[index] = asCString(&source.code[n->tokenPos], n->tokenLength);
			n = n->next;
		}
		++index;

		if( n && n->nodeType == snExpression )
		{
			// Strip out white space and comments to better share the string
			asCString *defaultArgStr = asNEW(asCString);
			if( defaultArgStr )
			{
				*defaultArgStr = GetCleanExpressionString(n, &source);
				func->defaultArgs.PushLast(defaultArgStr);
			}

			n = n->next;
		}
		else
			func->defaultArgs.PushLast(0);
	}

	// Set the read-only flag if const is declared after parameter list
	n = paramList->next;
	if( n && n->nodeType == snUndefined && n->tokenType == ttConst )
	{
		if( objType == 0 )
			return asINVALID_DECLARATION;
		func->SetReadOnly(true);

		n = n->next;
	}
	else
		func->SetReadOnly(false);

	// Check for additional function traits
	while (n && n->nodeType == snIdentifier)
	{
		if (source.TokenEquals(n->tokenPos, n->tokenLength, PROPERTY_TOKEN))
			func->SetProperty(true);
		else if (source.TokenEquals(n->tokenPos, n->tokenLength, ACCEPT_TEMPORARY_TOKEN))
			func->traits.SetTrait(asTRAIT_ACCEPT_TEMPORARY_OBJECT, true);
		else if (source.TokenEquals(n->tokenPos, n->tokenLength, EXTERNAL_IMPLICIT_THIS_TOKEN))
			func->traits.SetTrait(asTRAIT_EXTERNAL_IMPLICIT_THIS, true);
		else if (source.TokenEquals(n->tokenPos, n->tokenLength, NO_DISCARD_TOKEN))
			func->traits.SetTrait(asTRAIT_NODISCARD, true);
		else if (source.TokenEquals(n->tokenPos, n->tokenLength, ALLOW_DISCARD_TOKEN))
			func->traits.SetTrait(asTRAIT_ALLOWDISCARD, true);
		else if (source.TokenEquals(n->tokenPos, n->tokenLength, GENERATED_FUNCTION_TOKEN))
			func->traits.SetTrait(asTRAIT_GENERATED_FUNCTION, true);
		else if (source.TokenEquals(n->tokenPos, n->tokenLength, DEPRECATED_TOKEN))
			func->traits.SetTrait(asTRAIT_DEPRECATED, true);
		else
			return asINVALID_DECLARATION;

		n = n->next;
	}

	// If the caller expects a list pattern, check for the existence, else report an error if not
	if( listPattern )
	{
		if( n == 0 || n->nodeType != snListPattern )
			return asINVALID_DECLARATION;
		else
		{
			*listPattern = n;
			n->DisconnectParent();
		}
	}
	else
	{
		if( n )
			return asINVALID_DECLARATION;
	}

	// Make sure the default args are declared correctly
	ValidateDefaultArgs(&source, node, func);

	if( numErrors > 0 || numWarnings > 0 )
		return asINVALID_DECLARATION;

	return 0;
}

int asCBuilder::ParseVariableDeclaration(const char *decl, asSNameSpace *implicitNamespace, asCString &outName, asSNameSpace *&outNamespace, asCDataType &outDt)
{
	Reset();

	asCScriptCode source;
	source.SetCode(TXT_VARIABLE_DECL, decl, true);

	asCParser parser(this);

	int r = parser.ParsePropertyDeclaration(&source, false);
	if( r < 0 )
		return asINVALID_DECLARATION;

	asCScriptNode *node = parser.GetScriptNode();

	// Determine the scope from declaration
	asCScriptNode *n = node->firstChild->next;
	// TODO: child funcdef: The parentType will be set if the scope is actually a type rather than a namespace
	outNamespace = GetNameSpaceFromNode(n, &source, implicitNamespace, &n);
	if( outNamespace == 0 )
		return asINVALID_DECLARATION;

	// Find name
	outName.Assign(&source.code[n->tokenPos], n->tokenLength);

	// Initialize a script variable object for registration
	outDt = CreateDataTypeFromNode(node->firstChild, &source, implicitNamespace);

	if( numErrors > 0 || numWarnings > 0 )
		return asINVALID_DECLARATION;

	return 0;
}

int asCBuilder::CheckNameConflictMember(asCTypeInfo *t, const char *name, asCScriptNode *node, asCScriptCode *code, bool isProperty)
{
	// It's not necessary to check against object types

	asCObjectType *ot = CastToObjectType(t);
	if (!ot)
		return 0;

	if (ot->GetFirstProperty_CaseInsensitive(name) != nullptr)
	{
		if( code )
		{
			asCString str;
			str.Format(TXT_NAME_CONFLICT_s_OBJ_PROPERTY, name);
			WriteError(str, code, node);
		}

		return -1;
	}

	asCArray<asCFuncdefType*> &funcdefs = ot->childFuncDefs;
	for (asUINT n = 0; n < funcdefs.GetLength(); n++)
	{
		if (funcdefs[n]->name == name)
		{
			if (code)
			{
				asCString str;
				str.Format(TXT_NAME_CONFLICT_s_IS_FUNCDEF, name);
				WriteError(str, code, node);
			}

			return -1;
		}
	}

	// Property names must be checked against method names
	if( isProperty )
	{
		if (ot->GetFirstMethod_CaseInsensitive(name) != nullptr)
		{
			if( code )
			{
				asCString str;
				str.Format(TXT_NAME_CONFLICT_s_METHOD, name);
				WriteError(str, code, node);
			}

			return -1;
		}
	}

	return 0;
}

int asCBuilder::CheckNameConflict(const char *name, asCScriptNode *node, asCScriptCode *code, asSNameSpace *ns)
{
	// Check against registered object types
	// TODO: Must check against registered funcdefs too
	asCTypeInfo *ti = nullptr;
	engine->allRegisteredTypesByName.FindAllUntil_CaseInsensitive(name, [&](asCTypeInfo* checkType)
	{
		if (checkType->nameSpace == ns)
		{
			ti = checkType;
			return true;
		}
		return false;
	});

	if( ti != nullptr )
	{
		if( code )
		{
			asCString str;
			str.Format(TXT_NAME_CONFLICT_s_EXTENDED_TYPE, name);
			WriteError(str, code, node);
		}

		return -1;
	}

	// Check against global properties
	if( DoesGlobalPropertyExist(name, ns) )
	{
		if( code )
		{
			asCString str;
			str.Format(TXT_NAME_CONFLICT_s_GLOBAL_PROPERTY, name);
			WriteError(str, code, node);
		}

		return -1;
	}

	// TODO: Property names must be checked against function names

#ifndef AS_NO_COMPILER
	// Check against class types
	asUINT n;
	for( n = 0; n < classDeclarations.GetLength(); n++ )
	{
		if( classDeclarations[n]->name.Equals_CaseInsensitive(name) &&
			classDeclarations[n]->typeInfo->nameSpace == ns )
		{
			if( code )
			{
				asCString str;
				str.Format(TXT_NAME_CONFLICT_s_STRUCT, name);
				WriteError(str, code, node);
			}

			return -1;
		}
	}

	// Check against named types
	for( n = 0; n < namedTypeDeclarations.GetLength(); n++ )
	{
		if( namedTypeDeclarations[n]->name.Equals_CaseInsensitive(name) &&
			namedTypeDeclarations[n]->typeInfo->nameSpace == ns )
		{
			if( code )
			{
				asCString str;
				str.Format(TXT_NAME_CONFLICT_s_IS_NAMED_TYPE, name);
				WriteError(str, code, node);
			}

			return -1;
		}
	}

	// Must check for name conflicts with funcdefs
	for( n = 0; n < funcDefs.GetLength(); n++ )
	{
		if( funcDefs[n]->name == name &&
			module->funcDefs[funcDefs[n]->idx]->nameSpace == ns )
		{
			if( code )
			{
				asCString str;
				str.Format(TXT_NAME_CONFLICT_s_IS_FUNCDEF, name);
				WriteError(str, code, node);
			}

			return -1;
		}
	}

	// Check against mixin classes
	if( GetMixinClass(name, ns) )
	{
		if( code )
		{
			asCString str;
			str.Format(TXT_NAME_CONFLICT_s_IS_MIXIN, name);
			WriteError(str, code, node);
		}

		return -1;
	}
#endif

	return 0;
}

// Returns a negative value on invalid property
// -2 incorrect prefix
// -3 invalid signature
int asCBuilder::ValidatePropertyAccessorFunc(asCScriptFunction* func)
{
	if (!func->IsProperty())
		return 0;

	bool hasGetPrefix = func->name.StartsWith(AS_GET_PREFIX);
	bool hasSetPrefix = func->name.StartsWith(AS_SET_PREFIX);

	// A property accessor func must have the prefix one of the get/set prefixes
	if (!hasGetPrefix && !hasSetPrefix)
		return -2;

	// A getter must return a non-void type and have at most 1 argument (indexed property)
	if (hasGetPrefix && (func->returnType == asCDataType::CreatePrimitive(ttVoid, false) || func->parameterTypes.GetLength() > 1))
		return -3;

	// A setter must return a void and have 1 or 2 arguments (indexed property)
	if (hasSetPrefix && (func->returnType != asCDataType::CreatePrimitive(ttVoid, false) || func->parameterTypes.GetLength() < 1 || func->parameterTypes.GetLength() > 2))
		return -3;

	// Everything is OK
	return 0;
}

#ifndef AS_NO_COMPILER
sMixinClass *asCBuilder::GetMixinClass(const char *name, asSNameSpace *ns)
{
	for( asUINT n = 0; n < mixinClasses.GetLength(); n++ )
		if( mixinClasses[n]->name == name &&
			mixinClasses[n]->ns == ns )
			return mixinClasses[n];

	return 0;
}

int asCBuilder::RegisterFuncDef(asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns, asCObjectType *parent)
{
	// namespace and parent are exclusively mutual
	asASSERT((ns == 0 && parent) || (ns && parent == 0));

	// Skip leading 'shared' and 'external' keywords
	asCScriptNode *n = node->firstChild;
	while (n->nodeType == snIdentifier)
		n = n->next;

	// Find the name
	asASSERT( n->nodeType == snDataType );
	n = n->next->next;

	asCString name;
	name.Assign(&file->code[n->tokenPos], n->tokenLength);

	// Check for name conflict with other types
	if (ns)
	{
		int r = CheckNameConflict(name.AddressOf(), node, file, ns);
		if (asSUCCESS != r)
		{
			return r;
		}
	}
	else
	{
		int r = CheckNameConflictMember(parent, name.AddressOf(), node, file, false);
		if (asSUCCESS != r)
		{
			return r;
		}
	}

	// The function definition should be stored as a asCScriptFunction so that the application
	// can use the asIScriptFunction interface to enumerate the return type and parameters

	// The return type and parameter types aren't determined in this function. A second pass is
	// necessary after all type declarations have been identified. The second pass is implemented
	// in CompleteFuncDef().

	sFuncDef *fd = asNEW(sFuncDef);
	if( fd == 0 )
	{
		return asOUT_OF_MEMORY;
	}

	fd->name   = name;
	fd->node   = node;
	fd->script = file;
	fd->idx    = module->AddFuncDef(name, ns, parent);

	funcDefs.PushLast(fd);

	return 0;
}

void asCBuilder::CompleteFuncDef(sFuncDef *funcDef)
{
	asCArray<asCString *>      defaultArgs;
	asSFunctionTraits          funcTraits;

	asCFuncdefType *fdt = module->funcDefs[funcDef->idx];
	asASSERT( fdt );
	asCScriptFunction *func = fdt->funcdef;

	asSNameSpace *implicitNs = func->nameSpace ? func->nameSpace : fdt->parentClass->nameSpace;
	GetParsedFunctionDetails(funcDef->node, funcDef->script, fdt->parentClass, funcDef->name, func->returnType, func->parameterNames, func->parameterTypes, func->inOutFlags, defaultArgs, funcTraits, implicitNs);

	// There should not be any defaultArgs, but if there are any we need to delete them to avoid leaks
	for( asUINT n = 0; n < defaultArgs.GetLength(); n++ )
		if( defaultArgs[n] )
			asDELETE(defaultArgs[n], asCString);

	// All funcdefs are shared, unless one of the parameter types or return type is not shared
	bool declaredShared = funcTraits.GetTrait(asTRAIT_SHARED);
	funcTraits.SetTrait(asTRAIT_SHARED, true);
	if (func->returnType.GetTypeInfo() && !func->returnType.GetTypeInfo()->IsShared())
	{
		if (declaredShared)
		{
			asCString s;
			s.Format(TXT_SHARED_CANNOT_USE_NON_SHARED_TYPE_s, func->returnType.GetTypeInfo()->name.AddressOf());
			WriteError(s.AddressOf(), funcDef->script, funcDef->node);
		}
		funcTraits.SetTrait(asTRAIT_SHARED, false);
	}
	for( asUINT n = 0; funcTraits.GetTrait(asTRAIT_SHARED) && n < func->parameterTypes.GetLength(); n++ )
		if (func->parameterTypes[n].GetTypeInfo() && !func->parameterTypes[n].GetTypeInfo()->IsShared())
		{
			if (declaredShared)
			{
				asCString s;
				s.Format(TXT_SHARED_CANNOT_USE_NON_SHARED_TYPE_s, func->parameterTypes[n].GetTypeInfo()->name.AddressOf());
				WriteError(s.AddressOf(), funcDef->script, funcDef->node);
			}
			funcTraits.SetTrait(asTRAIT_SHARED, false);
		}
	func->SetShared(funcTraits.GetTrait(asTRAIT_SHARED));
	func->CalculateParameterOffsets();

	// Check if there is another identical funcdef from another module and if so reuse that instead
	bool found = false;
	if( func->IsShared() )
	{
		for( asUINT n = 0; n < engine->funcDefs.GetLength(); n++ )
		{
			asCFuncdefType *fdt2 = engine->funcDefs[n];
			if( fdt2 == 0 || fdt == fdt2 )
				continue;

			if( !fdt2->funcdef->IsShared() )
				continue;

			if( fdt2->name == fdt->name &&
				fdt2->nameSpace == fdt->nameSpace &&
				fdt2->funcdef->IsSignatureExceptNameEqual(func) )
			{
				// Replace our funcdef for the existing one
				funcDef->idx = fdt2->funcdef->id;
				module->funcDefs[module->funcDefs.IndexOf(fdt)] = fdt2;
				fdt2->AddRefInternal();

				engine->funcDefs.RemoveValue(fdt);

				fdt->ReleaseInternal();
				found = true;
				break;
			}
		}
	}

	// If the funcdef was declared as external then the existing shared declaration must have been found
	if (funcTraits.GetTrait(asTRAIT_EXTERNAL) && !found)
	{
		asCString str;
		str.Format(TXT_EXTERNAL_SHARED_s_NOT_FOUND, funcDef->name.AddressOf());
		WriteError(str, funcDef->script, funcDef->node);
	}

	// Remember if the type was declared as external so the saved bytecode can be flagged accordingly
	if (funcTraits.GetTrait(asTRAIT_EXTERNAL) && found)
		module->externalTypes.PushLast(engine->scriptFunctions[funcDef->idx]->funcdefType);
}

int asCBuilder::RegisterGlobalVar(asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns)
{
	// Has the application disabled global vars?
	if( engine->ep.disallowGlobalVars )
		WriteError(TXT_GLOBAL_VARS_NOT_ALLOWED, file, node);

	// What data type is it?
	asCDataType type = CreateDataTypeFromNode(node->firstChild, file, ns);

	if( !type.CanBeInstantiated() )
	{
		asCString str;
		if( type.IsAbstractClass() )
			str.Format(TXT_ABSTRACT_CLASS_s_CANNOT_BE_INSTANTIATED, type.Format(ns).AddressOf());
		else if( type.IsInterface() )
			str.Format(TXT_INTERFACE_s_CANNOT_BE_INSTANTIATED, type.Format(ns).AddressOf());
		else
			// TODO: Improve error message to explain why
			str.Format(TXT_DATA_TYPE_CANT_BE_s, type.Format(ns).AddressOf());

		WriteError(str, file, node);
	}

	asCScriptNode *n = node->firstChild->next;

	while( n )
	{
		// Verify that the name isn't taken
		asCString name(&file->code[n->tokenPos], n->tokenLength);
		if (name.GetLength() < 2 || name[0] != '_' || name[1] != '_')
			CheckNameConflict(name.AddressOf(), n, file, ns);

		// Register the global variable
		sGlobalVariableDescription *gvar = asNEW(sGlobalVariableDescription);
		if( gvar == 0 )
		{
			return asOUT_OF_MEMORY;
		}

		gvar->script      = file;
		gvar->name        = name;
		gvar->isCompiled  = false;
		gvar->datatype    = type;
		gvar->isEnumValue = false;
		gvar->nameSpace          = ns;
		gvar->isPureConstant = false;

		// We loosen these restrictions for global variables that start with _, as they are used internally and are
		// assumed to know exactly what they're doing.
		if (gvar->name.GetLength() >= 1 && gvar->name[0] != '_')
		{
			// We don't support handles as global variables
			if (gvar->datatype.IsObjectHandle())
			{
				asCString str;
				str.Format("Global variable '%s' invalid. Class types are not supported for global variables.", gvar->name.AddressOf());
				WriteError(str, file, n);
			}
			// Global variables must always be const, we don't support modifying them
			else if (!gvar->datatype.IsReadOnly())
			{
				asCString str;
				str.Format("Global variable '%s' must be const. Mutable global variables are not supported.", gvar->name.AddressOf());
				WriteError(str, file, n);
			}
		}

		// TODO: Give error message if wrong
		asASSERT(!gvar->datatype.IsReference());

		// Allocation is done when the variable is compiled, to allow for autos
		gvar->property = 0;
		gvar->index    = 0;

		globVariables.Add(gvar);
		globVariableList.PushLast(gvar);

		gvar->declaredAtNode = n;
		n = n->next;
		gvar->declaredAtNode->DisconnectParent();
		gvar->initializationNode = 0;
		if( n &&
			( n->nodeType == snAssignment ||
			  n->nodeType == snArgList    ||
			  n->nodeType == snInitList     ) )
		{
			gvar->initializationNode = n;
			n = n->next;
			gvar->initializationNode->DisconnectParent();
		}
	}

	return 0;
}

int asCBuilder::RegisterMixinClass(asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns)
{
	asCScriptNode *cl = node->firstChild;
	asASSERT( cl->nodeType == snClass );

	asCScriptNode *n = cl->firstChild;

	// Skip potential 'final' and 'shared' tokens
	/*while( n->tokenType == ttIdentifier &&
		   (file->TokenEquals(n->tokenPos, n->tokenLength, FINAL_TOKEN) ||
		    file->TokenEquals(n->tokenPos, n->tokenLength, SHARED_TOKEN)) )
	{
		// Report error, because mixin class cannot be final or shared
		asCString msg;
		msg.Format(TXT_MIXIN_CANNOT_BE_DECLARED_AS_s, asCString(&file->code[n->tokenPos], n->tokenLength).AddressOf());
		WriteError(msg, file, n);

		asCScriptNode *tmp = n;
		n = n->next;

		// Remove the invalid node, so compilation can continue as if it wasn't there
		tmp->DisconnectParent();
		tmp->Destroy(engine);
	}*/

	asCString name(&file->code[n->tokenPos], n->tokenLength);

	int r, c;
	file->ConvertPosToRowCol(n->tokenPos, &r, &c);

	CheckNameConflict(name.AddressOf(), n, file, ns);

	sMixinClass *decl = asNEW(sMixinClass);
	if( decl == 0 )
	{
		return asOUT_OF_MEMORY;
	}

	mixinClasses.PushLast(decl);
	decl->name   = name;
	decl->ns     = ns;
	decl->node   = cl;
	decl->script = file;

	// Clean up memory
	cl->DisconnectParent();

	// Check that the mixin class doesn't contain any child types
	// TODO: Add support for child types in mixin classes
	n = cl->firstChild;
	while (n)
	{
		if (n->nodeType == snFuncDef)
		{
			WriteError(TXT_MIXIN_CANNOT_HAVE_CHILD_TYPES, file, n);
			break;
		}
		n = n->next;
	}

	return 0;
}

int asCBuilder::RegisterClass(asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns)
{
	asCScriptNode *n = node->firstChild;
	bool isFinal = false;
	bool isShared = false;
	bool isAbstract = false;
	bool isTidy = false;
	bool isStruct = node->tokenType == ttStruct;
	bool isExternal = false;

	// Check the class modifiers
	/*while( n->tokenType == ttIdentifier )
	{
		if( file->TokenEquals(n->tokenPos, n->tokenLength, FINAL_TOKEN) )
		{
			if( isAbstract )
				WriteError(TXT_CLASS_CANT_BE_FINAL_AND_ABSTRACT, file, n);
			else
			{
				if( isFinal )
				{
					asCString msg;
					msg.Format(TXT_ATTR_s_INFORMED_MULTIPLE_TIMES, asCString(&file->code[n->tokenPos], n->tokenLength).AddressOf());
					WriteWarning(msg, file, n);
				}
				isFinal = true;
			}
		}
		else if( file->TokenEquals(n->tokenPos, n->tokenLength, SHARED_TOKEN) )
		{
			if( isShared )
			{
				asCString msg;
				msg.Format(TXT_ATTR_s_INFORMED_MULTIPLE_TIMES, asCString(&file->code[n->tokenPos], n->tokenLength).AddressOf());
				WriteWarning(msg, file, n);
			}
			isShared = true;
		}
		else if (file->TokenEquals(n->tokenPos, n->tokenLength, EXTERNAL_TOKEN))
		{
			if (isExternal)
			{
				asCString msg;
				msg.Format(TXT_ATTR_s_INFORMED_MULTIPLE_TIMES, asCString(&file->code[n->tokenPos], n->tokenLength).AddressOf());
				WriteWarning(msg, file, n);
			}
			isExternal = true;
		}
		else if( file->TokenEquals(n->tokenPos, n->tokenLength, ABSTRACT_TOKEN) )
		{
			if( isFinal )
				WriteError(TXT_CLASS_CANT_BE_FINAL_AND_ABSTRACT, file, n);
			else
			{
				if( isAbstract )
				{
					asCString msg;
					msg.Format(TXT_ATTR_s_INFORMED_MULTIPLE_TIMES, asCString(&file->code[n->tokenPos], n->tokenLength).AddressOf());
					WriteWarning(msg, file, n);
				}
				isAbstract = true;
			}
		}
		else if( file->TokenEquals(n->tokenPos, n->tokenLength, TIDY_TOKEN) )
		{
			isTidy = true;
		}
		else
		{
			// This is the name of the class
			break;
		}

		n = n->next;
	}*/

	asCString name(&file->code[n->tokenPos], n->tokenLength);

	int r, c;
	file->ConvertPosToRowCol(n->tokenPos, &r, &c);

	CheckNameConflict(name.AddressOf(), n, file, ns);

	sClassDeclaration *decl = asNEW(sClassDeclaration);
	if( decl == 0 )
	{
		return asOUT_OF_MEMORY;
	}

	classDeclarations.PushLast(decl);
	decl->name             = name;
	decl->script           = file;
	decl->node             = node;
	decl->isTidy           = isTidy;

	// External shared interfaces must not try to redefine the interface
	if (isExternal && (n->next == 0 || n->next->tokenType != ttEndStatement))
	{
		asCString str;
		str.Format(TXT_EXTERNAL_SHARED_s_CANNOT_REDEF, name.AddressOf());
		WriteError(str, file, n);
	}
	else if (!isExternal && n->next && n->next->tokenType == ttEndStatement)
	{
		asCString str;
		str.Format(TXT_MISSING_DEFINITION_OF_s, name.AddressOf());
		WriteError(str, file, n);
	}

	// If this type is shared and there already exist another shared
	// type of the same name, then that one should be used instead of
	// creating a new one.
	asCObjectType *st = 0;
	if( isShared )
	{
		for( asUINT i = 0; i < engine->sharedScriptTypes.GetLength(); i++ )
		{
			st = CastToObjectType(engine->sharedScriptTypes[i]);
			if( st &&
				st->IsShared() &&
				st->name == name &&
				st->nameSpace == ns &&
				!st->IsInterface() )
			{
				// We'll use the existing type
				decl->isExistingShared = true;
				decl->typeInfo         = st;
				module->classTypes.PushLast(st);
				module->allLocalTypes.Add(st);
				st->AddRefInternal();
				break;
			}
		}
	}

	// If the class was declared as external then it must have been compiled in a different module first
	if (isExternal && decl->typeInfo == 0)
	{
		asCString str;
		str.Format(TXT_EXTERNAL_SHARED_s_NOT_FOUND, name.AddressOf());
		WriteError(str, file, n);
	}

	// Remember if the class was declared as external so the saved bytecode can be flagged accordingly
	if (isExternal)
		module->externalTypes.PushLast(st);

	if (!decl->isExistingShared)
	{
		// Create a new object type for this class
		st = asNEW(asCObjectType)(engine);
		if (st == 0)
			return asOUT_OF_MEMORY;

		// By default all script classes are marked as garbage collected.
		// Only after the complete structure and relationship between classes
		// is known, can the flag be cleared for those objects that truly cannot
		// form circular references. This is important because a template
		// callback may be called with a script class before the compilation
		// completes, and until it is known, the callback must assume the class
		// is garbage collected.
		st->flags = asOBJ_REF | asOBJ_SCRIPT_OBJECT | asOBJ_NOCOUNT;

#if WITH_EDITOR
		if (IsNodeInEditorOnlyCode(decl->script, decl->node))
			st->flags |= asOBJ_EDITOR_ONLY;
#endif

		if (isShared)
			st->flags |= asOBJ_SHARED;

		if (isFinal)
			st->flags |= asOBJ_NOINHERIT;

		if (isAbstract)
			st->flags |= asOBJ_ABSTRACT;

		//if (node->tokenType == ttHandleSuffix)
			//st->flags |= asOBJ_IMPLICIT_HANDLE;

		if (isStruct)
		{
			// All script structs are by value
			st->flags &= ~(asOBJ_REF | asOBJ_NOCOUNT);
			st->flags |= asOBJ_VALUE;
			st->flags |= asOBJ_NOINHERIT;
		}
		else
		{
			// All script classes are implicit handle
			st->flags |= asOBJ_IMPLICIT_HANDLE;
		}

		st->size = -1;

		asPreClassData* classData = module->PreClassData.Find(name);
		if (classData != nullptr)
		{
			st->basePropertyOffset = (int)classData->PropertyOffset;
			st->shadowType = classData->ShadowType;
			st->plainUserData = (asPWORD)classData->InitialUserData;

			if (st->shadowType != nullptr && st->shadowType->alignment > st->alignment)
				st->alignment = st->shadowType->alignment;
		}
		else
		{
			if (!isStruct)
			{
				WriteError(TXT_CANNOT_INHERIT_FROM_STRUCT, file, n);
			}
		}

		st->name = name;
		st->nameSpace = ns;
		st->module = module;
		st->compilingDeclaration = decl;
		module->classTypes.PushLast(st);
		module->allLocalTypes.Add(st);
		if (isShared)
		{
			engine->sharedScriptTypes.PushLast(st);
			st->AddRefInternal();
		}
		decl->typeInfo = st;

		// Use the default script class behaviours
		st->beh = engine->scriptTypeBehaviours.beh;

		if (isStruct)
		{
			st->beh.factory = 0;
			st->beh.factories.SetLength(0);
		}
		else
		{
			engine->scriptFunctions[st->beh.factory]->AddRefInternal();
		}

		// TODO: Move this to asCObjectType so that the asCRestore can reuse it
		//engine->scriptFunctions[st->beh.addref]->AddRefInternal();
		//engine->scriptFunctions[st->beh.release]->AddRefInternal();
		//engine->scriptFunctions[st->beh.gcEnumReferences]->AddRefInternal();
		//engine->scriptFunctions[st->beh.gcGetFlag]->AddRefInternal();
		//engine->scriptFunctions[st->beh.gcGetRefCount]->AddRefInternal();
		//engine->scriptFunctions[st->beh.gcReleaseAllReferences]->AddRefInternal();
		//engine->scriptFunctions[st->beh.gcSetFlag]->AddRefInternal();
		engine->scriptFunctions[st->beh.copy]->AddRefInternal();
		engine->scriptFunctions[st->beh.construct]->AddRefInternal();
		// TODO: weak: Should not do this if the class has been declared with noweak
		//engine->scriptFunctions[st->beh.getWeakRefFlag]->AddRefInternal();

		engine->allScriptDeclaredTypes.Add(st);

		// Skip to the content of the class
		while (n && n->nodeType == snIdentifier)
			n = n->next;
	}

	// Register possible child types
	while (n)
	{
		node = n->next;
		if (n->nodeType == snFuncDef)
		{
			n->DisconnectParent();
			if (!decl->isExistingShared)
				RegisterFuncDef(n, file, 0, st);
			else
			{
				// Destroy the node, since it won't be used
				// TODO: Should verify that the funcdef is identical to the one in the existing shared class
			}
		}
		n = node;
	}

	return 0;
}

int asCBuilder::RegisterInterface(asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns)
{
	asCScriptNode *n = node->firstChild;

	bool isShared = false;
	bool isExternal = false;
	/*while( n->nodeType == snIdentifier )
	{
		if (file->TokenEquals(n->tokenPos, n->tokenLength, SHARED_TOKEN))
			isShared = true;
		else if (file->TokenEquals(n->tokenPos, n->tokenLength, EXTERNAL_TOKEN))
			isExternal = true;
		else
			break;
		n = n->next;
	}*/

	int r, c;
	file->ConvertPosToRowCol(n->tokenPos, &r, &c);

	asCString name;
	name.Assign(&file->code[n->tokenPos], n->tokenLength);
	CheckNameConflict(name.AddressOf(), n, file, ns);

	sClassDeclaration *decl = asNEW(sClassDeclaration);
	if( decl == 0 )
	{
		return asOUT_OF_MEMORY;
	}

	interfaceDeclarations.PushLast(decl);
	decl->name             = name;
	decl->script           = file;
	decl->node             = node;

	// External shared interfaces must not try to redefine the interface
	if (isExternal && (n->next == 0 || n->next->tokenType != ttEndStatement) )
	{
		asCString str;
		str.Format(TXT_EXTERNAL_SHARED_s_CANNOT_REDEF, name.AddressOf());
		WriteError(str, file, n);
	}
	else if (!isExternal && n->next && n->next->tokenType == ttEndStatement)
	{
		asCString str;
		str.Format(TXT_MISSING_DEFINITION_OF_s, name.AddressOf());
		WriteError(str, file, n);
	}

	// If this type is shared and there already exist another shared
	// type of the same name, then that one should be used instead of
	// creating a new one.
	if( isShared )
	{
		for( asUINT i = 0; i < engine->sharedScriptTypes.GetLength(); i++ )
		{
			asCObjectType *st = CastToObjectType(engine->sharedScriptTypes[i]);
			if( st &&
				st->IsShared() &&
				st->name == name &&
				st->nameSpace == ns &&
				st->IsInterface() )
			{
				// We'll use the existing type
				decl->isExistingShared = true;
				decl->typeInfo = st;
				module->classTypes.PushLast(st);
				module->allLocalTypes.Add(st);
				st->AddRefInternal();

				// Remember if the interface was declared as external so the saved bytecode can be flagged accordingly
				if (isExternal)
					module->externalTypes.PushLast(st);

				return 0;
			}
		}
	}

	// If the interface was declared as external then it must have been compiled in a different module first
	if (isExternal)
	{
		asCString str;
		str.Format(TXT_EXTERNAL_SHARED_s_NOT_FOUND, name.AddressOf());
		WriteError(str, file, n);
	}

	// Register the object type for the interface
	asCObjectType *st = asNEW(asCObjectType)(engine);
	if( st == 0 )
		return asOUT_OF_MEMORY;

	st->flags = asOBJ_REF | asOBJ_SCRIPT_OBJECT;

	if( isShared )
		st->flags |= asOBJ_SHARED;

	st->size = 0; // Cannot be instantiated
	st->name = name;
	st->nameSpace = ns;
	st->module = module;
	module->classTypes.PushLast(st);
	module->allLocalTypes.Add(st);
	if( isShared )
	{
		engine->sharedScriptTypes.PushLast(st);
		st->AddRefInternal();
	}
	decl->typeInfo = st;

	// Use the default script class behaviours
	st->beh.construct = 0;
	//st->beh.addref = engine->scriptTypeBehaviours.beh.addref;
	//engine->scriptFunctions[st->beh.addref]->AddRefInternal();
	//st->beh.release = engine->scriptTypeBehaviours.beh.release;
	//engine->scriptFunctions[st->beh.release]->AddRefInternal();
	st->beh.copy = 0;

	return 0;
}

void asCBuilder::RegisterGlobalVariables()
{
	sGlobalVariableDescription* prevEnum = nullptr;
	for (int i = 0, Count = globVariableList.GetLength(); i < Count; ++i)
	{
		auto* gvar = globVariableList[i];
		if (gvar->isEnumValue)
		{
			int r;
			if( gvar->initializationNode )
			{
				asCCompiler comp(this);
				asCScriptFunction func(engine, module, asFUNC_SCRIPT);

				// Set the namespace that should be used during the compilation
				func.nameSpace = gvar->datatype.GetTypeInfo()->nameSpace;

				// Temporarily switch the type of the variable to int so it can be compiled properly
				asCDataType saveType;
				saveType = gvar->datatype;
				gvar->datatype = asCDataType::CreatePrimitive(ttInt, true);
				r = comp.CompileGlobalVariable(this, gvar->script, gvar->initializationNode, gvar, &func);
				gvar->datatype = saveType;

				// Make the function a dummy so it doesn't try to release objects while destroying the function
				func.funcType = asFUNC_DUMMY;
			}
			else
			{
				r = 0;

				// When there is no assignment the value is the last + 1
				int enumVal = 0;
				if( prevEnum )
				{
					sGlobalVariableDescription *gvar2 = prevEnum;
					if(gvar2->datatype == gvar->datatype )
					{
						enumVal = int(gvar2->constantValue) + 1;

						if( !gvar2->isCompiled )
						{
							int row, col;
							gvar->script->ConvertPosToRowCol(gvar->declaredAtNode->tokenPos, &row, &col);

							asCString str = gvar->datatype.Format(gvar->nameSpace);
							str += " " + gvar->name;
							str.Format(TXT_COMPILING_s, str.AddressOf());
							WriteInfo(gvar->script->name, str, row, col, true);

							str.Format(TXT_UNINITIALIZED_GLOBAL_VAR_s, gvar2->name.AddressOf());
							WriteError(gvar->script->name, str, row, col);
							r = -1;
						}
					}
				}

				gvar->constantValue = enumVal;
				gvar->property->isPureConstant = true;
				gvar->property->storage = enumVal;
			}

			if( r >= 0 )
			{
				// Set the value as compiled
				gvar->isCompiled = true;
			}

			// Convert enums to true enum values, so subsequent compilations can access it as an enum
			if( gvar->isEnumValue )
			{
				asCEnumType *enumType = CastToEnumType(gvar->datatype.GetTypeInfo());
				asASSERT(NULL != enumType);

				asSEnumValue *e = asNEW(asSEnumValue);
				if( e == 0 )
				{
					// Out of memory
					numErrors++;
					return;
				}

				e->name = gvar->name;
				e->value = int(gvar->constantValue);

				if (e->value > MAX_int8 || e->value < MIN_int8)
				{
					int row, col;
					gvar->script->ConvertPosToRowCol(gvar->declaredAtNode->tokenPos, &row, &col);
					WriteError(gvar->script->name, "Enum value too large to fit inside signed byte", row, col);
					r = -1;
				}

				enumType->enumValues.PushLast(e);
			}

			prevEnum = gvar;
		}
		else if (gvar->property == nullptr)
		{
			asCGlobalProperty *prop = engine->AllocateGlobalProperty();
			prop->name = gvar->name;
			prop->nameSpace = gvar->nameSpace;
			prop->module = module;
			prop->type = gvar->datatype;
			
			gvar->index = prop->id;
			gvar->property = prop;

#if WITH_EDITOR
			if (IsNodeInEditorOnlyCode(gvar->script, gvar->declaredAtNode))
				prop->isEditorOnly = true;
#endif

			if (prop->nameSpace != module->defaultNamespace || prop->name.StartsWith("__StaticType_"))
				engine->allScriptGlobalVariables.Add(prop);

			module->scriptGlobals.Add(prop);
			module->scriptGlobalsList.PushLast(prop);
			prop->AddRef();
		}
	}
}

void asCBuilder::BuildAllocateGlobalVariables()
{
	for (int i = 0, Count = globVariableList.GetLength(); i < Count; ++i)
	{
		auto* gvar = globVariableList[i];
		if( gvar->isCompiled )
			continue;
		if( gvar->isEnumValue )
			continue;

		check(gvar->property != nullptr);
		if (!gvar->property->memoryAllocated)
		{
			gvar->property->AllocateMemory();
			engine->varAddressMap.Add(gvar->property->GetAddressOfValue(), gvar->property);
		}
	};
}

void asCBuilder::CompileGlobalVariables()
{
	TGuardValue<bool> MakeHardDependencies(bValueDependenciesAreHard, true);

	bool compileSucceeded = true;

	// Store state of compilation (errors, warning, output)
	int currNumErrors   = numErrors;
	int currNumWarnings = numWarnings;

	// Backup the original message stream
	bool                       msgCallback     = engine->msgCallback;
	asSSystemFunctionInterface msgCallbackFunc = engine->msgCallbackFunc;
	void                      *msgCallbackObj  = engine->msgCallbackObj;

	// Set the new temporary message stream
	asCOutputBuffer outBuffer;
	engine->SetMessageCallback(asMETHOD(asCOutputBuffer, Callback), &outBuffer, asCALL_THISCALL);

	asCOutputBuffer finalOutput;
	asCScriptFunction *initFunc = 0;

	asCArray<asCGlobalProperty*> initOrder;

	// We first try to compile all the primitive global variables, and only after that
	// compile the non-primitive global variables. This permits the constructors
	// for the complex types to use the already initialized variables of primitive
	// type. Note, we currently don't know which global variables are used in the
	// constructors, so we cannot guarantee that variables of complex types are
	// initialized in the correct order, so we won't reorder those.
	bool compilingPrimitives = true;

	// Compile each global variable
	while( compileSucceeded )
	{
		compileSucceeded = false;

		int accumErrors = 0;
		int accumWarnings = 0;

		// Restore state of compilation
		finalOutput.Clear();
		for (int i = 0, Count = globVariableList.GetLength(); i < Count; ++i)
		{
			auto* gvar = globVariableList[i];
			if( gvar->isCompiled )
				continue;
			check(!gvar->isEnumValue);

			asCByteCode init(this);
			numWarnings = 0;
			numErrors = 0;
			outBuffer.Clear();

			// Skip this for now if we're not compiling complex types yet
			if( compilingPrimitives && !gvar->datatype.IsPrimitive() )
				continue;

			/*if( gvar->declaredAtNode )
			{
				int r, c;
				gvar->script->ConvertPosToRowCol(gvar->declaredAtNode->tokenPos, &r, &c);
				asCString str = gvar->datatype.Format(gvar->ns);
				str += " " + gvar->name;
				str.Format(TXT_COMPILING_s, str.AddressOf());
				WriteInfo(gvar->script->name, str, r, c, true);
			}*/

			// Compile the global variable
			initFunc = asNEW(asCScriptFunction)(engine, module, asFUNC_SCRIPT);
			if( initFunc == 0 )
			{
				// Out of memory
				continue;
			}

			// Set the namespace that should be used for this function
			initFunc->nameSpace = gvar->nameSpace;

			asCCompiler comp(this);
			int r = comp.CompileGlobalVariable(this, gvar->script, gvar->initializationNode, gvar, initFunc);
			if( r >= 0 )
			{
				// Compilation succeeded
				gvar->isCompiled = true;
				compileSucceeded = true;
			}
			else
			{
				// Compilation failed
				initFunc->funcType = asFUNC_DUMMY;
				asDELETE(initFunc, asCScriptFunction);
				initFunc = 0;
			}

			if( gvar->isCompiled )
			{
				// Add warnings for this constant to the total build
				if( numWarnings )
				{
					currNumWarnings += numWarnings;
					if( msgCallback )
						outBuffer.SendToCallback(engine, &msgCallbackFunc, msgCallbackObj);
				}

				// Determine order of variable initializations
				if( gvar->property )
				{
					initOrder.PushLast(gvar->property);
				}

				// Does the function contain more than just a SUSPEND followed by a RET instruction?
				if( initFunc && initFunc->scriptData->byteCode.GetLength() > 2 )
				{
					// Create the init function for this variable
					initFunc->id = engine->GetNextScriptFunctionId();
					engine->AddScriptFunction(initFunc);

					// Finalize the init function for this variable
					initFunc->returnType = asCDataType::CreatePrimitive(ttVoid, false);
					initFunc->scriptData->scriptSectionIdx = engine->GetScriptSectionNameIndex(gvar->script->name.AddressOf());
					if( gvar->declaredAtNode )
					{
						int row, col;
						gvar->script->ConvertPosToRowCol(gvar->declaredAtNode->tokenPos, &row, &col);
						initFunc->scriptData->declaredAt = (row & 0xFFFFF)|((col & 0xFFF)<<20);
					}

					gvar->property->SetInitFunc(initFunc);

					initFunc->ReleaseInternal();
					initFunc = 0;
				}
				else if( initFunc )
				{
					// Destroy the function as it won't be used
					initFunc->funcType = asFUNC_DUMMY;
					asDELETE(initFunc, asCScriptFunction);
					initFunc = 0;
				}
			}
			else
			{
				// Add output to final output
				finalOutput.Append(outBuffer);
				accumErrors += numErrors;
				accumWarnings += numWarnings;
			}

			engine->preMessage.isSet = false;
		}

		if( !compileSucceeded )
		{
			if( compilingPrimitives )
			{
				// No more primitives could be compiled, so
				// switch to compiling the complex variables
				compilingPrimitives = false;
				compileSucceeded    = true;
			}
			else
			{
				// No more variables can be compiled
				// Add errors and warnings to total build
				currNumWarnings += accumWarnings;
				currNumErrors   += accumErrors;
				if( msgCallback )
					finalOutput.SendToCallback(engine, &msgCallbackFunc, msgCallbackObj);
			}
		}
	}

	// Restore states
	engine->msgCallback     = msgCallback;
	engine->msgCallbackFunc = msgCallbackFunc;
	engine->msgCallbackObj  = msgCallbackObj;

	numWarnings = currNumWarnings;
	numErrors   = currNumErrors;

	// Set the correct order of initialization
	if( numErrors == 0 )
	{
		if (module->scriptGlobalsList.GetLength() == initOrder.GetLength())
			module->scriptGlobalsList.SwapWith(initOrder);
	}

	// Delete the enum expressions
	TArray<sGlobalVariableDescription*> RemoveEnums;
	globVariables.IterateAll([&](sGlobalVariableDescription* gvar)
	{
		if( gvar->isEnumValue )
		{
			// Remove from symboltable. This has to be done prior to freeing the memeory
			RemoveEnums.Add(gvar);

			// Destroy the gvar property
			if( gvar->declaredAtNode )
			{
				gvar->declaredAtNode = 0;
			}
			if( gvar->initializationNode )
			{
				gvar->initializationNode = 0;
			}
			if( gvar->property )
			{
				engine->allScriptGlobalVariables.Remove(gvar->property);
				asDELETE(gvar->property, asCGlobalProperty);
				gvar->property = 0;
			}
		}
	});

	for (auto* gvar : RemoveEnums)
	{
		globVariables.Remove(gvar);
		asDELETE(gvar, sGlobalVariableDescription);
	}
}

int asCBuilder::GetNamespaceAndNameFromNode(asCScriptNode *n, asCScriptCode *script, asSNameSpace *implicitNs, asSNameSpace *&outNs, asCString &outName)
{
	// TODO: child funcdef: The node might be a snScope now
	asASSERT( n->nodeType == snIdentifier );

	// Get the optional scope from the node
	// TODO: child funcdef: The parentType will be set if the scope is actually a type rather than a namespace
	asSNameSpace *ns = GetNameSpaceFromNode(n->firstChild, script, implicitNs, 0);
	if( ns == 0 )
		return -1;

	// Get the name
	asCString name(&script->code[n->lastChild->tokenPos], n->lastChild->tokenLength);

	outNs = ns;
	outName = name;

	return 0;
}

void asCBuilder::AddInterfaceFromMixinToClass(sClassDeclaration *decl, asCScriptNode *errNode, sMixinClass *mixin)
{
	// Determine what interfaces that the mixin implements
	asCScriptNode *node = mixin->node;
	asASSERT(node->nodeType == snClass);

	// Skip the name of the mixin
	node = node->firstChild->next;


	while( node && node->nodeType == snIdentifier )
	{
		bool ok = true;
		asSNameSpace *ns;
		asCString name;
		if( GetNamespaceAndNameFromNode(node, mixin->script, mixin->ns, ns, name) < 0 )
			ok = false;
		else
		{
			// Find the object type for the interface
			asCObjectType *objType = GetObjectType(name.AddressOf(), ns);

			// Check that the object type is an interface
			if( objType && objType->IsInterface() )
			{
				// Only add the interface if the class doesn't already implement it
				if( !decl->typeInfo->Implements(objType) )
					AddInterfaceToClass(decl, errNode, objType);
			}
			else
			{
				WriteError(TXT_MIXIN_CLASS_CANNOT_INHERIT, mixin->script, node);
				ok = false;
			}
		}

		if( !ok )
		{
			// Remove this node so the error isn't reported again
			asCScriptNode *delNode = node;
			node = node->prev;
			delNode->DisconnectParent();
		}

		node = node->next;
	}
}

void asCBuilder::AddInterfaceToClass(sClassDeclaration *decl, asCScriptNode *errNode, asCObjectType *intfType)
{
	// A shared type may only implement from shared interfaces
	if( decl->typeInfo->IsShared() && !intfType->IsShared() )
	{
		asCString msg;
		msg.Format(TXT_SHARED_CANNOT_IMPLEMENT_NON_SHARED_s, intfType->name.AddressOf());
		WriteError(msg, decl->script, errNode);
		return;
	}

	if( decl->isExistingShared )
	{
		// If the class is an existing shared class, then just check if the
		// interface exists in the original declaration too
		if( !decl->typeInfo->Implements(intfType) )
		{
			asCString str;
			str.Format(TXT_SHARED_s_DOESNT_MATCH_ORIGINAL, decl->typeInfo->GetName());
			WriteError(str, decl->script, errNode);
			return;
		}
	}
	else
	{
		// If the interface is already in the class then don't add it again
		if( decl->typeInfo->Implements(intfType) )
			return;

		// Add the interface to the class
		CastToObjectType(decl->typeInfo)->interfaces.PushLast(intfType);

		// Add the inherited interfaces too
		// For interfaces this will be done outside to handle out-of-order declarations
		if( !CastToObjectType(decl->typeInfo)->IsInterface() )
		{
			for( asUINT n = 0; n < intfType->interfaces.GetLength(); n++ )
				AddInterfaceToClass(decl, errNode, intfType->interfaces[n]);
		}
	}
}

void asCBuilder::CompileInterfaces()
{
	asUINT n;

	// Order the interfaces with inheritances so that the inherited
	// of inherited interfaces can be added properly
	for( n = 0; n < interfaceDeclarations.GetLength(); n++ )
	{
		sClassDeclaration *intfDecl = interfaceDeclarations[n];
		asCObjectType *intfType = CastToObjectType(intfDecl->typeInfo);

		if( intfType->interfaces.GetLength() == 0 ) continue;

		// If any of the derived interfaces are found after this interface, then move this to the end of the list
		for( asUINT m = n+1; m < interfaceDeclarations.GetLength(); m++ )
		{
			if( intfType->Implements(interfaceDeclarations[m]->typeInfo) )
			{
				interfaceDeclarations.RemoveIndex(n);
				interfaceDeclarations.PushLast(intfDecl);

				// Decrease index so that we don't skip an entry
				n--;
				break;
			}
		}
	}

	// Now recursively add the additional inherited interfaces
	for( n = 0; n < interfaceDeclarations.GetLength(); n++ )
	{
		sClassDeclaration *intfDecl = interfaceDeclarations[n];
		asCObjectType *intfType = CastToObjectType(intfDecl->typeInfo);

		// TODO: Is this really at the correct place? Hasn't the vfTableIdx already been set here?
		// Co-opt the vfTableIdx value in our own methods to indicate the
		// index the function should have in the table chunk for this interface.
		for( asUINT d = 0; d < intfType->methods.GetLength(); d++ )
		{
			asCScriptFunction *func = GetFunctionDescription(intfType->methods[d]);
			func->vfTableIdx = d;

			asASSERT(func->objectType == intfType);
		}

		// As new interfaces will be added to the end of the list, all
		// interfaces will be traversed the same as recursively
		for( asUINT m = 0; m < intfType->interfaces.GetLength(); m++ )
		{
			asCObjectType *base = intfType->interfaces[m];

			// Add any interfaces not already implemented
			for( asUINT l = 0; l < base->interfaces.GetLength(); l++ )
				AddInterfaceToClass(intfDecl, intfDecl->node, base->interfaces[l]);

			// Add the methods from the implemented interface
			for( asUINT l = 0; l < base->methods.GetLength(); l++ )
			{
				// If the derived interface implements the same method, then don't add the base interface' method
				asCScriptFunction *baseFunc = GetFunctionDescription(base->methods[l]);
				asCScriptFunction *derivedFunc = 0;
				bool found = false;
				for( asUINT d = 0; d < intfType->methods.GetLength(); d++ )
				{
					derivedFunc = GetFunctionDescription(intfType->methods[d]);
					if( derivedFunc->IsSignatureEqual(baseFunc) )
					{
						found = true;
						break;
					}
				}

				if( !found )
				{
					// Add the method
					intfType->methods.PushLast(baseFunc->id);
					intfType->methodTable.Add(baseFunc);
					baseFunc->AddRefInternal();
				}
			}
		}
	}
}

void asCBuilder::DetermineTypeRelations()
{
	// Determine inheritance between interfaces
	for (asUINT n = 0; n < interfaceDeclarations.GetLength(); n++)
	{
		sClassDeclaration *intfDecl = interfaceDeclarations[n];
		asCObjectType *intfType = CastToObjectType(intfDecl->typeInfo);

		asCScriptNode *node = intfDecl->node;
		asASSERT(node && node->nodeType == snInterface);
		node = node->firstChild;

		// Skip the 'shared' & 'external' keywords
		/*while( node->nodeType == snIdentifier &&
			   (intfDecl->script->TokenEquals(node->tokenPos, node->tokenLength, SHARED_TOKEN) ||
				intfDecl->script->TokenEquals(node->tokenPos, node->tokenLength, EXTERNAL_TOKEN)) )
			node = node->next;*/

		// Skip the name
		node = node->next;

		// Verify the inherited interfaces
		while (node && node->nodeType == snIdentifier)
		{
			asSNameSpace *ns;
			asCString name;
			if (GetNamespaceAndNameFromNode(node, intfDecl->script, intfType->nameSpace, ns, name) < 0)
			{
				node = node->next;
				continue;
			}

			// Find the object type for the interface
			asCObjectType *objType = 0;
			while (ns)
			{
				objType = GetObjectType(name.AddressOf(), ns);
				if (objType) break;

				ns = engine->GetParentNameSpace(ns);
			}

			// Check that the object type is an interface
			bool ok = true;
			if (objType && objType->IsInterface())
			{
				// Check that the implemented interface is shared if the base interface is shared
				if (intfType->IsShared() && !objType->IsShared())
				{
					asCString str;
					str.Format(TXT_SHARED_CANNOT_IMPLEMENT_NON_SHARED_s, objType->GetName());
					WriteError(str, intfDecl->script, node);
					ok = false;
				}
			}
			else
			{
				WriteError(TXT_INTERFACE_CAN_ONLY_IMPLEMENT_INTERFACE, intfDecl->script, node);
				ok = false;
			}

			if (ok)
			{
				// Make sure none of the implemented interfaces implement from this one
				asCObjectType *base = objType;
				while (base != 0)
				{
					if (base == intfType)
					{
						WriteError(TXT_CANNOT_IMPLEMENT_SELF, intfDecl->script, node);
						ok = false;
						break;
					}

					// At this point there is at most one implemented interface
					if (base->interfaces.GetLength())
						base = base->interfaces[0];
					else
						break;
				}
			}

			if (ok)
				AddInterfaceToClass(intfDecl, node, objType);

			// Remove the nodes so they aren't parsed again
			asCScriptNode *delNode = node;
			node = node->next;
			delNode->DisconnectParent();
		}
	}

	// Determine class inheritances and interfaces
	for (asUINT n = 0; n < classDeclarations.GetLength(); n++)
	{
		sClassDeclaration *decl = classDeclarations[n];
		asCScriptCode *file = decl->script;

		// Find the base class that this class inherits from
		bool multipleInheritance = false;
		asCScriptNode *node = decl->node->firstChild;

		/*while (file->TokenEquals(node->tokenPos, node->tokenLength, FINAL_TOKEN) ||
			file->TokenEquals(node->tokenPos, node->tokenLength, SHARED_TOKEN) ||
			file->TokenEquals(node->tokenPos, node->tokenLength, ABSTRACT_TOKEN) ||
			file->TokenEquals(node->tokenPos, node->tokenLength, TIDY_TOKEN) ||
			file->TokenEquals(node->tokenPos, node->tokenLength, EXTERNAL_TOKEN))
		{
			node = node->next;
		}*/

		// Skip the name of the class
		asASSERT(node->tokenType == ttIdentifier);
		node = node->next;

		while (node && node->nodeType == snIdentifier)
		{
			asSNameSpace *ns;
			asCString name;
			if (GetNamespaceAndNameFromNode(node, file, decl->typeInfo->nameSpace, ns, name) < 0)
			{
				node = node->next;
				continue;
			}

			// Find the object type for the interface
			asCObjectType *objType = 0;
			sMixinClass *mixin = 0;
			asSNameSpace *origNs = ns;
			while (ns)
			{
				objType = GetObjectType(name.AddressOf(), ns);
				if (objType == 0)
					mixin = GetMixinClass(name.AddressOf(), ns);

				if (objType || mixin)
					break;

				ns = engine->GetParentNameSpace(ns);
			}

			if (objType == 0 && mixin == 0)
			{
				asCString str;
				if (origNs->name == "")
					str.Format(TXT_IDENTIFIER_s_NOT_DATA_TYPE_IN_GLOBAL_NS, name.AddressOf());
				else
					str.Format(TXT_IDENTIFIER_s_NOT_DATA_TYPE_IN_NS_s, name.AddressOf(), origNs->name.AddressOf());
				WriteError(str, file, node);
			}
			else if (mixin)
			{
				AddInterfaceFromMixinToClass(decl, node, mixin);
			}
			else if (!(objType->flags & asOBJ_SCRIPT_OBJECT) ||
				(objType->flags & asOBJ_NOINHERIT))
			{
				// Either the class is not a script class or interface
				// or the class has been declared as 'final'
				asCString str;
				str.Format(TXT_CANNOT_INHERIT_FROM_s_FINAL, objType->name.AddressOf());
				WriteError(str, file, node);
			}
			else
			{
				// The class inherits from another script class
				if (!decl->isExistingShared && CastToObjectType(decl->typeInfo)->derivedFrom != 0)
				{
					if (!multipleInheritance)
					{
						WriteError(TXT_CANNOT_INHERIT_FROM_MULTIPLE_CLASSES, file, node);
						multipleInheritance = true;
					}
				}
				else
				{
					// Make sure none of the base classes inherit from this one
					asCObjectType *base = objType;
					bool error = false;
					while (base != 0)
					{
						if (base == decl->typeInfo)
						{
							WriteError(TXT_CANNOT_INHERIT_FROM_SELF, file, node);
							error = true;
							break;
						}

						base = base->derivedFrom;
					}

					if (!error)
					{
						// A shared type may only inherit from other shared types
						if ((decl->typeInfo->IsShared()) && !(objType->IsShared()))
						{
							asCString msg;
							msg.Format(TXT_SHARED_CANNOT_INHERIT_FROM_NON_SHARED_s, objType->name.AddressOf());
							WriteError(msg, file, node);
							error = true;
						}
					}

					if (!error)
					{
						if (decl->isExistingShared)
						{
							// Verify that the base class is the same as the original shared type
							if (CastToObjectType(decl->typeInfo)->derivedFrom != objType)
							{
								asCString str;
								str.Format(TXT_SHARED_s_DOESNT_MATCH_ORIGINAL, decl->typeInfo->GetName());
								WriteError(str, file, node);
							}
						}
						else if (decl->typeInfo->GetFlags() & asOBJ_VALUE)
						{
							asCString str = "Script structs are not allowed to use inheritance.";
							WriteError(str, file, node);
							error = true;
						}
						else
						{
							// Set the base class
							CastToObjectType(decl->typeInfo)->derivedFrom = objType;
							objType->AddRefInternal();
						}
					}
				}
			}

			node = node->next;
		}
	}
}

// numTempl is the number of template instances that existed in the engine before the build begun
void asCBuilder::CompileClasses(asUINT numTempl)
{
	asUINT n;
	asCArray<sClassDeclaration*> toValidate((int)classDeclarations.GetLength());

	for (n = 0; n < classDeclarations.GetLength(); n++)
		CompileClass(classDeclarations[n]);
}

bool asCBuilder::EnsureClassCompiled(asITypeInfo* TypeInfo)
{
	auto flags = TypeInfo->GetFlags();
	if (!(flags & asOBJ_SCRIPT_OBJECT))
		return true;

	auto* objType = (asCObjectType*)(TypeInfo);
	if (objType->compilingDeclaration == nullptr)
		return true;
	if (objType->compilingDeclaration->isResolving)
		return false;
	if (objType->compilingDeclaration->hasResolved)
		return true;

	asCBuilder* otherBuilder = objType->module->builder;
	if (!otherBuilder->scriptsParsed)
		otherBuilder->ParseScripts();

	otherBuilder->CompileClass(objType->compilingDeclaration);

	return true;
}

void asCBuilder::CompileClass(sClassDeclaration* decl)
{
	if (decl->hasResolved)
		return;

	decl->isResolving = true;
	asCObjectType *ot = CastToObjectType(decl->typeInfo);
	bool isStruct = ot->GetFlags() & asOBJ_VALUE;
	if( decl->isExistingShared )
	{
		// Set the declaration as validated already, so that other
		// types that contain this will accept this type
		decl->validState = 1;

		// We'll still validate the declaration to make sure nothing new is
		// added to the shared class that wasn't there in the previous
		// compilation. We do not care if something that is there in the previous
		// declaration is not included in the new declaration though.

		asASSERT( ot->interfaces.GetLength() == ot->interfaceVFTOffsets.GetLength() );
	}

	if (ot->derivedFrom != nullptr)
	{
		if (!EnsureClassCompiled(ot->derivedFrom))
		{
			asCString msg;
			msg.Format("Class %s has a circular inheritance hierarchy.", ot->name.AddressOf());
			WriteError(msg, decl->script, decl->node);

			ot->derivedFrom = nullptr;
		}
		else
		{
			MarkStructuralDependency(ot, ot->derivedFrom, decl->node, decl->script);
		}

		CheckEditorOnlyType(ot->derivedFrom, decl->node, decl->script);
	}

	if (ot->shadowType != nullptr)
	{
		CheckEditorOnlyType((asCObjectType*)ot->shadowType, decl->node, decl->script);
	}

	// Enumerate each of the declared properties
	asCScriptNode *node = decl->node->firstChild->next;

	// Skip list of classes and interfaces
	while( node && node->nodeType == snIdentifier )
		node = node->next;

	while( node && node->nodeType == snDeclaration )
	{
		asCScriptNode *nd = node->firstChild;

		// Is the property declared as private or protected?
		asSAccessSpecifier* accessSpecifier = nullptr;
		bool isPrivate = false, isProtected = false;
		if( nd && nd->tokenType == ttPrivate )
		{
			isPrivate = true;
			nd = nd->next;
		}
		else if( nd && nd->tokenType == ttProtected )
		{
			isProtected = true;
			nd = nd->next;
		}
		else if( nd && nd->tokenType == ttAccess )
		{
			asCString accessName;
			if (auto* specNameNode = nd->firstChild)
			{
				accessName.Assign(&decl->script->code[specNameNode->tokenPos], specNameNode->tokenLength);
				accessSpecifier = ot->GetAccessSpecifier(accessName.AddressOf());
			}

			if (accessSpecifier == nullptr)
			{
				asCString msg;
				msg.Format("Unknown access specifier '%s'", accessName.AddressOf());
				WriteError(msg, decl->script, nd);
			}

			nd = nd->next;
		}

		// Determine the type of the property
		asCScriptCode *file = decl->script;
		asCDataType dt = CreateDataTypeFromNode(nd, file, ot->nameSpace, false, ot);
		//if( dt.IsReadOnly() )
			//WriteError(TXT_PROPERTY_CANT_BE_CONST, file, node);

		// Multiple properties can be declared separated by ,
		nd = nd->next;
		while( nd )
		{
			asCString name(&file->code[nd->tokenPos], nd->tokenLength);

			if (dt.IsObject()
				&& (dt.GetTypeInfo()->flags & asOBJ_VALUE)
				&& (dt.GetTypeInfo()->flags & asOBJ_SCRIPT_OBJECT))
			{
				if (!EnsureClassCompiled(dt.GetTypeInfo()))
				{
					asCString msg;
					msg.Format("Type %s includes a struct that includes itself.", ot->name.AddressOf());
					WriteError(msg, decl->script, decl->node);

					break;
				}
			}

			//CheckNameConflictMember(ot, name.AddressOf(), nd, file, true);
			auto* Property = AddPropertyToClass(decl, name, dt, isPrivate, isProtected, file, nd);
			if (Property != nullptr)
				Property->accessSpecifier = accessSpecifier;

#if WITH_EDITOR
			if (IsNodeInEditorOnlyCode(file, nd))
				Property->isEditorOnly = true;
#endif

			if (auto* TypeInfo = dt.GetTypeInfo())
			{
				if (TypeInfo->flags & asOBJ_VALUE)
					MarkStructuralDependency(ot, TypeInfo, nd, file);
				else
					MarkDependency(TypeInfo, nd, file);
				CheckEditorOnlyType(TypeInfo, nd, file);
			}

			// Skip the initialization node
			if( nd->next && nd->next->nodeType != snIdentifier )
				nd = nd->next;

			nd = nd->next;
		}

		node = node->next;
	}

	// Add properties from included mixin classes that don't conflict with existing properties
	IncludePropertiesFromMixins(decl);

	asASSERT( ot->interfaces.GetLength() == ot->interfaceVFTOffsets.GetLength() );

	decl->isResolving = false;
	decl->hasResolved = true;
}

bool asCBuilder::EnsureClassLayouted(asITypeInfo* TypeInfo)
{
	auto flags = TypeInfo->GetFlags();
	if ((flags & asOBJ_TEMPLATE_SUBTYPE_DETERMINES_SIZE) != 0)
	{
		auto* objType = (asCObjectType*)(TypeInfo);
		if (auto* subType = objType->templateSubTypes[0].GetTypeInfo())
		{
			if ((subType->flags & asOBJ_REF) == 0)
				EnsureClassLayouted(subType);
		}
		objType->CalculateTemplateSize();
		return true;
	}

	if (!(flags & asOBJ_SCRIPT_OBJECT))
		return true;

	auto* objType = (asCObjectType*)(TypeInfo);
	if (objType->compilingDeclaration == nullptr)
		return true;
	if (objType->compilingDeclaration->isLayouting)
		return false;
	if (objType->compilingDeclaration->hasLayouted)
		return true;

	asCBuilder* otherBuilder = objType->module->builder;
	otherBuilder->LayoutClass(objType->compilingDeclaration);

	return true;
}

void asCBuilder::LayoutClass(sClassDeclaration* decl)
{
	if (decl->hasLayouted)
		return;

	decl->isLayouting = true;
	asCObjectType *ot = CastToObjectType(decl->typeInfo);

	bool bIsStruct = (ot->flags & asOBJ_VALUE) != 0;
	bool bIsPOD = bIsStruct;
	bool hasBaseClass = ot->derivedFrom != 0;
	if (hasBaseClass)
		EnsureClassLayouted(ot->derivedFrom);

	// Layout all the properties now that we know the size of the derived class
	if (ot->derivedFrom != nullptr)
	{
		ot->size = ot->derivedFrom->size;
		ot->alignment = FMath::Max(ot->alignment, ot->derivedFrom->alignment);
	}
	else if (ot->shadowType != nullptr)
	{
		ot->size = ot->basePropertyOffset;
		ot->alignment = FMath::Max(ot->alignment, ot->shadowType->alignment);
	}
	else
		ot->size = 0;

	for( asUINT p = 0; p < ot->properties.GetLength(); p++ )
	{
		asCObjectProperty* prop = ot->properties[p];
		if (prop->byteOffset != -1)
			continue;

		if (auto* TypeInfo = prop->type.GetTypeInfo())
		{
			if ((TypeInfo->flags & asOBJ_VALUE)
				&& (TypeInfo->flags & (asOBJ_SCRIPT_OBJECT | asOBJ_TEMPLATE_SUBTYPE_DETERMINES_SIZE)))
			{
				EnsureClassLayouted(TypeInfo);
			}
		}

		// How big is the property?
		int propSize;
		if( prop->type.IsObject() )
		{
			if( prop->type.GetTypeInfo()->flags & asOBJ_VALUE )
				propSize = prop->type.GetSizeInMemoryBytes();
			else
				propSize = prop->type.GetSizeOnStackDWords()*4;
		}
		else
		{
			propSize = prop->type.GetSizeInMemoryBytes();
		}

		// Add extra bytes so that the property will be properly aligned
		asUINT propAlignment = prop->type.GetAlignment();
		const asUINT propSizeAlignmentDifference = ot->size & (propAlignment-1);
		if( propSizeAlignmentDifference != 0 )
			ot->size += (propAlignment - propSizeAlignmentDifference);

		prop->byteOffset = ot->size;
		ot->size += propSize;
	}

	// Check if the struct is POD or not
	if (bIsPOD)
	{
		for( asUINT p = 0; p < ot->properties.GetLength(); p++ )
		{
			asCObjectProperty* prop = ot->properties[p];

			if (auto* TypeInfo = prop->type.GetTypeInfo())
			{
				if ((TypeInfo->flags & asOBJ_VALUE) && !(TypeInfo->flags & asOBJ_POD))
				{
					bIsPOD = false;
					break;
				}
			}
		}

		if (bIsPOD)
		{
			ot->flags |= asOBJ_POD;
		}
	}

	// Add all properties and methods from the base class
	if (ot->derivedFrom != nullptr)
	{
		for (asUINT p = 0; p < ot->derivedFrom->properties.GetLength(); p++)
		{
			ot->properties.PushLast(ot->derivedFrom->properties[p]);
			ot->propertyTable.Add(ot->derivedFrom->properties[p]);
		}
	}

	// Make sure the size of the struct is aligned with its alignment so it works in arrays and stuff
	if (ot->alignment != 1)
		ot->size = (ot->size + ot->alignment - 1) & ~(ot->alignment - 1);

	// Copy methods from base class to derived class
	if (ot->derivedFrom != nullptr)
	{
		check(!bIsStruct);
		ot->virtualFunctionTable = ot->derivedFrom->virtualFunctionTable;

		for( asUINT m = 0; m < ot->derivedFrom->methods.GetLength(); m++ )
		{
			// If the derived class implements the same method, then don't add the base class' method
			asCScriptFunction* baseFunc = GetFunctionDescription(ot->derivedFrom->methods[m]);
			if (baseFunc->funcType == asFUNC_SYSTEM)
				continue;

			asCScriptFunction* derivedFunc = nullptr;
			ot->methodTable.FindAllUntil(
				baseFunc->name.AddressOf(),
				[&](asCScriptFunction* CheckFunction)
				{
					if (!CheckFunction->IsSignatureExceptNameAndReturnTypeEqual(baseFunc) )
						return false;

					if (CheckFunction->returnType != baseFunc->returnType)
					{
						if( baseFunc->name == "opConv" || baseFunc->name == "opImplConv" ||
							baseFunc->name == "opCast" || baseFunc->name == "opImplCast" )
						{
							return false;
						}
						else
						{
							asCString msg;
							msg.Format(TXT_DERIVED_METHOD_MUST_HAVE_SAME_RETTYPE_s, baseFunc->GetDeclaration(true, false, false, true));
							WriteError(msg, decl->script, decl->node);
						}
					}

					if( baseFunc->IsFinal() )
					{
						asCString msg;
						msg.Format(TXT_METHOD_CANNOT_OVERRIDE_s, baseFunc->GetDeclaration(true, false, false, true));
						WriteError(msg, decl->script, decl->node);
					}

					if( baseFunc->IsProperty() != CheckFunction->IsProperty())
					{
						asCString msg;
						msg.Format(TXT_METHOD_s_OVERRIDE_CHANGES_PROPERTY, CheckFunction->GetDeclaration(true, false, false, true));
						WriteError(msg, decl->script, decl->node);
					}

					derivedFunc = CheckFunction;
					return true;
				}
			);

			if (derivedFunc)
			{
				// Place the derived function in the correct vftable slot
				derivedFunc->vfTableIdx = baseFunc->vfTableIdx;
				ot->virtualFunctionTable[baseFunc->vfTableIdx] = derivedFunc;
			}
			else
			{
				// We haven't overridden this, so add it to the derived class' method lists
				ot->methods.PushLast(baseFunc->GetId());
				ot->methodTable.Add(baseFunc);
				baseFunc->AddRefInternal();
			}

		}
	}
	
	for( asUINT m = 0; m < ot->methods.GetLength(); m++ )
	{
		asCScriptFunction* func = GetFunctionDescription(ot->methods[m]);
		if (func->funcType == asFUNC_SYSTEM)
			continue;
		if (func->objectType != ot)
			continue;

		if (func->IsOverride())
		{
			// If we haven't been put in the virtual function table that means we didn't override anything!
			bool bOverrideEditorOnlyIncorrect = false;
#if WITH_EDITOR
			if (func->vfTableIdx != -1)
			{
				if ((unsigned)func->vfTableIdx < ot->derivedFrom->virtualFunctionTable.GetLength())
				{
					asCScriptFunction* parentFunc = ot->derivedFrom->virtualFunctionTable[func->vfTableIdx];
					if (parentFunc && parentFunc->traits.GetTrait(asTRAIT_EDITOR_ONLY) && !func->traits.GetTrait(asTRAIT_EDITOR_ONLY))
					{
						bOverrideEditorOnlyIncorrect = true;
					}
				}
			}
#endif

			if (func->vfTableIdx == -1 || bOverrideEditorOnlyIncorrect)
			{
				asCScriptCode* script = decl->script;
				asCScriptNode* node = decl->node;

				for (asUINT funcIndex = 0; funcIndex < functions.GetLength(); ++funcIndex)
				{
					const auto* FuncDesc = functions[funcIndex];
					if (FuncDesc != nullptr && FuncDesc->objType == ot && FuncDesc->name == func->name)
					{
						if (FuncDesc->script != nullptr && FuncDesc->node != nullptr)
						{
							script = FuncDesc->script;
							node = FuncDesc->node;
						}
						break;
					}
				}

				asCString msg;
				if (bOverrideEditorOnlyIncorrect)
					msg.Format("Method '%s' overrides an editor-only parent function, but is not in editor-only code", func->GetDeclaration(true, false, false, true));
				else
					msg.Format(TXT_METHOD_s_DOES_NOT_OVERRIDE, func->GetDeclaration(true, false, false, true));

				WriteError(msg, script, node);
			}
		}

		if (!bIsStruct)
		{
			// Allocate virtual function table slots for all new methods
			if (func->vfTableIdx == -1)
			{
				func->vfTableIdx = ot->virtualFunctionTable.GetLength();
				ot->virtualFunctionTable.PushLast(func);
			}
		}

		CheckForConflictsDueToDefaultArgs(decl->script, decl->node, func, ot);
	}

	// Refcount all the functions in our virtual function table
	for( asUINT m = 0; m < ot->virtualFunctionTable.GetLength(); m++ )
	{
		if (ot->virtualFunctionTable[m] != nullptr)
			ot->virtualFunctionTable[m]->AddRefInternal();
	}

	// Check for property name conflicts
	int baseOffset = 0;
	if (ot->derivedFrom != nullptr)
		baseOffset = ot->derivedFrom->size;
	else if (ot->shadowType != nullptr)
		baseOffset = ot->basePropertyOffset;

	for( asUINT p = 0; p < ot->properties.GetLength(); p++ )
	{
		asCObjectProperty* prop = ot->properties[p];
		if (prop->byteOffset < baseOffset)
			continue;

		bool bHasConflict = false;
		ot->FindPropertyUntil(prop->name.AddressOf(), [&](asCObjectProperty* OtherProperty)
		{
			if (OtherProperty != prop)
			{
				asCScriptCode* script = decl->script;
				asCScriptNode* node = decl->node;

				for (asUINT propIndex = 0; propIndex < decl->propInits.GetLength(); ++propIndex)
				{
					const auto& Initializer = decl->propInits[propIndex];
					if (Initializer.name == prop->name)
					{
						if (Initializer.declNode != nullptr && Initializer.file != nullptr)
						{
							script = Initializer.file;
							node = Initializer.declNode;
						}
						break;
					}
				}

				asCString str;
				str.Format(TXT_NAME_CONFLICT_s_OBJ_PROPERTY, prop->name.AddressOf());
				WriteError(str, script, node);

				bHasConflict = true;
				return true;
			}
			return false;
		});

		if (bHasConflict)
			continue;

		if (ot->GetFirstMethod_CaseInsensitive(prop->name.AddressOf()) != nullptr)
		{
			asCScriptCode* script = decl->script;
			asCScriptNode* node = decl->node;

			for (asUINT propIndex = 0; propIndex < decl->propInits.GetLength(); ++propIndex)
			{
				const auto& Initializer = decl->propInits[propIndex];
				if (Initializer.name == prop->name)
				{
					if (Initializer.declNode != nullptr && Initializer.file != nullptr)
					{
						script = Initializer.file;
						node = Initializer.declNode;
					}
					break;
				}
			}

			asCString str;
			str.Format(TXT_NAME_CONFLICT_s_METHOD, prop->name.AddressOf());
			WriteError(str, script, node);
		}
	}

	// Done compiling
	decl->isLayouting = false;
	decl->hasLayouted = true;
}

void asCBuilder::LayoutFunction(sFunctionDescription* funcDecl)
{
	asCScriptFunction* func = engine->scriptFunctions[funcDecl->funcId];
	if (func == nullptr)
		return;

	func->CalculateParameterOffsets();
}

void asCBuilder::IncludeMethodsFromMixins(sClassDeclaration *decl)
{
	asCScriptNode *node = decl->node->firstChild;

	// Skip the class attributes
	while( node->nodeType == snIdentifier &&
		   !decl->script->TokenEquals(node->tokenPos, node->tokenLength, decl->name.AddressOf()) )
		node = node->next;

	// Skip the name of the class
	node = node->next;

	// Find the included mixin classes
	while( node && node->nodeType == snIdentifier )
	{
		asSNameSpace *ns;
		asCString name;
		if( GetNamespaceAndNameFromNode(node, decl->script, decl->typeInfo->nameSpace, ns, name) < 0 )
		{
			node = node->next;
			continue;
		}

		sMixinClass *mixin = 0;
		while( ns )
		{
			// Need to make sure the name is not an object type
			asCObjectType *objType = GetObjectType(name.AddressOf(), ns);
			if( objType == 0 )
				mixin = GetMixinClass(name.AddressOf(), ns);

			if( objType || mixin )
				break;

			ns = engine->GetParentNameSpace(ns);
		}

		if( mixin )
		{
			// Find methods from mixin declaration
			asCScriptNode *n = mixin->node->firstChild;

			// Skip to the member declarations
			// Possible keywords 'final' and 'shared' are removed in RegisterMixinClass so we don't need to worry about those here
			while( n && n->nodeType == snIdentifier )
				n = n->next;

			// Add methods from the mixin that are not already existing in the class
			while( n )
			{
				if( n->nodeType == snFunction )
				{
					// Instead of disconnecting the node, we need to clone it, otherwise other
					// classes that include the same mixin will not see the methods
					asCScriptNode *copy = n->CreateCopy(MemStack, engine);

					// Register the method, but only if it doesn't already exist in the class
					RegisterScriptFunctionFromNode(copy, mixin->script, CastToObjectType(decl->typeInfo), false, false, mixin->ns, false, true);
				}
				else if( n->nodeType == snVirtualProperty )
				{
					// TODO: mixin: Support virtual properties too
					WriteError("The virtual property syntax is currently not supported for mixin classes", mixin->script, n);
					//RegisterVirtualProperty(node, decl->script, decl->objType, false, false);
				}

				n = n->next;
			}
		}

		node = node->next;
	}
}

void asCBuilder::IncludePropertiesFromMixins(sClassDeclaration *decl)
{
	asCScriptNode *node = decl->node->firstChild;

	// Skip the class attributes
	while( node->nodeType == snIdentifier &&
		   !decl->script->TokenEquals(node->tokenPos, node->tokenLength, decl->name.AddressOf()) )
		node = node->next;

	// Skip the name of the class
	node = node->next;

	// Find the included mixin classes
	while( node && node->nodeType == snIdentifier )
	{
		asSNameSpace *ns;
		asCString name;
		if( GetNamespaceAndNameFromNode(node, decl->script, decl->typeInfo->nameSpace, ns, name) < 0 )
		{
			node = node->next;
			continue;
		}

		sMixinClass *mixin = 0;
		while( ns )
		{
			// Need to make sure the name is not an object type
			asCObjectType *objType = GetObjectType(name.AddressOf(), ns);
			if( objType == 0 )
				mixin = GetMixinClass(name.AddressOf(), ns);

			if( objType || mixin )
				break;

			ns = engine->GetParentNameSpace(ns);
		}

		if( mixin )
		{
			// Find properties from mixin declaration
			asCScriptNode *n = mixin->node->firstChild;

			// Skip to the member declarations
			// Possible keywords 'final' and 'shared' are removed in RegisterMixinClass so we don't need to worry about those here
			while( n && n->nodeType == snIdentifier )
				n = n->next;

			// Add properties from the mixin that are not already existing in the class
			while( n )
			{
				if( n->nodeType == snDeclaration )
				{
					asSAccessSpecifier* accessSpecifier = nullptr;
					asCScriptNode *n2 = n->firstChild;
					bool isPrivate = false, isProtected = false;
					if( n2 && n2->tokenType == ttPrivate )
					{
						isPrivate = true;
						n2 = n2->next;
					}
					else if( n2 && n2->tokenType == ttProtected )
					{
						isProtected = true;
						n2 = n2->next;
					}
					else if( n2 && n2->tokenType == ttAccess )
					{
						asCString accessName;
						if (auto* specNameNode = n2->firstChild)
						{
							accessName.Assign(&decl->script->code[specNameNode->tokenPos], specNameNode->tokenLength);

							asCObjectType *ot = CastToObjectType(decl->typeInfo);
							accessSpecifier = ot->GetAccessSpecifier(accessName.AddressOf());
						}

						if (accessSpecifier == nullptr)
						{
							asCString msg;
							msg.Format("Unknown access specifier '%s'", accessName.AddressOf());
							WriteError(msg, decl->script, n2);
						}

						n2 = n2->next;
					}

					asCScriptCode *file = mixin->script;
					asCDataType dt = CreateDataTypeFromNode(n2, file, mixin->ns);

					if( decl->typeInfo->IsShared() && dt.GetTypeInfo() && !dt.GetTypeInfo()->IsShared() )
					{
						asCString msg;
						msg.Format(TXT_SHARED_CANNOT_USE_NON_SHARED_TYPE_s, dt.GetTypeInfo()->name.AddressOf());
						WriteError(msg, file, n);
						WriteInfo(TXT_WHILE_INCLUDING_MIXIN, decl->script, node);
					}

					//if( dt.IsReadOnly() )
						//WriteError(TXT_PROPERTY_CANT_BE_CONST, file, n);

					n2 = n2->next;
					while( n2 )
					{
						name.Assign(&file->code[n2->tokenPos], n2->tokenLength);

						// Add the property only if it doesn't already exist in the class
						bool exists = false;
						asCObjectType *ot = CastToObjectType(decl->typeInfo);
						for( asUINT p = 0; p < ot->properties.GetLength(); p++ )
							if( ot->properties[p]->name == name )
							{
								exists = true;
								break;
							}

						if( !exists )
						{
							if( !decl->isExistingShared )
							{
								// It must not conflict with the name of methods
								int r = CheckNameConflictMember(ot, name.AddressOf(), n2, file, true);
								if( r < 0 )
									WriteInfo(TXT_WHILE_INCLUDING_MIXIN, decl->script, node);

								auto* Property = AddPropertyToClass(decl, name, dt, isPrivate, isProtected, file, n2);
								if (Property != nullptr)
									Property->accessSpecifier = accessSpecifier;
							}
							else
							{
								// Verify that the property exists in the original declaration
								bool found = false;
								for( asUINT p = 0; p < ot->properties.GetLength(); p++ )
								{
									asCObjectProperty *prop = ot->properties[p];
									if( prop->isPrivate == isPrivate &&
										prop->isProtected == isProtected &&
										prop->name == name &&
										prop->type == dt )
									{
										found = true;
										break;
									}
								}
								if( !found )
								{
									asCString str;
									str.Format(TXT_SHARED_s_DOESNT_MATCH_ORIGINAL, ot->GetName());
									WriteError(str, decl->script, decl->node);
									WriteInfo(TXT_WHILE_INCLUDING_MIXIN, decl->script, node);
								}
							}
						}

						// Skip the initialization expression
						if( n2->next && n2->next->nodeType != snIdentifier )
							n2 = n2->next;

						n2 = n2->next;
					}
				}

				n = n->next;
			}
		}

		node = node->next;
	}
}

asCObjectProperty *asCBuilder::AddPropertyToClass(sClassDeclaration *decl, const asCString &name, const asCDataType &dt, bool isPrivate, bool isProtected, asCScriptCode *file, asCScriptNode *node)
{
	if( node )
	{
		// Check if the property is allowed
		if( !dt.CanBeInstantiated() )
		{
			if( file && node )
			{
				asCString str;
				if( dt.IsAbstractClass() )
					str.Format(TXT_ABSTRACT_CLASS_s_CANNOT_BE_INSTANTIATED, dt.Format(decl->typeInfo->nameSpace).AddressOf());
				else if( dt.IsInterface() )
					str.Format(TXT_INTERFACE_s_CANNOT_BE_INSTANTIATED, dt.Format(decl->typeInfo->nameSpace).AddressOf());
				else
					// TODO: Improve error message to explain why
					str.Format(TXT_DATA_TYPE_CANT_BE_s, dt.Format(decl->typeInfo->nameSpace).AddressOf());
				WriteError(str, file, node);
			}
			return 0;
		}

		// Register the initialization expression (if any) to be compiled later
		asCScriptNode *declNode = node;
		asCScriptNode *initNode = 0;
		if( node->next && node->next->nodeType != snIdentifier )
		{
			asASSERT( node->next->nodeType == snAssignment );
			initNode = node->next;
		}

		sPropertyInitializer p(name, declNode, initNode, file);
		decl->propInits.PushLast(p);
	}

	// Add the property to the object type
	return CastToObjectType(decl->typeInfo)->AddPropertyToClass(name, dt, isPrivate, isProtected);
}

bool asCBuilder::DoesMethodExist(asCObjectType *objType, int methodId, asUINT *methodIndex)
{
	asCScriptFunction *method = GetFunctionDescription(methodId);

	bool found = false;
	objType->FindMethod(method->name.AddressOf(), [&](asCScriptFunction* m)
	{
		if( m->name           != method->name           ) return;
		if( m->returnType     != method->returnType     ) return;
		if( m->IsReadOnly()   != method->IsReadOnly()   ) return;
		if( m->parameterTypes != method->parameterTypes ) return;
		if( m->inOutFlags     != method->inOutFlags     ) return;

		if( methodIndex )
		{
			for (asUINT n = 0, Cnt = objType->methods.GetLength(); n < Cnt; ++n)
			{
				if (objType->methods[n] == m->id)
				{
					*methodIndex = n;
					break;
				}
			}
		}

		found = true;
	});

	return found;
}

void asCBuilder::AddDefaultConstructor(asCObjectType *objType, asCScriptCode *file)
{
	int funcId = engine->GetNextScriptFunctionId();

	asCDataType returnType = asCDataType::CreatePrimitive(ttVoid, false);
	asCArray<asCDataType> parameterTypes;
	asCArray<asETypeModifiers> inOutFlags;
	asCArray<asCString *> defaultArgs;
	asCArray<asCString> parameterNames;

	// Add the script function
	// TODO: declaredAt should be set to where the class has been declared
	module->AddScriptFunction(file->idx, 0, funcId, objType->name, returnType, parameterTypes, parameterNames, inOutFlags, defaultArgs, false, objType, false, asSFunctionTraits(), objType->nameSpace);

	// Set it as default constructor
	if( objType->beh.construct )
		engine->scriptFunctions[objType->beh.construct]->ReleaseInternal();
	objType->beh.construct = funcId;
	objType->beh.constructors[0] = funcId;
	engine->scriptFunctions[funcId]->AddRefInternal();

	// The bytecode for the default constructor will be generated
	// only after the potential inheritance has been established
	sFunctionDescription *func = asNEW(sFunctionDescription);
	if( func == 0 )
	{
		// Out of memory
		return;
	}

	functions.PushLast(func);

	func->script            = file;
	func->node              = 0;
	func->name              = objType->name;
	func->objType           = objType;
	func->funcId            = funcId;
	func->isExistingShared  = false;

	auto* scriptFunction = engine->scriptFunctions[funcId];
	scriptFunction->traits.SetTrait(asTRAIT_CONSTRUCTOR, true);

	// Add a default factory as well
	if ((objType->flags & asOBJ_VALUE) == 0)
	{
		funcId = engine->GetNextScriptFunctionId();
		if (objType->beh.factory)
			engine->scriptFunctions[objType->beh.factory]->ReleaseInternal();
		objType->beh.factory = funcId;
		objType->beh.factories[0] = funcId;
		returnType = asCDataType::CreateObjectHandle(objType, false);
		// TODO: should be the same as the constructor
		module->AddScriptFunction(file->idx, 0, funcId, objType->name, returnType, parameterTypes, parameterNames, inOutFlags, defaultArgs, false);
		functions.PushLast(0);

		sFactoryDescription factoryDesc;
		factoryDesc.script = file;
		factoryDesc.funcId = funcId;
		factories.PushLast(factoryDesc);

		//asCCompiler compiler(engine);
		//compiler.CompileFactory(this, file, engine->scriptFunctions[funcId]);
		engine->scriptFunctions[funcId]->AddRefInternal();
	}

	// If the object is shared, then the factory must also be marked as shared
	if( objType->flags & asOBJ_SHARED )
		engine->scriptFunctions[funcId]->SetShared(true);
}

void asCBuilder::AddInitDefaultsFunction(asCObjectType *objType, asCScriptCode *file)
{
	int funcId = engine->GetNextScriptFunctionId();

	asCDataType returnType = asCDataType::CreatePrimitive(ttVoid, false);
	asCArray<asCDataType> parameterTypes;
	asCArray<asETypeModifiers> inOutFlags;
	asCArray<asCString *> defaultArgs;
	asCArray<asCString> parameterNames;

	// Add the script function
	// TODO: declaredAt should be set to where the class has been declared
	module->AddScriptFunction(file->idx, 0, funcId, "__InitDefaults", returnType, parameterTypes, parameterNames, inOutFlags, defaultArgs, false, objType, false, asSFunctionTraits(), objType->nameSpace);
	engine->scriptFunctions[funcId]->AddRefInternal();

	// The bytecode for the default constructor will be generated
	// only after the potential inheritance has been established
	sFunctionDescription *func = asNEW(sFunctionDescription);
	if( func == 0 )
	{
		// Out of memory
		return;
	}

	functions.PushLast(func);

	func->script            = file;
	func->node              = 0;
	func->name              = "__InitDefaults";
	func->objType           = objType;
	func->funcId            = funcId;
	func->isExistingShared  = false;

	objType->methods.PushLast(funcId);

	asCScriptFunction *f = engine->scriptFunctions[funcId];
	objType->methodTable.Add(f);
}

void asCBuilder::AddDefaultDestructor(asCObjectType *objType, asCScriptCode *file)
{
	int funcId = engine->GetNextScriptFunctionId();

	asCDataType returnType = asCDataType::CreatePrimitive(ttVoid, false);
	asCArray<asCDataType> parameterTypes;
	asCArray<asETypeModifiers> inOutFlags;
	asCArray<asCString *> defaultArgs;
	asCArray<asCString> parameterNames;

	// Add the script function
	// TODO: declaredAt should be set to where the class has been declared
	asCString funcName = "~";
	funcName += objType->name;

	module->AddScriptFunction(file->idx, 0, funcId, funcName, returnType, parameterTypes, parameterNames, inOutFlags, defaultArgs, false, objType, false, asSFunctionTraits(), objType->nameSpace);

	// Set it as default constructor
	objType->beh.destruct = funcId;
	engine->scriptFunctions[funcId]->AddRefInternal();

	// The bytecode for the default constructor will be generated
	// only after the potential inheritance has been established
	sFunctionDescription *func = asNEW(sFunctionDescription);
	if( func == 0 )
	{
		// Out of memory
		return;
	}

	functions.PushLast(func);

	func->script            = file;
	func->node              = 0;
	func->name              = funcName;
	func->objType           = objType;
	func->funcId            = funcId;
	func->isExistingShared  = false;

	auto* scriptFunction = engine->scriptFunctions[funcId];
	scriptFunction->traits.SetTrait(asTRAIT_DESTRUCTOR, true);

	// If the object is shared, then the factory must also be marked as shared
	if( objType->flags & asOBJ_SHARED )
		engine->scriptFunctions[funcId]->SetShared(true);
}

int asCBuilder::RegisterEnum(asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns)
{
	// Is it a shared enum?
	bool isShared = false;
	bool isExternal = false;
	asCEnumType *existingSharedType = 0;
	asCScriptNode *tmp = node->firstChild;
	/*while( tmp->nodeType == snIdentifier )
	{
		if (file->TokenEquals(tmp->tokenPos, tmp->tokenLength, SHARED_TOKEN))
			isShared = true;
		else if (file->TokenEquals(tmp->tokenPos, tmp->tokenLength, EXTERNAL_TOKEN))
			isExternal = true;
		else
			break;
		tmp = tmp->next;
	}*/

	// Grab the name of the enumeration
	asCString name;
	asASSERT(snDataType == tmp->nodeType);
	asASSERT(snIdentifier == tmp->firstChild->nodeType);
	name.Assign(&file->code[tmp->firstChild->tokenPos], tmp->firstChild->tokenLength);

	if( isShared )
	{
		// Look for a pre-existing shared enum with the same signature
		for( asUINT n = 0; n < engine->sharedScriptTypes.GetLength(); n++ )
		{
			asCTypeInfo *o = engine->sharedScriptTypes[n];
			if( o &&
				o->IsShared() &&
				(o->flags & asOBJ_ENUM) &&
				o->name == name &&
				o->nameSpace == ns )
			{
				existingSharedType = CastToEnumType(o);
				break;
			}
		}
	}

	// If the enum was declared as external then it must have been compiled in a different module first
	if (isExternal && existingSharedType == 0)
	{
		asCString str;
		str.Format(TXT_EXTERNAL_SHARED_s_NOT_FOUND, name.AddressOf());
		WriteError(str, file, tmp);
	}

	// Remember if the type was declared as external so the saved bytecode can be flagged accordingly
	if (isExternal && existingSharedType)
		module->externalTypes.PushLast(existingSharedType);

	// Check the name and add the enum
	int r = CheckNameConflict(name.AddressOf(), tmp->firstChild, file, ns);
	if( asSUCCESS == r )
	{
		asCEnumType *st;

		if( existingSharedType )
		{
			st = existingSharedType;
			st->AddRefInternal();
		}
		else
		{
			st = asNEW(asCEnumType)(engine);
			if( st == 0 )
				return asOUT_OF_MEMORY;

			st->flags     = asOBJ_ENUM;
			if( isShared )
				st->flags |= asOBJ_SHARED;
			st->size      = 1;
			st->alignment = 1;
			st->name      = name;
			st->nameSpace = ns;
			st->module    = module;
		}
		module->enumTypes.PushLast(st);
		module->allLocalTypes.Add(st);

		if( !existingSharedType && isShared )
		{
			engine->sharedScriptTypes.PushLast(st);
			st->AddRefInternal();
		}

		// Store the location of this declaration for reference in name collisions
		sClassDeclaration *decl = asNEW(sClassDeclaration);
		if( decl == 0 )
			return asOUT_OF_MEMORY;

		decl->name             = name;
		decl->script           = file;
		decl->typeInfo         = st;
		namedTypeDeclarations.PushLast(decl);
		engine->allScriptDeclaredTypes.Add(st);

		asCDataType type = CreateDataTypeFromNode(tmp, file, ns);
		asASSERT(!type.IsReference());

		// External shared enums must not redeclare the enum values
		if (isExternal && (tmp->next == 0 || tmp->next->tokenType != ttEndStatement) )
		{
			asCString str;
			str.Format(TXT_EXTERNAL_SHARED_s_CANNOT_REDEF, name.AddressOf());
			WriteError(str, file, tmp);
		}
		else if (!isExternal && tmp->next && tmp->next->tokenType == ttEndStatement)
		{
			asCString str;
			str.Format(TXT_MISSING_DEFINITION_OF_s, name.AddressOf());
			WriteError(str, file, tmp);
		}

		// Register the enum values
		tmp = tmp->next;
		while( tmp && tmp->nodeType == snIdentifier )
		{
			name.Assign(&file->code[tmp->tokenPos], tmp->tokenLength);

			if( existingSharedType )
			{
				// If this is a pre-existent shared enum, then just double check
				// that the value is already defined in the original declaration
				bool found = false;
				for( asUINT n = 0; n < st->enumValues.GetLength(); n++ )
					if( st->enumValues[n]->name == name )
					{
						found = true;
						break;
					}

				if( !found )
				{
					asCString str;
					str.Format(TXT_SHARED_s_DOESNT_MATCH_ORIGINAL, st->GetName());
					WriteError(str, file, tmp);
					break;
				}

				tmp = tmp->next;
				if( tmp && tmp->nodeType == snAssignment )
					tmp = tmp->next;
				continue;
			}
			else
			{
				// Check for name conflict errors with other values in the enum
				bool bCollision = false;
				globVariables.FindAllUntil(name.AddressOf(), ns, [&](sGlobalVariableDescription* Other)
				{
					if (Other->datatype == type)
					{
						bCollision = true;
						return true;
					}
					return false;
				});

				if(bCollision)
				{
					asCString str;
					str.Format(TXT_NAME_CONFLICT_s_ALREADY_USED, name.AddressOf());
					WriteError(str, file, tmp);

					tmp = tmp->next;
					if( tmp && tmp->nodeType == snAssignment )
						tmp = tmp->next;
					continue;
				}

				// Check for assignment
				asCScriptNode *asnNode = tmp->next;
				if( asnNode && snAssignment == asnNode->nodeType )
					asnNode->DisconnectParent();
				else
					asnNode = 0;

				// Create the global variable description so the enum value can be evaluated
				sGlobalVariableDescription *gvar = asNEW(sGlobalVariableDescription);
				if( gvar == 0 )
					return asOUT_OF_MEMORY;

				gvar->script             = file;
				gvar->declaredAtNode     = tmp;
				tmp = tmp->next;
				gvar->declaredAtNode->DisconnectParent();
				gvar->initializationNode = asnNode;
				gvar->name               = name;
				gvar->datatype           = type;
				gvar->nameSpace                 = ns;
				// No need to allocate space on the global memory stack since the values are stored in the asCObjectType
				// Set the index to a negative to allow compiler to diferentiate from ordinary global var when compiling the initialization
				gvar->index              = -1;
				gvar->isCompiled         = false;
				gvar->isPureConstant     = true;
				gvar->isEnumValue        = true;
				gvar->constantValue      = 0xdeadbeef;

				// Allocate dummy property so we can compile the value.
				// This will be removed later on so we don't add it to the engine.
				gvar->property = asNEW(asCGlobalProperty);
				if( gvar->property == 0 )
					return asOUT_OF_MEMORY;

				gvar->property->name      = name;
				gvar->property->nameSpace = ns;
				gvar->property->type      = gvar->datatype;
				gvar->property->id        = 0;

				globVariables.Add(gvar);
				globVariableList.PushLast(gvar);
				engine->allScriptGlobalVariables.Add(gvar->property);
			}
		}
	}

	return r;
}

int asCBuilder::RegisterTypedef(asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns)
{
	// Get the native data type
	asCScriptNode *tmp = node->firstChild;
	asASSERT(NULL != tmp && snDataType == tmp->nodeType);
	asCDataType dataType;
	dataType.CreatePrimitive(tmp->tokenType, false);
	dataType.SetTokenType(tmp->tokenType);
	tmp = tmp->next;

	// Grab the name of the typedef
	asASSERT(NULL != tmp && NULL == tmp->next);
	asCString name;
	name.Assign(&file->code[tmp->tokenPos], tmp->tokenLength);

	// If the name is not already in use add it
 	int r = CheckNameConflict(name.AddressOf(), tmp, file, ns);

	asCTypedefType *st = 0;
	if( asSUCCESS == r )
	{
		// Create the new type
		st = asNEW(asCTypedefType)(engine);
		if( st == 0 )
			r = asOUT_OF_MEMORY;
	}

	if( asSUCCESS == r )
	{
		st->flags           = asOBJ_TYPEDEF;
		st->size            = dataType.GetSizeInMemoryBytes();
		st->name            = name;
		st->nameSpace       = ns;
		st->aliasForType    = dataType;
		st->module          = module;

		module->typeDefs.PushLast(st);
		module->allLocalTypes.Add(st);

		// Store the location of this declaration for reference in name collisions
		sClassDeclaration *decl = asNEW(sClassDeclaration);
		if( decl == 0 )
			r = asOUT_OF_MEMORY;
		else
		{
			decl->name             = name;
			decl->script           = file;
			decl->typeInfo         = st;
			namedTypeDeclarations.PushLast(decl);
		}
	}

	return r;
}

void asCBuilder::GetParsedFunctionDetails(asCScriptNode *node, asCScriptCode *file, asCObjectType *objType, asCString &name, asCDataType &returnType, asCArray<asCString> &parameterNames, asCArray<asCDataType> &parameterTypes, asCArray<asETypeModifiers> &inOutFlags, asCArray<asCString *> &defaultArgs, asSFunctionTraits &funcTraits, asSNameSpace *implicitNamespace, asCString* accessSpecifier)
{
	node = node->firstChild;

	// Is the function shared?
	funcTraits.SetTrait(asTRAIT_SHARED, false);
	funcTraits.SetTrait(asTRAIT_EXTERNAL, false);
	funcTraits.SetTrait(asTRAIT_LOCAL, false);
	/*while (node->tokenType == ttIdentifier)
	{
		if (file->TokenEquals(node->tokenPos, node->tokenLength, SHARED_TOKEN))
			funcTraits.SetTrait(asTRAIT_SHARED, true);
		else if (file->TokenEquals(node->tokenPos, node->tokenLength, EXTERNAL_TOKEN))
			funcTraits.SetTrait(asTRAIT_EXTERNAL, true);
		else
			break;
		node = node->next;
	}*/
	if (node->tokenType == ttMixin)
	{
		funcTraits.SetTrait(asTRAIT_MIXIN, true);
		node = node->next;
	}
	if (node->tokenType == ttLocal)
	{
		funcTraits.SetTrait(asTRAIT_LOCAL, true);
		node = node->next;
	}

	// Is the function a private or protected class method?
	funcTraits.SetTrait(asTRAIT_PRIVATE, false);
	funcTraits.SetTrait(asTRAIT_PROTECTED, false);
	if( node->tokenType == ttPrivate )
	{
		funcTraits.SetTrait(asTRAIT_PRIVATE, true);
		node = node->next;
	}
	else if( node->tokenType == ttProtected )
	{
		funcTraits.SetTrait(asTRAIT_PROTECTED, true);
		node = node->next;
	}
	else if( node->tokenType == ttAccess )
	{
		if (accessSpecifier != nullptr)
		{
			if (auto* specNameNode = node->firstChild)
				accessSpecifier->Assign(&file->code[specNameNode->tokenPos], specNameNode->tokenLength);
		}

		node = node->next;
	}

	// Find the name
	funcTraits.SetTrait(asTRAIT_CONSTRUCTOR, false);
	funcTraits.SetTrait(asTRAIT_DESTRUCTOR, false);
	asCScriptNode *n = 0;
	if( node->nodeType == snDataType )
		n = node->next->next;
	else
	{
		// If the first node is a ~ token, then we know it is a destructor
		if( node->tokenType == ttBitNot )
		{
			n = node->next;
			funcTraits.SetTrait(asTRAIT_DESTRUCTOR, true);
		}
		else
		{
			n = node;
			funcTraits.SetTrait(asTRAIT_CONSTRUCTOR, true);
		}
	}
	name.Assign(&file->code[n->tokenPos], n->tokenLength);

	if( !funcTraits.GetTrait(asTRAIT_CONSTRUCTOR) && !funcTraits.GetTrait(asTRAIT_DESTRUCTOR) )
	{
		returnType = CreateDataTypeFromNode(node, file, implicitNamespace, false, objType);
		returnType = ModifyDataTypeFromNode(returnType, node->next, file, 0, 0);

		if( engine->ep.disallowValueAssignForRefType &&
			returnType.GetTypeInfo() &&
			(returnType.GetTypeInfo()->flags & asOBJ_REF) &&
			!(returnType.GetTypeInfo()->flags & asOBJ_SCOPED) &&
			!returnType.IsReference() &&
			!returnType.IsObjectHandle() )
		{
			WriteError(TXT_REF_TYPE_CANT_BE_RETURNED_BY_VAL, file, node);
		}
	}
	else
		returnType = asCDataType::CreatePrimitive(ttVoid, false);

	funcTraits.SetTrait(asTRAIT_CONST, false);
	funcTraits.SetTrait(asTRAIT_FINAL, false);
	funcTraits.SetTrait(asTRAIT_OVERRIDE, false);
	funcTraits.SetTrait(asTRAIT_PROPERTY, false);
	funcTraits.SetTrait(asTRAIT_NODISCARD, false);
	funcTraits.SetTrait(asTRAIT_ALLOWDISCARD, false);

	if( n->next->next )
	{
		asCScriptNode *decorator = n->next->next;

		// Is this a const method?
		if( objType && decorator->tokenType == ttConst )
		{
			funcTraits.SetTrait(asTRAIT_CONST, true);
			decorator = decorator->next;
		}

		while( decorator )
		{
			if( objType && decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, FINAL_TOKEN) )
				funcTraits.SetTrait(asTRAIT_FINAL, true);
			else if( objType && decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, OVERRIDE_TOKEN) )
				funcTraits.SetTrait(asTRAIT_OVERRIDE, true);
			else if( decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, PROPERTY_TOKEN) ) // Property token can also appear on global functions
				funcTraits.SetTrait(asTRAIT_PROPERTY, true);
			else if( objType && decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, ACCEPT_TEMPORARY_TOKEN) )
				funcTraits.SetTrait(asTRAIT_ACCEPT_TEMPORARY_OBJECT, true);
			else if( !objType && decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, EXTERNAL_IMPLICIT_THIS_TOKEN) )
				funcTraits.SetTrait(asTRAIT_EXTERNAL_IMPLICIT_THIS, true);
			else if( decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, NO_DISCARD_TOKEN) )
				funcTraits.SetTrait(asTRAIT_NODISCARD, true);
			else if( decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, ALLOW_DISCARD_TOKEN) )
				funcTraits.SetTrait(asTRAIT_ALLOWDISCARD, true);
			else if( decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, GENERATED_FUNCTION_TOKEN) )
				funcTraits.SetTrait(asTRAIT_GENERATED_FUNCTION, true);
			else if( decorator->tokenType == ttIdentifier && file->TokenEquals(decorator->tokenPos, decorator->tokenLength, DEPRECATED_TOKEN) )
				funcTraits.SetTrait(asTRAIT_DEPRECATED, true);

			decorator = decorator->next;
		}
	}

	// Count the number of parameters
	int count = 0;
	asCScriptNode *c = n->next->firstChild;
	while( c )
	{
		count++;
		c = c->next->next;
		if( c && c->nodeType == snIdentifier )
			c = c->next;
		if( c && c->nodeType == snExpression )
			c = c->next;
	}

	// Get the parameter types
	parameterNames.Allocate(count, false);
	parameterTypes.Allocate(count, false);
	inOutFlags.Allocate(count, false);
	defaultArgs.Allocate(count, false);
	n = n->next->firstChild;
	while( n )
	{
		asETypeModifiers inOutFlag;
		asCDataType type = CreateDataTypeFromNode(n, file, implicitNamespace, false, objType);
		type = ModifyDataTypeFromNode(type, n->next, file, &inOutFlag, 0);

		if( engine->ep.disallowValueAssignForRefType &&
			type.GetTypeInfo() &&
			(type.GetTypeInfo()->flags & asOBJ_REF) &&
			!(type.GetTypeInfo()->flags & asOBJ_SCOPED) &&
			!type.IsReference() &&
			!type.IsObjectHandle() )
		{
			WriteError(TXT_REF_TYPE_CANT_BE_PASSED_BY_VAL, file, node);
		}

		if (!type.IsReference())
		{
			// Script functions that take a value type by value should always take a const ref instead
			auto* typeInfo = type.GetTypeInfo();
			if (typeInfo != nullptr && typeInfo->GetFlags() & asOBJ_VALUE)
			{
				type.MakeReference(true);
				type.MakeReadOnly(true);
				inOutFlag = asETypeModifiers::asTM_INOUTREF;
			}

			// Script functions that take a primitive type by value should make it const, for consistency
			// so parameters can never be changed.
			if (type.IsPrimitive() && !type.IsReadOnly())
			{
				type.MakeReadOnly(true);
			}
		}

		// Store the parameter type
		parameterTypes.PushLast(type);
		inOutFlags.PushLast(inOutFlag);

		// Move to next parameter
		n = n->next->next;
		if( n && n->nodeType == snIdentifier )
		{
			asCString paramName(&file->code[n->tokenPos], n->tokenLength);
			parameterNames.PushLast(paramName);
			n = n->next;
		}
		else
		{
			// No name was given for the parameter
			parameterNames.PushLast(asCString());
		}

		if( n && n->nodeType == snExpression )
		{
			// Strip out white space and comments to better share the string
			asCString *defaultArgStr = asNEW(asCString);
			if( defaultArgStr )
				*defaultArgStr = GetCleanExpressionString(n, file);
			defaultArgs.PushLast(defaultArgStr);

			n = n->next;
		}
		else
			defaultArgs.PushLast(0);
	}
}
#endif

asCString asCBuilder::GetCleanExpressionString(asCScriptNode *node, asCScriptCode *file)
{
	asASSERT(node && node->nodeType == snExpression);

	asCString str;
	str.Assign(file->code + node->tokenPos, node->tokenLength);

	asCString cleanStr;
	for( asUINT n = 0; n < str.GetLength(); )
	{
		asUINT len = 0;
		asETokenClass tok = engine->ParseToken(str.AddressOf() + n, str.GetLength() - n, &len);
		if( tok != asTC_COMMENT && tok != asTC_WHITESPACE )
		{
			if( cleanStr.GetLength() ) cleanStr += " ";
			cleanStr.Concatenate(str.AddressOf() + n, len);
		}
		n += len;
	}

	return cleanStr;
}

#ifndef AS_NO_COMPILER
int asCBuilder::RegisterScriptFunctionFromNode(asCScriptNode *node, asCScriptCode *file, asCObjectType *objType, bool isInterface, bool isGlobalFunction, asSNameSpace *ns, bool isExistingShared, bool isMixin)
{
	asCString                  name;
	asCDataType                returnType;
	asCArray<asCString>        parameterNames;
	asCArray<asCDataType>      parameterTypes;
	asCArray<asETypeModifiers> inOutFlags;
	asCArray<asCString *>      defaultArgs;
	asSFunctionTraits          funcTraits;

	asASSERT( (objType && ns == 0) || isGlobalFunction || isMixin );

	// Set the default namespace
	if( ns == 0 )
	{
		if( objType )
			ns = objType->nameSpace;
		else
			ns = engine->nameSpaces[0];
	}

	asCString accessSpecifier;
	GetParsedFunctionDetails(node, file, objType, name, returnType, parameterNames, parameterTypes, inOutFlags, defaultArgs, funcTraits, ns, &accessSpecifier);

	return RegisterScriptFunction(node, file, objType, isInterface, isGlobalFunction, ns, isExistingShared, isMixin, name, returnType, parameterNames, parameterTypes, inOutFlags, defaultArgs, funcTraits, &accessSpecifier);
}

asCScriptFunction *asCBuilder::RegisterLambda(asCScriptNode *node, asCScriptCode *file, asCScriptFunction *funcDef, const asCString &name, asSNameSpace *ns)
{
	// Get the parameter names from the node
	asCArray<asCString> parameterNames;
	asCArray<asCString*> defaultArgs;
	asCScriptNode *args = node->firstChild;
	while( args && args->nodeType != snStatementBlock )
	{
		if (args->nodeType == snIdentifier)
		{
			asCString argName;
			argName.Assign(&file->code[args->tokenPos], args->tokenLength);
			parameterNames.PushLast(argName);
			defaultArgs.PushLast(0);
		}
		args = args->next;
	}

	// The statement block for the function must be disconnected, as the builder is going to be the owner of it
	args->DisconnectParent();

	// Get the return and parameter types from the funcDef
	asCString funcName = name;
	int r = RegisterScriptFunction(args, file, 0, 0, true, ns, false, false, funcName, funcDef->returnType, parameterNames, funcDef->parameterTypes, funcDef->inOutFlags, defaultArgs, asSFunctionTraits());
	if( r < 0 )
		return 0;

	// Return the function that was just created (but that will be compiled later)
	return engine->scriptFunctions[functions[functions.GetLength()-1]->funcId];
}

int asCBuilder::RegisterScriptFunction(asCScriptNode *node, asCScriptCode *file, asCObjectType *objType, bool isInterface, bool isGlobalFunction, asSNameSpace *ns, bool isExistingShared, bool isMixin, asCString &name, asCDataType &returnType, asCArray<asCString> &parameterNames, asCArray<asCDataType> &parameterTypes, asCArray<asETypeModifiers> &inOutFlags, asCArray<asCString *> &defaultArgs, asSFunctionTraits funcTraits, asCString* accessSpecifier)
{
	// Determine default namespace if not specified
	if( ns == 0 )
	{
		if( objType )
			ns = objType->nameSpace;
		else
			ns = engine->nameSpaces[0];
	}

	if( isExistingShared )
	{
		asASSERT( objType );

		// Should validate that the function really exists in the class/interface
		bool found = false;
		if(funcTraits.GetTrait(asTRAIT_CONSTRUCTOR) || funcTraits.GetTrait(asTRAIT_DESTRUCTOR) )
		{
			// TODO: shared: Should check the existance of these too
			found = true;
		}
		else
		{
			objType->FindMethod(name.AddressOf(), [&](asCScriptFunction* func)
			{
				if(func->IsSignatureExceptNameEqual(returnType, parameterTypes, inOutFlags, objType, funcTraits.GetTrait(asTRAIT_CONST)) )
				{
					// Add the shared function in this module too
					module->AddScriptFunction(func);

					found = true;
				}
			});
		}

		if( !found )
		{
			asCString str;
			str.Format(TXT_SHARED_s_DOESNT_MATCH_ORIGINAL, objType->GetName());
			WriteError(str, file, node);
		}

		// Free the default args
		for( asUINT n = 0; n < defaultArgs.GetLength(); n++ )
			if( defaultArgs[n] )
				asDELETE(defaultArgs[n], asCString);

		return 0;
	}

	// Check for name conflicts
	if( !funcTraits.GetTrait(asTRAIT_CONSTRUCTOR) && !funcTraits.GetTrait(asTRAIT_DESTRUCTOR) )
	{
		if( objType )
		{
			CheckNameConflictMember(objType, name.AddressOf(), node, file, false);

			if( name == objType->name )
				WriteError(TXT_METHOD_CANT_HAVE_NAME_OF_CLASS, file, node);
		}
		else
			CheckNameConflict(name.AddressOf(), node, file, ns);
	}
	else
	{
		if( isMixin )
		{
			// Mixins cannot implement constructors/destructors
			WriteError(TXT_MIXIN_CANNOT_HAVE_CONSTRUCTOR, file, node);

			// Free the default args
			for( asUINT n = 0; n < defaultArgs.GetLength(); n++ )
				if( defaultArgs[n] )
					asDELETE(defaultArgs[n], asCString);

			return 0;
		}

		// Verify that the name of the constructor/destructor is the same as the class
		if( name != objType->name )
		{
			asCString str;
			if(funcTraits.GetTrait(asTRAIT_DESTRUCTOR) )
				str.Format(TXT_DESTRUCTOR_s_s_NAME_ERROR, objType->name.AddressOf(), name.AddressOf());
			else
				str.Format(TXT_METHOD_s_s_HAS_NO_RETURN_TYPE, objType->name.AddressOf(), name.AddressOf());
			WriteError(str, file, node);
		}

		if(funcTraits.GetTrait(asTRAIT_DESTRUCTOR))
			name = "~" + name;
	}

	// Validate virtual properties signature
	if (funcTraits.GetTrait(asTRAIT_PROPERTY))
	{
		asCScriptFunction func(engine, module, asFUNC_SCRIPT);
		func.name = name;
		func.nameSpace = ns;
		func.objectType = objType;
		if (objType)
			objType->AddRefInternal();
		func.traits = funcTraits;
		func.returnType = returnType;
		func.parameterTypes = parameterTypes;

		int r = ValidatePropertyAccessorFunc(&func);
		if (r < 0)
		{
			asCString str;
			WriteError(TXT_INVALID_SIG_FOR_PROP_ACCESSOR, file, node);
		}

		func.funcType = asFUNC_DUMMY;
	}

	isExistingShared = false;
	int funcId = engine->GetNextScriptFunctionId();
	if( !isInterface )
	{
		sFunctionDescription *func = asNEW(sFunctionDescription);
		if( func == 0 )
		{
			// Free the default args
			for( asUINT n = 0; n < defaultArgs.GetLength(); n++ )
				if( defaultArgs[n] )
					asDELETE(defaultArgs[n], asCString);

			return asOUT_OF_MEMORY;
		}

		functions.PushLast(func);

		func->script            = file;
		func->node              = node;
		func->name              = name;
		func->objType           = objType;
		func->funcId            = funcId;
		func->isExistingShared  = false;
		func->paramNames        = parameterNames;

		if(funcTraits.GetTrait(asTRAIT_SHARED))
		{
			// Look for a pre-existing shared function with the same signature
			for( asUINT n = 0; n < engine->scriptFunctions.GetLength(); n++ )
			{
				asCScriptFunction *f = engine->scriptFunctions[n];
				if( f &&
					f->IsShared() &&
					f->name == name &&
					f->nameSpace == ns &&
					f->objectType == objType &&
					f->IsSignatureExceptNameEqual(returnType, parameterTypes, inOutFlags, 0, false) )
				{
					funcId = func->funcId = f->id;
					isExistingShared = func->isExistingShared = true;
					break;
				}
			}
		}

		// Remember if the function was declared as external so the saved bytecode can be flagged accordingly
		if (funcTraits.GetTrait(asTRAIT_EXTERNAL) && func->isExistingShared)
			module->externalFunctions.PushLast(engine->scriptFunctions[func->funcId]);

		if (funcTraits.GetTrait(asTRAIT_EXTERNAL) && !func->isExistingShared)
		{
			// Mark it as existing shared to avoid compiling it
			func->isExistingShared = true;

			asCString str;
			str.Format(TXT_EXTERNAL_SHARED_s_NOT_FOUND, name.AddressOf());
			WriteError(str, file, node);
		}

		// External shared function must not try to redefine the interface
		if (funcTraits.GetTrait(asTRAIT_EXTERNAL) && !(node->tokenType == ttEndStatement || node->lastChild->tokenType == ttEndStatement))
		{
			asCString str;
			str.Format(TXT_EXTERNAL_SHARED_s_CANNOT_REDEF, name.AddressOf());
			WriteError(str, file, node);
		}
		else if (!funcTraits.GetTrait(asTRAIT_EXTERNAL) && !(node->nodeType == snStatementBlock || node->lastChild->nodeType == snStatementBlock) )
		{
			asCString str;
			str.Format(TXT_MISSING_DEFINITION_OF_s, name.AddressOf());
			WriteError(str, file, node);
		}
	}

	// Destructors may not have any parameters
	if (funcTraits.GetTrait(asTRAIT_DESTRUCTOR) && parameterTypes.GetLength() > 0)
		WriteError(TXT_DESTRUCTOR_MAY_NOT_HAVE_PARM, file, node);

	// If a function, class, or interface is shared then only shared types may be used in the signature
	if( (objType && objType->IsShared()) || funcTraits.GetTrait(asTRAIT_SHARED))
	{
		asCTypeInfo *ti = returnType.GetTypeInfo();
		if( ti && !ti->IsShared() )
		{
			asCString msg;
			msg.Format(TXT_SHARED_CANNOT_USE_NON_SHARED_TYPE_s, ti->name.AddressOf());
			WriteError(msg, file, node);
		}

		for( asUINT p = 0; p < parameterTypes.GetLength(); ++p )
		{
			ti = parameterTypes[p].GetTypeInfo();
			if( ti && !ti->IsShared() )
			{
				asCString msg;
				msg.Format(TXT_SHARED_CANNOT_USE_NON_SHARED_TYPE_s, ti->name.AddressOf());
				WriteError(msg, file, node);
			}
		}
	}

	// Check that the same function hasn't been registered already in the namespace
	asCArray<int> funcs;
	if( objType )
		GetObjectMethodDescriptions(name.AddressOf(), objType, funcs, false);
	else
		GetFunctionDescriptions(name.AddressOf(), funcs, ns);
	if( objType && (name == "opConv" || name == "opImplConv" || name == "opCast" || name == "opImplCast") && parameterTypes.GetLength() == 0 )
	{
		// opConv and opCast are special methods used for type casts
		for( asUINT n = 0; n < funcs.GetLength(); ++n )
		{
			asCScriptFunction *func = GetFunctionDescription(funcs[n]);
			if( func->IsSignatureExceptNameEqual(returnType, parameterTypes, inOutFlags, objType, funcTraits.GetTrait(asTRAIT_CONST)) )
			{
				// TODO: clean up: Reuse the same error handling for both opConv and normal methods
				if( isMixin )
				{
					// Clean up the memory, as the function will not be registered
					sFunctionDescription *funcDesc = functions.PopLast();
					asDELETE(funcDesc, sFunctionDescription);

					// Free the default args
					for( n = 0; n < defaultArgs.GetLength(); n++ )
						if( defaultArgs[n] )
							asDELETE(defaultArgs[n], asCString);

					return 0;
				}

				if (func->funcType == asFUNC_SYSTEM)
					WriteError(TXT_FUNCTION_ALREADY_EXIST_SYSTEM, file, node);
				else
					WriteError(TXT_FUNCTION_ALREADY_EXIST, file, node);
				break;
			}
		}
	}
	else
	{
		for( asUINT n = 0; n < funcs.GetLength(); ++n )
		{
			asCScriptFunction *func = GetFunctionDescription(funcs[n]);
			if( func->IsSignatureExceptNameAndReturnTypeEqual(parameterTypes, inOutFlags, objType, funcTraits.GetTrait(asTRAIT_CONST)) )
			{
				if( isMixin )
				{
					// Clean up the memory, as the function will not be registered
					sFunctionDescription *funcDesc = functions.PopLast();
					asDELETE(funcDesc, sFunctionDescription);

					// Free the default args
					for( n = 0; n < defaultArgs.GetLength(); n++ )
						if( defaultArgs[n] )
							asDELETE(defaultArgs[n], asCString);

					return 0;
				}

				if (func->funcType == asFUNC_SYSTEM)
					WriteError(TXT_FUNCTION_ALREADY_EXIST_SYSTEM, file, node);
				else
					WriteError(TXT_FUNCTION_ALREADY_EXIST, file, node);
				break;
			}
		}
	}

	// Register the function
	if( isExistingShared )
	{
		// Delete the default args as they won't be used anymore
		for( asUINT n = 0; n < defaultArgs.GetLength(); n++ )
			if( defaultArgs[n] )
				asDELETE(defaultArgs[n], asCString);

		asCScriptFunction *f = engine->scriptFunctions[funcId];
		module->AddScriptFunction(f);

		// TODO: clean up: This should be done by AddScriptFunction() itself
		module->globalFunctions.Add(f);
		module->globalFunctionList.PushLast(f);
	}
	else
	{
		int row = 0, col = 0;
		if( node )
			file->ConvertPosToRowCol(node->tokenPos, &row, &col);
		module->AddScriptFunction(file->idx, (row&0xFFFFF)|((col&0xFFF)<<20), funcId, name, returnType, parameterTypes, parameterNames, inOutFlags, defaultArgs, isInterface, objType, isGlobalFunction, funcTraits, ns);
	}

	// Make sure the default args are declared correctly
	ValidateDefaultArgs(file, node, engine->scriptFunctions[funcId]);
	CheckForConflictsDueToDefaultArgs(file, node, engine->scriptFunctions[funcId], objType);

#if WITH_EDITOR
	if (IsNodeInEditorOnlyCode(file, node))
		engine->scriptFunctions[funcId]->traits.SetTrait(asTRAIT_EDITOR_ONLY, true);
#endif

	if( objType )
	{
		asASSERT( !isExistingShared );

		engine->scriptFunctions[funcId]->AddRefInternal();
		if(funcTraits.GetTrait(asTRAIT_CONSTRUCTOR))
		{
			if ((objType->flags & asOBJ_VALUE) != 0)
			{
				if (parameterTypes.GetLength() == 0)
				{
					// Overload the default constructor
					engine->scriptFunctions[objType->beh.construct]->ReleaseInternal();
					objType->beh.construct = funcId;
					objType->beh.constructors[0] = funcId;
				}
				else
				{
					objType->beh.constructors.PushLast(funcId);
				}
			}
			else
			{
				int factoryId = engine->GetNextScriptFunctionId();
				if (parameterTypes.GetLength() == 0)
				{
					// Overload the default constructor
					engine->scriptFunctions[objType->beh.construct]->ReleaseInternal();
					objType->beh.construct = funcId;
					objType->beh.constructors[0] = funcId;

					// Register the default factory as well
					engine->scriptFunctions[objType->beh.factory]->ReleaseInternal();
					objType->beh.factory = factoryId;
					objType->beh.factories[0] = factoryId;
				}
				else
				{
					objType->beh.constructors.PushLast(funcId);

					// Register the factory as well
					objType->beh.factories.PushLast(factoryId);
				}

				// We must copy the default arg strings to avoid deleting the same object multiple times
				for (asUINT n = 0; n < defaultArgs.GetLength(); n++)
					if (defaultArgs[n])
						defaultArgs[n] = asNEW(asCString)(*defaultArgs[n]);

				asCDataType dt = asCDataType::CreateObjectHandle(objType, false);
				module->AddScriptFunction(file->idx, engine->scriptFunctions[funcId]->scriptData->declaredAt, factoryId, name, dt, parameterTypes, parameterNames, inOutFlags, defaultArgs, false, 0, false, funcTraits);

				// If the object is shared, then the factory must also be marked as shared
				if (objType->flags & asOBJ_SHARED)
					engine->scriptFunctions[factoryId]->SetShared(true);

				// Add a dummy function to the builder so that it doesn't mix up the fund Ids
				functions.PushLast(0);

				sFactoryDescription factoryDesc;
				factoryDesc.script = file;
				factoryDesc.funcId = factoryId;
				factories.PushLast(factoryDesc);

				// Compile the factory immediately
				//asCCompiler compiler(engine);
				//compiler.CompileFactory(this, file, engine->scriptFunctions[factoryId]);

				engine->scriptFunctions[factoryId]->AddRefInternal();
			}
		}
		else if(funcTraits.GetTrait(asTRAIT_DESTRUCTOR))
			objType->beh.destruct = funcId;
		else
		{
			// If the method is the assignment operator we need to replace the default implementation
			asCScriptFunction *f = engine->scriptFunctions[funcId];
			if( f->name == "opAssign" && f->parameterTypes.GetLength() == 1 &&
				f->parameterTypes[0].GetTypeInfo() == f->objectType &&
				(f->inOutFlags[0] & asTM_INREF) )
			{
				engine->scriptFunctions[objType->beh.copy]->ReleaseInternal();
				objType->beh.copy = funcId;
				f->AddRefInternal();
			}

			// Lookup the access specifier
			if (accessSpecifier != nullptr && accessSpecifier->GetLength() != 0)
			{
				f->accessSpecifier = objType->GetAccessSpecifier(accessSpecifier->AddressOf());
				if (f->accessSpecifier == nullptr)
				{
					asCString str;
					str.Format("Unknown access specifier %s on function %s", accessSpecifier->AddressOf(), name.AddressOf());
					WriteError(str, file, node);
				}
			}

			objType->methods.PushLast(funcId);
			objType->methodTable.Add(f);
		}
	}
	else
	{
		engine->allScriptGlobalFunctions.Add(engine->scriptFunctions[funcId]);
	}

	return 0;
}

int asCBuilder::RegisterVirtualProperty(asCScriptNode *node, asCScriptCode *file, asCObjectType *objType, bool isInterface, bool isGlobalFunction, asSNameSpace *ns, bool isExistingShared)
{
	if( engine->ep.propertyAccessorMode < 2 )
	{
		WriteError(TXT_PROPERTY_ACCESSOR_DISABLED, file, node);
		return 0;
	}

	asASSERT( (objType && ns == 0) || isGlobalFunction );

	if( ns == 0 )
	{
		if( objType )
			ns = objType->nameSpace;
		else
			ns = engine->nameSpaces[0];
	}

	bool isPrivate = false, isProtected = false;
	asCString emulatedName;
	asCDataType emulatedType;

	asCScriptNode *mainNode = node;
	asSAccessSpecifier* accessSpecifier = nullptr;
	node = node->firstChild;

	if( !isGlobalFunction && node->tokenType == ttPrivate )
	{
		isPrivate = true;
		node = node->next;
	}
	else if( !isGlobalFunction && node->tokenType == ttProtected )
	{
		isProtected = true;
		node = node->next;
	}
	else if( !isGlobalFunction && node->tokenType == ttAccess )
	{
		asCString accessName;
		if (auto* specNameNode = node->firstChild)
		{
			accessName.Assign(&file->code[specNameNode->tokenPos], specNameNode->tokenLength);

			accessSpecifier = objType->GetAccessSpecifier(accessName.AddressOf());
		}

		if (accessSpecifier == nullptr)
		{
			asCString msg;
			msg.Format("Unknown access specifier '%s'", accessName.AddressOf());
			WriteError(msg, file, node);
		}

		node = node->next;
	}

	emulatedType = CreateDataTypeFromNode(node, file, ns);
	emulatedType = ModifyDataTypeFromNode(emulatedType, node->next, file, 0, 0);
	node = node->next->next;
	emulatedName.Assign(&file->code[node->tokenPos], node->tokenLength);

	if( node->next == 0 )
		WriteError(TXT_PROPERTY_WITHOUT_ACCESSOR, file, node);

	node = node->next;
	while (node)
	{
		asCScriptNode             *next = node->next;
		asCScriptNode             *funcNode = 0;
		bool                       success = false;
		asSFunctionTraits          funcTraits;
		asCDataType                returnType;
		asCArray<asCString>        paramNames;
		asCArray<asCDataType>      paramTypes;
		asCArray<asETypeModifiers> paramModifiers;
		asCArray<asCString*>       defaultArgs;
		asCString                  name;

		funcTraits.SetTrait(asTRAIT_PRIVATE, isPrivate);
		funcTraits.SetTrait(asTRAIT_PROTECTED, isProtected);

		// Virtual property accessor methods are implicitly marked as property accessors
		funcTraits.SetTrait(asTRAIT_PROPERTY, true);

		// TODO: getset: Allow private for individual property accessors
		// TODO: getset: If the accessor uses its own name, then the property should be automatically declared

		if (node->firstChild->nodeType == snIdentifier && file->TokenEquals(node->firstChild->tokenPos, node->firstChild->tokenLength, GET_TOKEN))
			name = AS_GET_PREFIX;
		else if (node->firstChild->nodeType == snIdentifier && file->TokenEquals(node->firstChild->tokenPos, node->firstChild->tokenLength, SET_TOKEN))
			name = AS_SET_PREFIX;
		else
			WriteError(TXT_UNRECOGNIZED_VIRTUAL_PROPERTY_NODE, file, node);

		if (name != "")
		{
			success = true;
			funcNode = node->firstChild->next;

			if (funcNode && funcNode->tokenType == ttConst)
			{
				funcTraits.SetTrait(asTRAIT_CONST, true);
				funcNode = funcNode->next;
			}

			while (funcNode && funcNode->nodeType != snStatementBlock)
			{
				if (funcNode->tokenType == ttIdentifier && file->TokenEquals(funcNode->tokenPos, funcNode->tokenLength, FINAL_TOKEN))
					funcTraits.SetTrait(asTRAIT_FINAL, true);
				else if (funcNode->tokenType == ttIdentifier && file->TokenEquals(funcNode->tokenPos, funcNode->tokenLength, OVERRIDE_TOKEN))
					funcTraits.SetTrait(asTRAIT_OVERRIDE, true);

				funcNode = funcNode->next;
			}

			if (funcNode)
				funcNode->DisconnectParent();

			if (funcNode == 0 && (objType == 0 || !objType->IsInterface()))
			{
				// TODO: getset: If no implementation is supplied the builder should provide an automatically generated implementation
				//               The compiler needs to be able to handle the different types, primitive, value type, and handle
				//               The code is also different for global property accessors
				WriteError(TXT_PROPERTY_ACCESSOR_MUST_BE_IMPLEMENTED, file, node);
			}

			if (name == AS_GET_PREFIX)
			{
				// Setup the signature for the get accessor method
				returnType = emulatedType;
				name = AS_GET_PREFIX + emulatedName;
			}
			else if (name == AS_SET_PREFIX)
			{
				// Setup the signature for the set accessor method
				returnType = asCDataType::CreatePrimitive(ttVoid, false);
				paramModifiers.PushLast(asTM_NONE);
				paramNames.PushLast("value");
				paramTypes.PushLast(emulatedType);
				defaultArgs.PushLast(0);
				name = AS_SET_PREFIX + emulatedName;
			}
		}

		if( success )
		{
			if (!isExistingShared)
			{
				asCString funcName(name);
				RegisterScriptFunction(funcNode, file, objType, isInterface, isGlobalFunction, ns, false, false, funcName, returnType, paramNames, paramTypes, paramModifiers, defaultArgs, funcTraits);
			}
			else
			{
				// Should validate that the function really exists in the class/interface
				bool found = false;
				objType->FindMethod(name.AddressOf(), [&](asCScriptFunction* func)
				{
					if(func->IsSignatureExceptNameEqual(returnType, paramTypes, paramModifiers, objType, funcTraits.GetTrait(asTRAIT_CONST)) )
					{
						found = true;
					}
				});

				if( !found )
				{
					asCString str;
					str.Format(TXT_SHARED_s_DOESNT_MATCH_ORIGINAL, objType->GetName());
					WriteError(str, file, node);
				}
			}
		}

		node = next;
	};

	return 0;
}

int asCBuilder::RegisterAccessSpecifier(asCScriptNode* node, asCScriptCode* file, asCObjectType* objType)
{
	asCScriptNode* mainNode = node;

	asCScriptNode* nameNode = node->firstChild;
	if (nameNode == nullptr)
	{
		return 0;
	}

	asCString name;
	name.Assign(&file->code[nameNode->tokenPos], nameNode->tokenLength);

	// Make sure we don't already have an access specifier with this name
	for (asUINT n = 0, count = objType->accessSpecifiers.GetLength(); n < count; ++n)
	{
		if (objType->accessSpecifiers[n]->name == name)
		{
			asCString str;
			str.Format("Access specifier %s is already declared.", name.AddressOf());
			WriteError(str, file, node);

			return 0;
		}
	}

	asSAccessSpecifier* spec = asNEW(asSAccessSpecifier);
	spec->name = name;

	asCScriptNode* specNode = nameNode->next;
	while (specNode != nullptr)
	{
		if (specNode->tokenType == ttPrivate)
		{
			spec->bIsPrivate = true;
		}
		else if (specNode->tokenType == ttProtected)
		{
			spec->bIsProtected = true;
		}
		else if (specNode->tokenType == ttStar)
		{
			asCScriptNode* modNode = specNode->firstChild;
			while (modNode != nullptr)
			{
				if (file->TokenEquals(modNode->tokenPos, modNode->tokenLength, READONLY_TOKEN))
				{
					spec->bAnyReadOnly = true;
				}
				else if (file->TokenEquals(modNode->tokenPos, modNode->tokenLength, EDITDEFAULTS_TOKEN))
				{
					spec->bAnyEditDefaults = true;
				}

				modNode = modNode->next;
			}
		}
		else if (specNode->firstChild && specNode->firstChild->tokenType == ttIdentifier)
		{
			asSAccessPermission perm;
			perm.accessName.Assign(&file->code[specNode->firstChild->tokenPos], specNode->firstChild->tokenLength);

			asCScriptNode* modNode = specNode->firstChild->next;
			while (modNode != nullptr)
			{
				if (file->TokenEquals(modNode->tokenPos, modNode->tokenLength, READONLY_TOKEN))
				{
					perm.bReadOnly = true;
				}
				else if (file->TokenEquals(modNode->tokenPos, modNode->tokenLength, EDITDEFAULTS_TOKEN))
				{
					perm.bEditDefaults = true;
				}
				else if (file->TokenEquals(modNode->tokenPos, modNode->tokenLength, INHERITED_TOKEN))
				{
					perm.bInherited = true;
				}

				modNode = modNode->next;
			}

			spec->permissions.PushLast(perm);
		}

		specNode = specNode->next;
	}

	objType->accessSpecifiers.PushLast(spec);

	return 0;
}

int asCBuilder::RegisterImportedFunction(int importID, asCScriptNode *node, asCScriptCode *file, asSNameSpace *ns)
{
	asCString                  name;
	asCDataType                returnType;
	asCArray<asCString>        parameterNames;
	asCArray<asCDataType>      parameterTypes;
	asCArray<asETypeModifiers> inOutFlags;
	asCArray<asCString *>      defaultArgs;
	asSFunctionTraits          funcTraits;

	if( ns == 0 )
		ns = engine->nameSpaces[0];

	// Function imports are meaningless when automatic imports are on
	if (engine->ep.automaticImports)
	{
		return 0;
	}

	GetParsedFunctionDetails(node->firstChild, file, 0, name, returnType, parameterNames, parameterTypes, inOutFlags, defaultArgs, funcTraits, ns);
	CheckNameConflict(name.AddressOf(), node, file, ns);

	// Check that the same function hasn't been registered already in the namespace
	asCArray<int> funcs;
	GetFunctionDescriptions(name.AddressOf(), funcs, ns);
	for( asUINT n = 0; n < funcs.GetLength(); ++n )
	{
		asCScriptFunction *func = GetFunctionDescription(funcs[n]);
		if( func->IsSignatureExceptNameAndReturnTypeEqual(parameterTypes, inOutFlags, 0, false) )
		{
			WriteError(TXT_FUNCTION_ALREADY_EXIST, file, node);
			break;
		}
	}

	// Read the module name as well
	asCScriptNode *nd = node->lastChild;
	asASSERT( nd->nodeType == snConstant && nd->tokenType == ttStringConstant );
	asCString moduleName;
	moduleName.Assign(&file->code[nd->tokenPos+1], nd->tokenLength-2);

	// Register the function
	module->AddImportedFunction(importID, name, returnType, parameterTypes, inOutFlags, defaultArgs, ns, moduleName);

	return 0;
}

asCScriptFunction *asCBuilder::GetFunctionDescription(int id)
{
	// TODO: import: This should be improved when the imported functions are removed
	// Get the description from the engine
	if( (id & FUNC_IMPORTED) == 0 )
		return engine->scriptFunctions[id];
	else
		return engine->importedFunctions[id & ~FUNC_IMPORTED]->importedFunctionSignature;
}

void asCBuilder::GetFunctionDescriptions(const char *name, asCArray<int> &funcs, asSNameSpace *ns, bool lookForMixins)
{
	asUINT n;

	if (engine->ep.automaticImports)
	{
		engine->allScriptGlobalFunctions.FindAll(name, ns, [&](asCScriptFunction* checkFunc)
		{
			if (lookForMixins != checkFunc->IsMixin())
				return;
			if (checkFunc->traits.GetTrait(asTRAIT_LOCAL) && checkFunc->module != module)
				return;
			funcs.PushLast(checkFunc->id);
		});
	}
	else
	{
		module->FindGlobalFunctions(name, ns, funcs, lookForMixins);

		// Add the imported functions
		// TODO: optimize: Linear search: This is probably not that critial. Also bindInformation will probably be removed in near future
		if (!lookForMixins)
		{
			for (n = 0; n < module->bindInformations.GetLength(); n++)
			{
				if (module->bindInformations[n]->importedFunctionSignature->name == name &&
					module->bindInformations[n]->importedFunctionSignature->nameSpace == ns)
					funcs.PushLast(module->bindInformations[n]->importedFunctionSignature->id);
			}
		}
	}

	// Add the registered global functions
	engine->registeredGlobalFuncTable.FindAll(name, ns, [&](asCScriptFunction* f)
	{
		if (lookForMixins == f->IsMixin())
		{
			funcs.PushLast(f->id);
		}
	});
}

// scope is only informed when looking for a base class' method
void asCBuilder::GetObjectMethodDescriptions(const char *name, asCObjectType *objectType, asCArray<int> &methods, bool objIsConst, const asCString &scope, asCScriptNode *errNode, asCScriptCode *script, bool bConstDueToTemporary)
{
	asASSERT(objectType);

	bool bAllowSystemFunctions = true;
	if( scope != "" )
	{
		// If searching with a scope informed, then the node and script must also be informed for potential error reporting
		asASSERT( errNode && script );

		// If the scope contains ::identifier, then use the last identifier as the class name and the rest of it as the namespace
		// TODO: child funcdef: A scope can include a template type, e.g. array<ns::type>
		int n = scope.FindLast("::");
		asCString className = n >= 0 ? scope.SubString(n+2) : scope;
		asCString nsName = n >= 0 ? scope.SubString(0, n) : "";

		// If a namespace was specifically defined, then this must be used
		asSNameSpace *ns = 0;
		if (n >= 0)
		{
			if (nsName == "")
				ns = engine->nameSpaces[0];
			else
				ns = GetNameSpaceByString(nsName, objectType->nameSpace, errNode, script, 0, false);

			// If the namespace isn't found return silently and let the calling
			// function report the error if it cannot resolve the symbol
			if (ns == 0)
				return;
		}

		// Find the base class with the specified scope
		if (nsName == "Super" || className == "Super")
		{
			objectType = objectType->derivedFrom;
			bAllowSystemFunctions = false;
		}
		else
		{
			while (objectType)
			{
				// If the name and namespace matches it is the correct class. If no
				// specific namespace was given, then don't compare the namespace
				if (objectType->name == className && (ns == 0 || objectType->nameSpace == ns))
					break;

				objectType = objectType->derivedFrom;
			}
		}

		// If the scope is not any of the base classes, then return no methods
		if( objectType == 0 )
			return;
	}
		
	// If we're searching with an explicit scope, search for _Implementation functions first,
	// since those are often the correct ones if they exist.
	bool bFoundImplementation = false;
	if (scope.GetLength() != 0)
	{
		objectType->FindMethod((asCString(name)+ "_Implementation").AddressOf(), [&](asCScriptFunction* func)
		{
			if (!func->IsReadOnly())
			{
				if (objIsConst)
					return;
				if (bConstDueToTemporary && !func->traits.GetTrait(asTRAIT_ACCEPT_TEMPORARY_OBJECT))
					return;
			}
			if (!bAllowSystemFunctions && func->funcType == asFUNC_SYSTEM)
				return;

			methods.PushLast(func->id);
			bFoundImplementation = true;
		});
	}

	// Find the methods in the object that match the name
	if (!bFoundImplementation)
	{
		objectType->FindMethod(name, [&](asCScriptFunction* func)
		{
			if (!func->IsReadOnly())
			{
				if (objIsConst)
					return;
				if (bConstDueToTemporary && !func->traits.GetTrait(asTRAIT_ACCEPT_TEMPORARY_OBJECT))
					return;
			}
			if (!bAllowSystemFunctions && func->funcType == asFUNC_SYSTEM)
				return;

			methods.PushLast(func->id);
		});
	}
}
#endif

void asCBuilder::WriteInfo(const asCString &scriptname, const asCString &message, int r, int c, bool pre)
{
	// Need to store the pre message in a structure
	if( pre )
	{
		engine->preMessage.isSet      = true;
		engine->preMessage.c          = c;
		engine->preMessage.r          = r;
		engine->preMessage.message    = message;
		engine->preMessage.scriptname = scriptname;
	}
	else
	{
		engine->preMessage.isSet = false;

		if( !silent )
			engine->WriteMessage(scriptname.AddressOf(), r, c, asMSGTYPE_INFORMATION, message.AddressOf());
	}
}

void asCBuilder::WriteInfo(const asCString &message, asCScriptCode *file, asCScriptNode *node)
{
	int r = 0, c = 0;
	if( node )
		file->ConvertPosToRowCol(node->tokenPos, &r, &c);

	WriteInfo(file->name, message, r, c, false);
}

void asCBuilder::WriteError(const asCString &message, asCScriptCode *file, asCScriptNode *node)
{
	int r = 0, c = 0;
	if( node && file )
		file->ConvertPosToRowCol(node->tokenPos, &r, &c);

	WriteError(file ? file->name : asCString(""), message, r, c);
}

void asCBuilder::WriteError(const asCString &scriptname, const asCString &message, int r, int c)
{
	numErrors++;

	if( !silent )
		engine->WriteMessage(scriptname.AddressOf(), r, c, asMSGTYPE_ERROR, message.AddressOf());
}

void asCBuilder::WriteWarning(const asCString &scriptname, const asCString &message, int r, int c)
{
	if( engine->ep.compilerWarnings )
	{
		numWarnings++;

		if( !silent )
			engine->WriteMessage(scriptname.AddressOf(), r, c, asMSGTYPE_WARNING, message.AddressOf());
	}
}

void asCBuilder::WriteWarning(const asCString &message, asCScriptCode *file, asCScriptNode *node)
{
	int r = 0, c = 0;
	if( node && file )
		file->ConvertPosToRowCol(node->tokenPos, &r, &c);

	WriteWarning(file ? file->name : asCString(""), message, r, c);
}

// TODO: child funcdef: Should try to eliminate this function. GetNameSpaceFromNode is more complete
asCString asCBuilder::GetScopeFromNode(asCScriptNode *node, asCScriptCode *script, asCScriptNode **next)
{
	if (node->nodeType != snScope)
	{
		if (next)
			*next = node;
		return "";
	}

	asCString scope;
	asCScriptNode *sn = node->firstChild;
	if( sn->tokenType == ttScope )
	{
		scope = "::";
		sn = sn->next;
	}

	// TODO: child funcdef: A scope can have a template type as the innermost
	while( sn && sn->next && sn->next->tokenType == ttScope )
	{
		asCString tmp;
		tmp.Assign(&script->code[sn->tokenPos], sn->tokenLength);
		if( scope != "" && scope != "::" )
			scope += "::";
		scope += tmp;
		sn = sn->next->next;
	}

	if( next )
		*next = node->next;

	return scope;
}

asSNameSpace *asCBuilder::GetNameSpaceFromNode(asCScriptNode *node, asCScriptCode *script, asSNameSpace *implicitNs, asCScriptNode **next, asCObjectType **objType)
{
	if (objType)
		*objType = 0;

	// If no scope has been informed, then return the implicit namespace
	if (node->nodeType != snScope)
	{
		if (next)
			*next = node;
		return implicitNs ? implicitNs : engine->nameSpaces[0];
	}

	if (next)
		*next = node->next;

	asCString scope;
	asCScriptNode *sn = node->firstChild;
	if (sn && sn->tokenType == ttScope)
	{
		scope = "::";
		sn = sn->next;
	}

	while (sn)
	{
		if (sn->next->tokenType == ttScope)
		{
			asCString tmp;
			tmp.Assign(&script->code[sn->tokenPos], sn->tokenLength);
			if (scope != "" && scope != "::")
				scope += "::";
			scope += tmp;
			sn = sn->next->next;
		}
		else
		{
			// This is a template type
			asASSERT(sn->next->nodeType == snDataType);

			asSNameSpace *ns = implicitNs;
			if (scope != "")
				ns = engine->FindNameSpace(scope.AddressOf());

			asCString templateName(&script->code[sn->tokenPos], sn->tokenLength);
			asCObjectType *templateType = GetObjectType(templateName.AddressOf(), ns);
			if (templateType == 0 || (templateType->flags & asOBJ_TEMPLATE) == 0)
			{
				// TODO: child funcdef: Report error
				return ns;
			}

			if (objType)
				*objType = GetTemplateInstanceFromNode(sn, script, templateType, implicitNs, 0);

			// Return no namespace, since this is an object type
			return 0;
		}
	}

	asCTypeInfo *ti = 0;
	asSNameSpace *ns = GetNameSpaceByString(scope, implicitNs ? implicitNs : engine->nameSpaces[0], node, script, &ti);
	if (ti && objType)
		*objType = CastToObjectType(ti);
	return ns;
}

asSNameSpace *asCBuilder::GetNameSpaceByString(const asCString &nsName, asSNameSpace *implicitNs, asCScriptNode *errNode, asCScriptCode *script, asCTypeInfo **scopeType, bool isRequired)
{
	if( scopeType )
		*scopeType = 0;

	asSNameSpace *ns = implicitNs;
	if( nsName == "::" )
		ns = engine->nameSpaces[0];
	else if( nsName != "" )
	{
		asSNameSpace* insideNs = implicitNs;

		ns = nullptr;
		while (ns == nullptr && insideNs != nullptr)
		{
			if (insideNs == engine->nameSpaces[0])
			{
				ns = engine->FindNameSpace(nsName.AddressOf());
			}
			else
			{
				asCString checkNsName = insideNs->name + "::" + nsName;
				ns = engine->FindNameSpace(checkNsName.AddressOf());
			}

			insideNs = engine->GetParentNameSpace(insideNs);
		}

		if (ns == 0 && scopeType)
		{
			asCString typeName;
			asCString searchNs;

			// Split the scope with at the inner most ::
			int pos = nsName.FindLast("::");
			bool recursive = false;
			if (pos >= 0)
			{
				// Fully qualified namespace
				typeName = nsName.SubString(pos + 2);
				searchNs = nsName.SubString(0, pos);
			}
			else
			{
				// Partially qualified, use the implicit namespace and then search recursively for the type
				typeName = nsName;
				searchNs = implicitNs->name;
				recursive = true;
			}

			asSNameSpace *nsTmp = searchNs == "::" ? engine->nameSpaces[0] : engine->FindNameSpace(searchNs.AddressOf());
			asCTypeInfo *ti = 0;
			while( !ti && nsTmp )
			{
				// Check if the typeName is an existing type in the namespace
				ti = GetType(typeName.AddressOf(), nsTmp, 0);
				if (ti)
				{
					// The informed scope is not a namespace, but it does match a type
					*scopeType = ti;
					return 0;
				}
				nsTmp = recursive ? engine->GetParentNameSpace(nsTmp) : 0;
			}
		}

		if (ns == 0 && isRequired)
		{
			asCString msg;
			msg.Format(TXT_NAMESPACE_s_DOESNT_EXIST, nsName.AddressOf());
			WriteError(msg, script, errNode);
		}
	}

	return ns;
}

asCDataType asCBuilder::CreateDataTypeFromNode(asCScriptNode *node, asCScriptCode *file, asSNameSpace *implicitNamespace, bool acceptHandleForScope, asCObjectType *currentType, bool reportError, bool *isValid)
{
	asASSERT(node->nodeType == snDataType || node->nodeType == snIdentifier || node->nodeType == snScope );

	asCDataType dt;

	asCScriptNode *n = node->firstChild;

	if (isValid)
		*isValid = true;

	// If the informed node is an identifier or scope, then the
	// datatype should be identified directly from that
	if (node->nodeType != snDataType)
		n = node;

	bool isConst = false;
	bool isImplicitHandle = false;
	if( n->tokenType == ttConst )
	{
		isConst = true;
		n = n->next;
	}

	// Determine namespace (or parent type) to search for the data type in
	asCObjectType *parentType = 0;
	asSNameSpace *ns = GetNameSpaceFromNode(n, file, implicitNamespace, &n, &parentType);
	if( ns == 0 && parentType == 0 )
	{
		// The namespace and parent type doesn't exist. Return a dummy type instead.
		dt = asCDataType::CreatePrimitive(ttInt, false);
		dt.SetIsErrorDataType(true);
		if (isValid)
			*isValid = false;
		return dt;
	}

	if( n->tokenType == ttIdentifier )
	{
		bool found = false;

		asCString str;
		str.Assign(&file->code[n->tokenPos], n->tokenLength);

		// Recursively search parent namespaces for matching type
		asSNameSpace *origNs = ns;
		asCObjectType *origParentType = parentType;
		while( (ns || parentType) && !found )
		{
			asCTypeInfo *ti = 0;

			if (currentType)
			{
				// If this is for a template type, then we must first determine if the
				// identifier matches any of the template subtypes
				if (currentType->flags & asOBJ_TEMPLATE)
				{
					for (asUINT subtypeIndex = 0; subtypeIndex < currentType->templateSubTypes.GetLength(); subtypeIndex++)
					{
						asCTypeInfo *type = currentType->templateSubTypes[subtypeIndex].GetTypeInfo();
						if (type && str == type->name)
						{
							ti = type;
							break;
						}
					}
				}

				if (ti == 0)
				{
					// Check if the type is a child type of the current type
					ti = GetFuncDef(str.AddressOf(), 0, currentType);
					if (ti)
					{
						dt = asCDataType::CreateType(ti, false);
						found = true;
					}
				}
			}

			if( ti == 0 )
				ti = GetType(str.AddressOf(), ns, parentType);
			if( ti == 0 && !module && currentType )
				ti = GetTypeFromTypesKnownByObject(str.AddressOf(), currentType);

			if( ti && !found )
			{
				found = true;

				if( ti->flags & asOBJ_IMPLICIT_HANDLE )
					isImplicitHandle = true;

				// Make sure the module has access to the object type
				if( !module || true )
				{
					if( asOBJ_TYPEDEF == (ti->flags & asOBJ_TYPEDEF) )
					{
						// TODO: typedef: A typedef should be considered different from the original type (though with implicit conversions between the two)
						// Create primitive data type based on object flags
						dt = CastToTypedefType(ti)->aliasForType;
						dt.MakeReadOnly(isConst);
					}
					else
					{
						if( ti->flags & asOBJ_TEMPLATE )
						{
							ti = GetTemplateInstanceFromNode(n, file, CastToObjectType(ti), implicitNamespace, currentType, &n);
							if (ti == 0)
							{
								if (isValid)
									*isValid = false;

								// Return a dummy
								auto dummy = asCDataType::CreatePrimitive(ttInt, false);
								dummy.SetIsErrorDataType(true);
								return dummy;
							}
						}
						else if( n && n->next && n->next->nodeType == snDataType )
						{
							if (reportError)
							{
								asCString msg;
								msg.Format(TXT_TYPE_s_NOT_TEMPLATE, ti->name.AddressOf());
								WriteError(msg, file, n);
							}
							if (isValid)
								*isValid = false;
						}

						// Create object data type
						if( ti )
							dt = asCDataType::CreateType(ti, isConst);
						else
							dt = asCDataType::CreatePrimitive(ttInt, isConst);
					}
				}
				else
				{
					if (reportError)
					{
						asCString msg;
						msg.Format(TXT_TYPE_s_NOT_AVAILABLE_FOR_MODULE, (const char *)str.AddressOf());
						WriteError(msg, file, n);
					}

					dt.SetTokenType(ttInt);
					dt.SetIsErrorDataType(true);
					if (isValid)
						*isValid = false;
				}
			}

			if( !found )
			{
				// Try to find it in the parent namespace
				if( ns )
					ns = engine->GetParentNameSpace(ns);
				if (parentType)
					parentType = 0;
			}
		}

		if( !found )
		{
			if (reportError)
			{
				asCString msg;
				if (origNs && origNs->name == "")
					msg.Format(TXT_IDENTIFIER_s_NOT_DATA_TYPE_IN_GLOBAL_NS, str.AddressOf());
				else if (origNs)
					msg.Format(TXT_IDENTIFIER_s_NOT_DATA_TYPE_IN_NS_s, str.AddressOf(), origNs->name.AddressOf());
				else
				{
					// TODO: child funcdef: Message should explain that the identifier is not a type of the parent type
					asCDataType pt = asCDataType::CreateType(origParentType, false);
					msg.Format(TXT_IDENTIFIER_s_NOT_DATA_TYPE_IN_NS_s, str.AddressOf(), pt.Format(origParentType->nameSpace, false).AddressOf());
				}
				WriteError(msg, file, n);
			}

			dt = asCDataType::CreatePrimitive(ttInt, isConst);
			dt.SetIsErrorDataType(true);
			if (isValid)
				*isValid = false;
			return dt;
		}
	}
	else if( n->tokenType == ttAuto )
	{
		dt = asCDataType::CreateAuto(isConst);
	}
	else
	{
		// Create primitive data type
		dt = asCDataType::CreatePrimitive(n->tokenType, isConst);
	}

	// Determine array dimensions and object handles
	n = n->next;
	while( n && (n->tokenType == ttOpenBracket || n->tokenType == ttHandleSuffix || n->tokenType == ttUnresolvedObject) )
	{
		if( n->tokenType == ttOpenBracket )
		{
			// Make sure the sub type can be instantiated
			if( !dt.CanBeInstantiated() )
			{
				if (reportError)
				{
					asCString str;
					if (dt.IsAbstractClass())
						str.Format(TXT_ABSTRACT_CLASS_s_CANNOT_BE_INSTANTIATED, dt.Format(ns).AddressOf());
					else if (dt.IsInterface())
						str.Format(TXT_INTERFACE_s_CANNOT_BE_INSTANTIATED, dt.Format(ns).AddressOf());
					else
						// TODO: Improve error message to explain why
						str.Format(TXT_DATA_TYPE_CANT_BE_s, dt.Format(ns).AddressOf());

					WriteError(str, file, n);
				}
				if (isValid)
					*isValid = false;
			}

			// Make the type an array (or multidimensional array)
			if( dt.MakeArray(engine, module) < 0 )
			{
				if( reportError )
					WriteError(TXT_NO_DEFAULT_ARRAY_TYPE, file, n);
				if (isValid)
					*isValid = false;
				break;
			}
		}
		else if (n->tokenType == ttUnresolvedObject)
		{
			dt.MakeUnresolvedObject(true);
		}
		else
		{
			// Make the type a handle
			if( dt.IsObjectHandle() )
			{
				if( reportError )
					WriteError(TXT_HANDLE_OF_HANDLE_IS_NOT_ALLOWED, file, n);
				if (isValid)
					*isValid = false;
				break;
			}
			else if( dt.MakeHandle(true, acceptHandleForScope) < 0 )
			{
				if( reportError )
					WriteError(TXT_OBJECT_HANDLE_NOT_SUPPORTED, file, n);
				if (isValid)
					*isValid = false;
				break;
			}
		}
		n = n->next;
	}

	if( isImplicitHandle )
	{
		// Make the type a handle
		if (dt.MakeHandle(true, acceptHandleForScope) < 0)
		{
			if( reportError )
				WriteError(TXT_OBJECT_HANDLE_NOT_SUPPORTED, file, n);
			if (isValid)
				*isValid = false;
		}
	}

	return dt;
}

asCObjectType *asCBuilder::GetTemplateInstanceFromNode(asCScriptNode *node, asCScriptCode *file, asCObjectType *templateType, asSNameSpace *implicitNamespace, asCObjectType *currentType, asCScriptNode **next)
{
	// Check if the subtype is a type or the template's subtype
	// if it is the template's subtype then this is the actual template type,
	// orderwise it is a template instance.
	// Only do this for application registered interface, as the
	// scripts cannot implement templates.
	asCArray<asCDataType> subTypes;
	asUINT subtypeIndex;
	asCScriptNode *n = node;
	while (n && n->next && n->next->nodeType == snDataType)
	{
		n = n->next;

		// When parsing function definitions for template registrations (currentType != 0) it is necessary
		// to pass in the current template type to the recursive call since it is this ones sub-template types
		// that should be allowed.
		asCDataType subType = CreateDataTypeFromNode(n, file, implicitNamespace, false, module ? 0 : (currentType ? currentType : templateType));
		subTypes.PushLast(subType);

		if (subType.IsReadOnly())
		{
			asCString msg;
			msg.Format(TXT_TMPL_SUBTYPE_MUST_NOT_BE_READ_ONLY);
			WriteError(msg, file, n);

			// Return a dummy
			return 0;
		}
	}

	if (next)
		*next = n;

	if (subTypes.GetLength() != templateType->templateSubTypes.GetLength())
	{
		asCString msg;
		msg.Format(TXT_TMPL_s_EXPECTS_d_SUBTYPES, templateType->name.AddressOf(), int(templateType->templateSubTypes.GetLength()));
		WriteError(msg, file, node);

		// Return a dummy
		return 0;
	}

	// Check if any of the given subtypes are different from the template's declared subtypes
	bool isDifferent = false;
	for (subtypeIndex = 0; subtypeIndex < subTypes.GetLength(); subtypeIndex++)
	{
		if (subTypes[subtypeIndex].GetTypeInfo() != templateType->templateSubTypes[subtypeIndex].GetTypeInfo())
		{
			isDifferent = true;
			break;
		}
	}

	if (isDifferent)
	{
		// This is a template instance
		// Need to find the correct object type
		asCString ErrorMessage;
		asCObjectType *otInstance = engine->GetTemplateInstanceType(templateType, subTypes, module, &ErrorMessage);

		if (otInstance && otInstance->scriptSectionIdx < 0)
		{
			// If this is the first time the template instance is used, store where it was declared from
			otInstance->scriptSectionIdx = engine->GetScriptSectionNameIndex(file->name.AddressOf());
			int row, column;
			file->ConvertPosToRowCol(n->tokenPos, &row, &column);
			otInstance->declaredAt = (row & 0xFFFFF) | (column << 20);
		}

		if (!otInstance)
		{
			asCString sub = subTypes[0].Format(templateType->nameSpace);
			for (asUINT s = 1; s < subTypes.GetLength(); s++)
			{
				sub += ",";
				sub += subTypes[s].Format(templateType->nameSpace);
			}
			asCString msg;
			msg.Format(TXT_INSTANCING_INVLD_TMPL_TYPE_s_s, templateType->name.AddressOf(), sub.AddressOf());
			if (ErrorMessage.GetLength() != 0)
			{
				msg += ": ";
				msg += ErrorMessage;
			}
			WriteError(msg, file, n);
		}

		return otInstance;
	}

	return templateType;
}

asCDataType asCBuilder::ModifyDataTypeFromNode(const asCDataType &type, asCScriptNode *node, asCScriptCode *file, asETypeModifiers *inOutFlags, bool *autoHandle)
{
	asCDataType dt = type;

	if( inOutFlags ) *inOutFlags = asTM_NONE;

	// Is the argument sent by reference?
	asCScriptNode *n = node->firstChild;
	if( n && n->tokenType == ttAmp )
	{
		if (dt.GetTokenType() == ttVoid)
		{
			asCString msg;
			msg.Format(TXT_TYPE_s_CANNOT_BE_REFERENCE, type.Format(0).AddressOf());
			WriteError(msg, file, node->firstChild);
			return dt;
		}

		dt.MakeReference(true);
		n = n->next;

		if( n )
		{
			if( inOutFlags )
			{
				if( n->tokenType == ttIn )
					*inOutFlags = asTM_INREF;
				else if( n->tokenType == ttOut )
					*inOutFlags = asTM_OUTREF;
				else if( n->tokenType == ttInOut )
					*inOutFlags = asTM_INOUTREF;
				else
					asASSERT(false);
			}

			n = n->next;
		}
		else
		{
			if( inOutFlags )
				*inOutFlags = asTM_INOUTREF; // ttInOut
		}

		if( !engine->ep.allowUnsafeReferences &&
			inOutFlags && *inOutFlags == asTM_INOUTREF &&
			!(dt.GetTypeInfo() && (dt.GetTypeInfo()->flags & asOBJ_TEMPLATE_SUBTYPE)) )
		{
			// Verify that the base type support &inout parameter types
			if( !dt.IsObject() || dt.IsObjectHandle() || 
				!((dt.GetTypeInfo()->flags & asOBJ_NOCOUNT) || (CastToObjectType(dt.GetTypeInfo())->beh.addref && CastToObjectType(dt.GetTypeInfo())->beh.release)) )
				WriteError(TXT_ONLY_OBJECTS_MAY_USE_REF_INOUT, file, node->firstChild);
		}
	}

	if( autoHandle ) *autoHandle = false;

	if (n && n->tokenType == ttUnresolvedObject)
	{
		dt.MakeUnresolvedObject(true);
	}

	if( n && n->tokenType == ttPlus )
	{
		// Autohandles are not supported for types with NOCOUNT
		// If the type is not a handle then there was an error with building the type, but
		// this error would already have been reported so no need to report another error here
		if( dt.IsObjectHandle() && (dt.GetTypeInfo()->flags & asOBJ_NOCOUNT) )
			WriteError(TXT_AUTOHANDLE_CANNOT_BE_USED_FOR_NOCOUNT, file, node->firstChild);

		if( autoHandle ) *autoHandle = true;
	}

	if (n && n->tokenType == ttIdentifier)
	{
		asCString str;
		str.Assign(&file->code[n->tokenPos], n->tokenLength);
		if (str == IF_HANDLE_TOKEN)
		{
			dt.SetIfHandleThenConst(true);
		}
		else if (str == HANDLE_ONLY_TOKEN)
		{
			dt.MakeHandle(true);
		}
		else if (str == ANY_IMPLICIT_INTEGER_TOKEN)
		{
			dt.SetAnyImplicitIntegerType(true);
		}
		else if (str == AUTO_CONSTREF_TOKEN)
		{
			if (dt.IsAuto())
			{
				dt.SetAutoConstRef(true);
			}
			else
			{
				if (!dt.IsReference())
				{
					if (dt.IsNonPrimitiveValueType())
						dt.MakeReference(true);
					dt.MakeReadOnly(true);
				}
			}
		}
		else
		{
			// TODO: Should give error if not currently parsing template registration
			asCString msg;
			msg.Format(TXT_UNEXPECTED_TOKEN_s, str.AddressOf());
			WriteError(msg, file, node->firstChild);
		}
	}

	return dt;
}

asCTypeInfo *asCBuilder::GetType(const char *type, asSNameSpace *ns, asCObjectType *parentType)
{
	asASSERT((ns == 0 && parentType) || (ns && parentType == 0));

	if (ns)
	{
		asCTypeInfo *ti = nullptr;
		engine->allRegisteredTypesByName.FindAllUntil(type, [&](asCTypeInfo* checkType)
		{
			if (checkType->nameSpace == ns)
			{
				ti = checkType;
				return true;
			}
			return false;
		});

		if (ti)
		{
			return ti;
		}
		else if (module)
		{
			if (engine->ep.automaticImports)
			{
				engine->allScriptDeclaredTypes.FindAllUntil(type, [&](asCTypeInfo* checkType)
				{
					if (checkType->nameSpace == ns)
					{
						ti = checkType;
						return true;
					}
					return false;
				});
			}
			else
			{
				ti = module->GetType(type, ns);
			}
		}

		return ti;
	}
	else
	{
		// Recursively check base classes
		asCObjectType *currType = parentType;
		while (currType)
		{
			for (asUINT n = 0; n < currType->childFuncDefs.GetLength(); n++)
			{
				asCFuncdefType *funcDef = currType->childFuncDefs[n];
				if (funcDef && funcDef->name == type)
					return funcDef;
			}
			currType = currType->derivedFrom;
		}
	}

	return 0;
}

asCObjectType *asCBuilder::GetObjectType(const char *type, asSNameSpace *ns)
{
	return CastToObjectType(GetType(type, ns, 0));
}

#ifndef AS_NO_COMPILER
// This function will return true if there are any types in the engine or module
// with the given name. The namespace is ignored in this verification.
bool asCBuilder::DoesTypeExist(const char* type)
{
	if (module)
	{
		if (engine->ep.automaticImports)
		{
			if (engine->allScriptDeclaredTypes.Contains(type))
				return true;
		}
		else
		{
			if (module->GetType(type, nullptr) != nullptr)
				return true;
		}
	}

	if (engine->allRegisteredTypesByName.Contains(type))
		return true;
	return false;
}
#endif

asCTypeInfo *asCBuilder::GetTypeFromTypesKnownByObject(const char *type, asCObjectType *currentType)
{
	if (currentType->name == type)
		return currentType;

	asUINT n;

	asCTypeInfo *found = 0;

	for (n = 0; found == 0 && n < currentType->properties.GetLength(); n++)
		if (currentType->properties[n]->type.GetTypeInfo() &&
			currentType->properties[n]->type.GetTypeInfo()->name == type)
			found = currentType->properties[n]->type.GetTypeInfo();

	for (n = 0; found == 0 && n < currentType->methods.GetLength(); n++)
	{
		asCScriptFunction *func = engine->scriptFunctions[currentType->methods[n]];
		if (func->returnType.GetTypeInfo() &&
			func->returnType.GetTypeInfo()->name == type)
			found = func->returnType.GetTypeInfo();

		for (asUINT f = 0; found == 0 && f < func->parameterTypes.GetLength(); f++)
			if (func->parameterTypes[f].GetTypeInfo() &&
				func->parameterTypes[f].GetTypeInfo()->name == type)
				found = func->parameterTypes[f].GetTypeInfo();
	}

	if (found)
	{
		// In case we find a template instance it mustn't be returned
		// because it is not known if the subtype is really matching
		if (found->flags & asOBJ_TEMPLATE)
			return 0;
	}

	return found;
}

asCFuncdefType *asCBuilder::GetFuncDef(const char *type, asSNameSpace *ns, asCObjectType *parentType)
{
	asASSERT((ns == 0 && parentType) || (ns && parentType == 0));

	if (ns)
	{
		for (asUINT n = 0; n < engine->registeredFuncDefs.GetLength(); n++)
		{
			asCFuncdefType *funcDef = engine->registeredFuncDefs[n];
			// TODO: access: Only return the definitions that the module has access to
			if (funcDef && funcDef->nameSpace == ns && funcDef->name == type)
				return funcDef;
		}

		if (module)
		{
			for (asUINT n = 0; n < module->funcDefs.GetLength(); n++)
			{
				asCFuncdefType *funcDef = module->funcDefs[n];
				if (funcDef && funcDef->nameSpace == ns && funcDef->name == type)
					return funcDef;
			}
		}
	}
	else
	{
		// Recursively check base classes
		asCObjectType *currType = parentType;
		while (currType)
		{
			for (asUINT n = 0; n < currType->childFuncDefs.GetLength(); n++)
			{
				asCFuncdefType *funcDef = currType->childFuncDefs[n];
				if (funcDef && funcDef->name == type)
					return funcDef;
			}
			currType = currType->derivedFrom;
		}
	}

	return 0;
}

#ifndef AS_NO_COMPILER

int asCBuilder::GetEnumValueFromType(asCEnumType *type, const char *name, asCDataType &outDt, asDWORD &outValue)
{
	if( !type || !(type->flags & asOBJ_ENUM) )
		return 0;

	for( asUINT n = 0; n < type->enumValues.GetLength(); ++n )
	{
		if( type->enumValues[n]->name == name )
		{
			outDt = asCDataType::CreateType(type, true);
			outValue = type->enumValues[n]->value;
			return 1;
		}
	}

	return 0;
}

int asCBuilder::GetEnumValue(const char *name, asCDataType &outDt, asDWORD &outValue, asSNameSpace *ns)
{
	bool found = false;

	// Search all available enum types
	asUINT t;
	for( t = 0; t < engine->registeredEnums.GetLength(); t++ )
	{
		asCEnumType *et = engine->registeredEnums[t];
		if( ns != et->nameSpace ) continue;

		if( GetEnumValueFromType(et, name, outDt, outValue) )
		{
			if( !found )
				found = true;
			else
			{
				// Found more than one value in different enum types
				return 2;
			}
		}
	}

	for( t = 0; t < module->enumTypes.GetLength(); t++ )
	{
		asCEnumType *et = module->enumTypes[t];
		if( ns != et->nameSpace ) continue;

		if( GetEnumValueFromType(et, name, outDt, outValue) )
		{
			if( !found )
				found = true;
			else
			{
				// Found more than one value in different enum types
				return 2;
			}
		}
	}

	if( found )
		return 1;

	// Didn't find any value
	return 0;
}

void asCBuilder::MarkStructuralDependency(asCTypeInfo* UserTypeInfo, asCTypeInfo* DependentOnTypeInfo, asCScriptNode* node, asCScriptCode* script)
{
	if (!module)
		return;
	if (DependentOnTypeInfo->module == nullptr)
		return;

	asUINT SubTypeCount = DependentOnTypeInfo->GetSubTypeCount();
	if (SubTypeCount != 0)
	{
		for (asUINT i = 0; i < SubTypeCount; ++i)
		{
			// This one doesn't need to be structural, because template types cannot change size!
			if (auto* SubTypeInfo = (asCTypeInfo*)DependentOnTypeInfo->GetSubType(i))
				MarkDependency(SubTypeInfo, node, script);
		}
		return;
	}

	if (DependentOnTypeInfo->module == module)
		return;

	asCModule::FModuleDependencyInfo* ExistingInfo = module->moduleDependencies.Find(DependentOnTypeInfo->module);
	if (ExistingInfo != nullptr)
	{
		ExistingInfo->bIsStructuralDependency = true;
		return;
	}

	asCModule::FModuleDependencyInfo DependencyInfo;
	DependencyInfo.bIsStructuralDependency = true;
	if (node != nullptr && script != nullptr)
	{
		script->ConvertPosToRowCol(node->tokenPos, &DependencyInfo.FirstLineNumber, &DependencyInfo.FirstColumn);
	}
	else
	{
		DependencyInfo.FirstLineNumber = 0;
		DependencyInfo.FirstColumn = 0;
	}

	module->moduleDependencies.Add(DependentOnTypeInfo->module, DependencyInfo);
}

void asCBuilder::MarkDependency(asCModule* DependencyModule, asCScriptNode* node, asCScriptCode* script)
{
	if (!module)
		return;
	if (DependencyModule == nullptr)
		return;
	if (DependencyModule == module)
		return;

	asCModule::FModuleDependencyInfo* ExistingInfo = module->moduleDependencies.Find(DependencyModule);
	if (ExistingInfo != nullptr)
		return;

	asCModule::FModuleDependencyInfo DependencyInfo;
	if (node != nullptr && script != nullptr)
	{
		script->ConvertPosToRowCol(node->tokenPos, &DependencyInfo.FirstLineNumber, &DependencyInfo.FirstColumn);
	}
	else
	{
		DependencyInfo.FirstLineNumber = 0;
		DependencyInfo.FirstColumn = 0;
	}

	module->moduleDependencies.Add(DependencyModule, DependencyInfo);
}

void asCBuilder::MarkHardValueDependency(asCModule* DependencyModule, asCScriptNode* node, asCScriptCode* script)
{
	if (!module)
		return;
	if (DependencyModule == nullptr)
		return;
	if (DependencyModule == module)
		return;

	asCModule::FModuleDependencyInfo* ExistingInfo = module->moduleDependencies.Find(DependencyModule);
	if (ExistingInfo != nullptr)
	{
		ExistingInfo->bIsHardValueDependency = true;
		return;
	}

	asCModule::FModuleDependencyInfo DependencyInfo;
	DependencyInfo.bIsHardValueDependency = true;

	if (node != nullptr && script != nullptr)
	{
		script->ConvertPosToRowCol(node->tokenPos, &DependencyInfo.FirstLineNumber, &DependencyInfo.FirstColumn);
	}
	else
	{
		DependencyInfo.FirstLineNumber = 0;
		DependencyInfo.FirstColumn = 0;
	}

	module->moduleDependencies.Add(DependencyModule, DependencyInfo);
}


void asCBuilder::MarkDependency(asCTypeInfo* TypeInfo, asCScriptNode* node, asCScriptCode* script)
{
	if (!module)
		return;
	if (TypeInfo == nullptr)
		return;
	if (TypeInfo->module == nullptr)
		return;

	asUINT SubTypeCount = TypeInfo->GetSubTypeCount();
	if (SubTypeCount != 0)
	{
		for (asUINT i = 0; i < SubTypeCount; ++i)
		{
			if (auto* SubTypeInfo = (asCTypeInfo*)TypeInfo->GetSubType(i))
				MarkDependency(SubTypeInfo, node, script);
		}
		return;
	}

	MarkDependency(TypeInfo->module, node, script);
}

void asCBuilder::MarkDependency(asCGlobalProperty* Variable, asCScriptNode* node, asCScriptCode* script)
{
	if (bValueDependenciesAreHard && !Variable->name.StartsWith("__StaticType_"))
		MarkHardValueDependency(Variable->module, node, script);
	else
		MarkDependency(Variable->module, node, script);
}

void asCBuilder::MarkDependency(asCScriptFunction* Function, asCScriptNode* node, asCScriptCode* script)
{
	if (Function->module != nullptr)
	{
		if (bValueDependenciesAreHard && (Function->name != "StaticClass" || !Function->traits.GetTrait(asTRAIT_GENERATED_FUNCTION)))
			MarkHardValueDependency(Function->module, node, script);
		else
			MarkDependency(Function->module, node, script);
	}
	else if (Function->objectType != nullptr && Function->objectType->module != nullptr)
	{
		MarkDependency(Function->objectType, node, script);
	}
}

#if WITH_EDITOR
void asCBuilder::SetEditorOnlyBlockLinePositions(const TArray<TPair<int,int>>& LinePositions)
{
	if (scripts.GetLength() == 0)
		return;

	asCScriptCode* script = scripts[0];
	for (auto& LineStartAndEnd : LinePositions)
	{
		TPair<size_t,size_t>& CharacterStartAndEnd = EditorOnlyBlockCharacterPositions.Emplace_GetRef();
		if ((asUINT)LineStartAndEnd.Key <= script->linePositions.GetLength())
			CharacterStartAndEnd.Key = script->linePositions[LineStartAndEnd.Key - 1];
		else
			CharacterStartAndEnd.Key = 0;

		if ((asUINT)LineStartAndEnd.Value < script->linePositions.GetLength())
			CharacterStartAndEnd.Value = script->linePositions[LineStartAndEnd.Value];
		else
			CharacterStartAndEnd.Value = (size_t)-1;
	}
}

bool asCBuilder::IsNodeInEditorOnlyCode(asCScriptCode* script, asCScriptNode* node)
{
	if (isEditorOnlyModule)
		return true;
	if (scripts.GetLength() == 0 || script != scripts[0])
		return false;
	if (node == nullptr)
		return false;

	for (auto& CharacterStartAndEnd : EditorOnlyBlockCharacterPositions)
	{
		if (node->tokenPos >= CharacterStartAndEnd.Key && node->tokenPos <= CharacterStartAndEnd.Value)
			return true;
	}

	return false;
}

void asCBuilder::CheckEditorOnlyType(asCTypeInfo* TypeInfo, asCScriptNode* node, asCScriptCode* script)
{
	if (TypeInfo != nullptr && (TypeInfo->flags & asOBJ_EDITOR_ONLY))
	{
		if (!IsNodeInEditorOnlyCode(script, node))
		{
			if (FAngelscriptManager::Get().ConfigSettings->bErrorOnIncorrectEditorOnlyCode)
			{
				asCString str;
				str.Format("Cannot use editor-only type %s outside of an EDITOR block", TypeInfo->GetName());
				WriteError(str, script, node);
			}
		}
	}
}

void asCBuilder::CheckEditorOnlyFunction(asCScriptFunction* Function, asCScriptNode* node, asCScriptCode* script)
{
	if (Function != nullptr && (Function->traits.GetTrait(asTRAIT_EDITOR_ONLY) || (Function->objectType != nullptr && Function->objectType->flags & asOBJ_EDITOR_ONLY)))
	{
		if (!IsNodeInEditorOnlyCode(script, node))
		{
			if (FAngelscriptManager::Get().ConfigSettings->bErrorOnIncorrectEditorOnlyCode)
			{
				asCString str;
				str.Format("Cannot use editor-only function %s outside of an EDITOR block", Function->GetName());
				WriteError(str, script, node);
			}
		}
	}
}

void asCBuilder::CheckEditorOnlyProperty(asCObjectProperty* Property, asCScriptNode* node, asCScriptCode* script)
{
	if (Property != nullptr && Property->isEditorOnly)
	{
		if (!IsNodeInEditorOnlyCode(script, node))
		{
			if (FAngelscriptManager::Get().ConfigSettings->bErrorOnIncorrectEditorOnlyCode)
			{
				asCString str;
				str.Format("Cannot use editor-only property %s outside of an EDITOR block", Property->GetName());
				WriteError(str, script, node);
			}
		}
	}
}

void asCBuilder::CheckEditorOnlyGlobal(asCGlobalProperty* Property, asCScriptNode* node, asCScriptCode* script)
{
	if (Property != nullptr && Property->isEditorOnly)
	{
		if (!IsNodeInEditorOnlyCode(script, node))
		{
			if (FAngelscriptManager::Get().ConfigSettings->bErrorOnIncorrectEditorOnlyCode)
			{
				asCString str;
				str.Format("Cannot use editor-only property %s outside of an EDITOR block", Property->name.AddressOf());
				WriteError(str, script, node);
			}
		}
	}
}
#endif

#endif // AS_NO_COMPILER

END_AS_NAMESPACE
