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
// as_context.cpp
//
// This class handles the execution of the byte code
//


#include "as_context.h"
#include "as_config.h"
#include "as_scriptengine.h"
#include "as_tokendef.h"
#include "as_texts.h"
#include "as_callfunc.h"
#include "as_generic.h"
#include "as_debug.h" // mkdir()
#include "as_bytecode.h"
#include "as_scriptobject.h"

#include "AngelscriptSettings.h"
#include "AngelscriptManager.h"
#include <math.h> // fmodf() pow()

#ifdef _MSC_VER
#pragma warning(disable:4702) // unreachable code
#endif

BEGIN_AS_NAMESPACE

// We need at least 2 PTRs reserved for exception handling
// We need at least 1 PTR reserved for calling system functions
const int RESERVE_STACK = 2*AS_PTR_SIZE;

// For each script function call we push 9 PTRs on the call stack
const int CALLSTACK_FRAME_SIZE = 11;

bool asCContext::CanEverRunLineCallback = false;
bool asCContext::ShouldAlwaysRunLineCallback = false;

#if defined(AS_DEBUG)

class asCDebugStats
{
public:
	asCDebugStats()
	{
		memset(instrCount, 0, sizeof(instrCount));
		memset(instrCount2, 0, sizeof(instrCount2));
		lastBC = 255;
	}

	~asCDebugStats()
	{
		// This code writes out some statistics for the VM.
		// It's useful for determining what needs to be optimized.

		_mkdir("AS_DEBUG");
		#if _MSC_VER >= 1500 && !defined(AS_MARMALADE)
			FILE *f;
			fopen_s(&f, "AS_DEBUG/stats.txt", "wt");
		#else
			FILE *f = fopen("AS_DEBUG/stats.txt", "wt");
		#endif
		if( f )
		{
			// Output instruction statistics
			fprintf(f, "\nTotal count\n");
			int n;
			for( n = 0; n < asBC_MAXBYTECODE; n++ )
			{
				if( asBCInfo[n].name && instrCount[n] > 0 )
					fprintf(f, "%-10.10s : %.0f\n", asBCInfo[n].name, instrCount[n]);
			}

			fprintf(f, "\nNever executed\n");
			for( n = 0; n < asBC_MAXBYTECODE; n++ )
			{
				if( asBCInfo[n].name && instrCount[n] == 0 )
					fprintf(f, "%-10.10s\n", asBCInfo[n].name);
			}

			fprintf(f, "\nSequences\n");
			for( n = 0; n < 256; n++ )
			{
				if( asBCInfo[n].name )
				{
					for( int m = 0; m < 256; m++ )
					{
						if( instrCount2[n][m] )
							fprintf(f, "%-10.10s, %-10.10s : %.0f\n", asBCInfo[n].name, asBCInfo[m].name, instrCount2[n][m]);
					}
				}
			}
			fclose(f);
		}
	}

	void Instr(asBYTE bc)
	{
		++instrCount[bc];
		++instrCount2[lastBC][bc];
		lastBC = bc;
	}

	// Instruction statistics
	double instrCount[256];
	double instrCount2[256][256];
	int lastBC;
} stats;

#endif

// interface
AS_API asIScriptContext *asGetActiveContext()
{
	asCThreadLocalData *tld = asCThreadManager::GetLocalData();
	return tld->activeContext;
}

AS_API asIScriptContext *asGetUsableContext()
{
	asCThreadLocalData *tld = asCThreadManager::GetLocalData();
	if (tld->activeContext != nullptr)
		return tld->activeContext;
	return tld->primaryContext;
}

AS_API asIScriptFunction *asGetActiveFunction()
{
	asCThreadLocalData *tld = asCThreadManager::GetLocalData();
	return tld->activeFunction;
}

struct FScopeSetActiveContext
{
	asCThreadLocalData* tld;
	FScriptExecution* prevExecution;
	asCContext* prevContext;

	FScopeSetActiveContext(asCThreadLocalData* InTld, asCContext* NewContext)
		: tld(InTld)
		, prevExecution(InTld->activeExecution)
		, prevContext(InTld->activeContext)
	{
		InTld->activeContext = NewContext;
		InTld->activeExecution = nullptr;
	}

	~FScopeSetActiveContext()
	{
		tld->activeContext = prevContext;
		tld->activeExecution = prevExecution;
	}
};

asCContext::asCContext(asCScriptEngine *engine, bool holdRef)
{
	m_refCount.set(1);

	m_holdEngineRef = holdRef;
	if( holdRef )
		engine->AddRef();

	m_engine                    = engine;
	m_status                    = asEXECUTION_UNINITIALIZED;
	m_stackBlockSize            = 0;
	m_originalStackPointer      = 0;
	m_originalStackIndex        = 0;
	m_inExceptionHandler        = false;
	m_isStackMemoryNotAllocated = false;
	m_needToCleanupArgs         = false;
	m_currentFunction           = 0;
	m_regs.objectRegister       = 0;
	m_initialFunction           = 0;
	m_lineCallback              = nullptr;
	m_stackPopCallback			= nullptr;
	m_exceptionCallback         = false;
	m_userData                  = 0;
	m_regs.ctx                  = this;
	m_regs.tld = asCThreadManager::GetLocalData();
	m_callingSystemFunction = nullptr;
}

void asCContext::MovedToNewThread()
{
	m_regs.tld = asCThreadManager::GetLocalData();
}

asCContext::~asCContext()
{
	DetachEngine();
}

// interface
bool asCContext::IsNested(asUINT *nestCount) const
{
	if( nestCount )
		*nestCount = 0;

	asUINT c = GetCallstackSize();
	if( c == 0 )
		return false;

	// Search for a marker on the call stack
	// This loop starts at 2 because the 0th entry is not stored in m_callStack,
	// and then we need to subtract one more to get the base of each frame
	for( asUINT n = 2; n <= c; n++ )
	{
		const asPWORD *s = m_callStack.AddressOf() + (c - n)*CALLSTACK_FRAME_SIZE;
		if( s && s[0] == 0 )
		{
			if( nestCount )
				(*nestCount)++;
			else
				return true;
		}
	}

	if( nestCount && *nestCount > 0 )
		return true;

	return false;
}

// interface
int asCContext::AddRef() const
{
	return m_refCount.atomicInc();
}

// interface
int asCContext::Release() const
{
	int r = m_refCount.atomicDec();

	if( r == 0 )
	{
		asDELETE(const_cast<asCContext*>(this),asCContext);
		return 0;
	}

	return r;
}

// internal
void asCContext::DetachEngine()
{
	if( m_engine == 0 ) return;

	// Clean up all calls, included nested ones
	do
	{
		// Abort any execution
		Abort();

		// Free all resources
		Unprepare();
	}
	while( IsNested() );

	// Free the stack blocks
	for( asUINT n = 0; n < m_stackBlocks.GetLength(); n++ )
	{
		if( m_stackBlocks[n] )
		{
#ifndef WIP_16BYTE_ALIGN
			asDELETEARRAY(m_stackBlocks[n]);
#else
			asDELETEARRAYALIGNED(m_stackBlocks[n]);
#endif
		}
	}
	m_stackBlocks.SetLength(0);
	m_stackBlockSize = 0;

	// Clean the user data
	for( asUINT n = 0; n < m_userData.GetLength(); n += 2 )
	{
		if( m_userData[n+1] )
		{
			for( asUINT c = 0; c < m_engine->cleanContextFuncs.GetLength(); c++ )
				if( m_engine->cleanContextFuncs[c].type == m_userData[n] )
					m_engine->cleanContextFuncs[c].cleanFunc(this);
		}
	}
	m_userData.SetLength(0);

	// Clear engine pointer
	if( m_holdEngineRef )
		m_engine->Release();
	m_engine = 0;
}

// interface
asIScriptEngine *asCContext::GetEngine() const
{
	return m_engine;
}

// interface
void *asCContext::SetUserData(void *data, asPWORD type)
{
	// As a thread might add a new new user data at the same time as another
	// it is necessary to protect both read and write access to the userData member
	ACQUIREEXCLUSIVE(m_engine->engineRWLock);

	// It is not intended to store a lot of different types of userdata,
	// so a more complex structure like a associative map would just have
	// more overhead than a simple array.
	for( asUINT n = 0; n < m_userData.GetLength(); n += 2 )
	{
		if( m_userData[n] == type )
		{
			void *oldData = reinterpret_cast<void*>(m_userData[n+1]);
			m_userData[n+1] = reinterpret_cast<asPWORD>(data);

			RELEASEEXCLUSIVE(m_engine->engineRWLock);

			return oldData;
		}
	}

	m_userData.PushLast(type);
	m_userData.PushLast(reinterpret_cast<asPWORD>(data));

	RELEASEEXCLUSIVE(m_engine->engineRWLock);

	return 0;
}

// interface
void *asCContext::GetUserData(asPWORD type) const
{
	// There may be multiple threads reading, but when
	// setting the user data nobody must be reading.
	ACQUIRESHARED(m_engine->engineRWLock);

	for( asUINT n = 0; n < m_userData.GetLength(); n += 2 )
	{
		if( m_userData[n] == type )
		{
			RELEASESHARED(m_engine->engineRWLock);
			return reinterpret_cast<void*>(m_userData[n+1]);
		}
	}

	RELEASESHARED(m_engine->engineRWLock);

	return 0;
}

// interface
int asCContext::Prepare(asIScriptFunction *func)
{
	if( func == 0 )
	{
		asCString str;
		str.Format(TXT_FAILED_IN_FUNC_s_WITH_s_s_d, "Prepare", "null", errorNames[-asNO_FUNCTION], asNO_FUNCTION);
		m_engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, str.AddressOf());
		return asNO_FUNCTION;
	}

	if( m_status == asEXECUTION_ACTIVE || m_status == asEXECUTION_SUSPENDED )
	{
		asCString str;
		str.Format(TXT_FAILED_IN_FUNC_s_WITH_s_s_d, "Prepare", func->GetDeclaration(true, true, false, true), errorNames[-asCONTEXT_ACTIVE], asCONTEXT_ACTIVE);
		m_engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, str.AddressOf());
		return asCONTEXT_ACTIVE;
	}

	// Clean the stack if not done before
	if( m_status != asEXECUTION_FINISHED && m_status != asEXECUTION_UNINITIALIZED )
	{
		m_loopDetectionCounter = 0;
		m_loopDetectionTimer = -1.0;
		CleanStack();
	}

	// Release the returned object (if any)
	CleanReturnObject();

	// Release the object if it is a script object
	if( m_initialFunction && m_initialFunction->objectType && (m_initialFunction->objectType->flags & asOBJ_SCRIPT_OBJECT) )
	{
		asCScriptObject *obj = *(asCScriptObject**)&m_regs.stackFramePointer[0];
		//if( obj )
			//obj->Release();

		*(asPWORD*)&m_regs.stackFramePointer[0] = 0;
	}

	if( m_initialFunction && m_initialFunction == func )
	{
		// If the same function is executed again, we can skip a lot of the setup
		m_currentFunction = m_initialFunction;

		// Reset stack pointer
		m_regs.stackPointer = m_originalStackPointer;
		m_stackIndex = m_originalStackIndex;

		// Make sure the stack pointer is pointing to the original position,
		// otherwise something is wrong with the way it is being updated
		asASSERT( IsNested() || m_stackIndex > 0 || (m_regs.stackPointer == m_stackBlocks[0] + m_stackBlockSize) );

		m_regs.stackFramePointer = (asDWORD*)((asPWORD)(m_regs.stackPointer - m_argumentsSize - m_returnValueSize) & ~(16 - 1));
	}
	else
	{
		asASSERT( m_engine );

		// Make sure the function is from the same engine as the context to avoid mixups
		if( m_engine != func->GetEngine() )
		{
			asCString str;
			str.Format(TXT_FAILED_IN_FUNC_s_WITH_s_s_d, "Prepare", func->GetDeclaration(true, true, false, true), errorNames[-asINVALID_ARG], asINVALID_ARG);
			m_engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, str.AddressOf());
			return asINVALID_ARG;
		}

		if( m_initialFunction )
		{
			m_initialFunction->Release();

			// Reset stack pointer
			m_regs.stackPointer = m_originalStackPointer;
			m_stackIndex = m_originalStackIndex;

			// Make sure the stack pointer is pointing to the original position,
			// otherwise something is wrong with the way it is being updated
			asASSERT( IsNested() || m_stackIndex > 0 || (m_regs.stackPointer == m_stackBlocks[0] + m_stackBlockSize) );
		}

		// We trust the application not to pass anything else but a asCScriptFunction
		m_initialFunction = static_cast<asCScriptFunction *>(func);
		m_initialFunction->AddRef();
		m_currentFunction = m_initialFunction;

		m_argumentsSize = m_currentFunction->spaceNeededForArguments + (m_currentFunction->objectType ? AS_PTR_SIZE : 0);

		// Reserve space for the arguments and return value
		if( m_currentFunction->DoesReturnOnStack() )
		{
			m_argumentsSize += AS_PTR_SIZE;

			// Alignment is in bytes, m_argumentsSize in dwords, so only need to apply alignments bigger than 4
			int returnAlignment = m_currentFunction->returnType.GetAlignment();
			if (returnAlignment > 4)
			{
				int dwordAlignment = returnAlignment / 4;
				m_argumentsSize = (m_argumentsSize + dwordAlignment - 1) & ~(dwordAlignment - 1);
			}

			m_returnValueSize = m_currentFunction->returnType.GetSizeInMemoryDWords();
		}
		else
			m_returnValueSize = 0;

		// Determine the minimum stack size needed
		int stackSize = m_argumentsSize + m_returnValueSize;
		if( m_currentFunction->scriptData )
			stackSize += m_currentFunction->scriptData->stackNeeded;

		// Make sure there is enough space on the stack for the arguments and return value
		if( !ReserveStackSpace(stackSize + 16) )
			return asOUT_OF_MEMORY;

		m_regs.stackFramePointer = (asDWORD*)((asPWORD)(m_regs.stackPointer - m_argumentsSize - m_returnValueSize) & ~(16 - 1));
	}

	// Reset state
	// Most of the time the previous state will be asEXECUTION_FINISHED, in which case the values are already initialized
	if( m_status != asEXECUTION_FINISHED )
	{
		m_exceptionLine           = -1;
		m_exceptionFunction       = 0;
		m_doAbort                 = false;
		m_externalSuspendRequest  = false;
	}
	m_status = asEXECUTION_PREPARED;
	m_executeVirtualCall = true;
	m_regs.programPointer = 0;

	if (m_callStack.GetLength() == 0)
	{
		m_loopDetectionCounter = 0;
		m_loopDetectionTimer = -1.0;
	}

	// Reserve space for the arguments and return value
	m_originalStackPointer   = m_regs.stackPointer;
	m_originalStackIndex     = m_stackIndex;
	m_regs.stackPointer      = m_regs.stackFramePointer;

#if DO_BLUEPRINT_GUARD
	if (auto* BPStack = FBlueprintContextTracker::TryGet())
		m_blueprintStackFrameIndex = BPStack->GetCurrentScriptStack().Num();
	else
		m_blueprintStackFrameIndex = 0;
#else
	m_blueprintStackFrameIndex = 0;
#endif

	// Set arguments to 0
	memset(m_regs.stackPointer, 0, 4*m_argumentsSize);

	if( m_returnValueSize )
	{
		// Set the address of the location where the return value should be put
		asDWORD *ptr = m_regs.stackFramePointer;
		if( m_currentFunction->objectType )
			ptr += AS_PTR_SIZE;

		*(void**)ptr = (void*)(m_regs.stackFramePointer + m_argumentsSize);
	}

	return asSUCCESS;
}

// Free all resources
int asCContext::Unprepare()
{
	if( m_status == asEXECUTION_ACTIVE || m_status == asEXECUTION_SUSPENDED )
		return asCONTEXT_ACTIVE;

	// Set the context as active so that any clean up code can use access it if desired
	FScopeSetActiveContext MakeActive(m_regs.tld, this);

	asDWORD count = m_refCount.get();
	UNUSED_VAR(count);

	// Only clean the stack if the context was prepared but not executed until the end
	if( m_status != asEXECUTION_UNINITIALIZED &&
		m_status != asEXECUTION_FINISHED )
		CleanStack();

	asASSERT( m_needToCleanupArgs == false );

	// Release the returned object (if any)
	CleanReturnObject();

	// TODO: Unprepare is called during destruction, so nobody
	//       must be allowed to keep an extra reference
	asASSERT(m_refCount.get() == count);

	// Release the object if it is a script object
	if( m_initialFunction && m_initialFunction->objectType && (m_initialFunction->objectType->flags & asOBJ_SCRIPT_OBJECT) )
	{
		asCScriptObject *obj = *(asCScriptObject**)&m_regs.stackFramePointer[0];
		//if( obj )
			//obj->Release();
	}

	// Release the initial function
	if( m_initialFunction )
	{
		m_initialFunction->Release();

		// Reset stack pointer
		m_regs.stackPointer = m_originalStackPointer;
		m_stackIndex = m_originalStackIndex;

		// Make sure the stack pointer is pointing to the original position,
		// otherwise something is wrong with the way it is being updated
		asASSERT( IsNested() || m_stackIndex > 0 || (m_regs.stackPointer == m_stackBlocks[0] + m_stackBlockSize) );
	}

	// Clear function pointers
	m_initialFunction = 0;
	m_currentFunction = 0;
	m_exceptionFunction = 0;
	m_regs.programPointer = 0;

	// Reset status
	m_status = asEXECUTION_UNINITIALIZED;

	m_regs.stackFramePointer = 0;

	return 0;
}

asBYTE asCContext::GetReturnByte()
{
	return *(asBYTE*)&m_regs.valueRegister;
}

asWORD asCContext::GetReturnWord()
{
	return *(asWORD*)&m_regs.valueRegister;
}

asDWORD asCContext::GetReturnDWord()
{
	return *(asDWORD*)&m_regs.valueRegister;
}

asQWORD asCContext::GetReturnQWord()
{
	return m_regs.valueRegister;
}

float asCContext::GetReturnFloat()
{
	return *(float*)&m_regs.valueRegister;
}

double asCContext::GetReturnDouble()
{
	return *(double*)&m_regs.valueRegister;
}

void *asCContext::GetReturnAddress()
{
	if( m_status != asEXECUTION_FINISHED ) return 0;

	asCDataType *dt = &m_initialFunction->returnType;

	if( dt->IsReference() )
		return *(void**)&m_regs.valueRegister;
	else if( dt->IsObject() || dt->IsFuncdef() )
	{
		if( m_initialFunction->DoesReturnOnStack() )
		{
			// The address of the return value was passed as the first argument, after the object pointer
			int offset = 0;
			if( m_initialFunction->objectType )
				offset += AS_PTR_SIZE;

			return *(void**)(&m_regs.stackFramePointer[offset]);
		}

		return m_regs.objectRegister;
	}

	return 0;
}

void *asCContext::GetReturnObject()
{
	if( m_status != asEXECUTION_FINISHED ) return 0;

	asCDataType *dt = &m_initialFunction->returnType;

	if( !dt->IsObject() && !dt->IsFuncdef() ) return 0;

	if( dt->IsReference() )
		return *(void**)(asPWORD)m_regs.valueRegister;
	else
	{
		if( m_initialFunction->DoesReturnOnStack() )
		{
			// The address of the return value was passed as the first argument, after the object pointer
			int offset = 0;
			if( m_initialFunction->objectType )
				offset += AS_PTR_SIZE;

			return *(void**)(&m_regs.stackFramePointer[offset]);
		}

		return m_regs.objectRegister;
	}
}

void asCContext::SetReturnIntoValue(void* ReturnValue)
{
	asASSERT(m_initialFunction->DoesReturnOnStack());

	// The address of the return value was passed as the first argument, after the object pointer
	int offset = 0;
	if( m_initialFunction->objectType )
		offset += AS_PTR_SIZE;

	*(void**)(&m_regs.stackFramePointer[offset]) = ReturnValue;
	m_returnIsExternal = true;
}

void *asCContext::GetAddressOfReturnValue()
{
	if( m_status != asEXECUTION_FINISHED ) return 0;

	asCDataType *dt = &m_initialFunction->returnType;

	// An object is stored in the objectRegister
	if( !dt->IsReference() && (dt->IsObject() || dt->IsFuncdef()) )
	{
		// Need to dereference objects
		if( !dt->IsObjectHandle() )
		{
			if( m_initialFunction->DoesReturnOnStack() )
			{
				// The address of the return value was passed as the first argument, after the object pointer
				int offset = 0;
				if( m_initialFunction->objectType )
					offset += AS_PTR_SIZE;

				return *(void**)(&m_regs.stackFramePointer[offset]);
			}

			return *(void**)&m_regs.objectRegister;
		}
		return &m_regs.objectRegister;
	}

	// Primitives and references are stored in valueRegister
	return &m_regs.valueRegister;
}

int asCContext::SetObject(void *obj)
{
	*(asPWORD*)&m_regs.stackFramePointer[0] = (asPWORD)obj;
	return 0;
}

int asCContext::SetArgByte(asUINT arg, asBYTE value)
{
	// Set the value
	*(asBYTE*)&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	] = value;

	return 0;
}

int asCContext::SetArgWord(asUINT arg, asWORD value)
{
	// Set the value
	*(asWORD*)&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	] = value;

	return 0;
}

int asCContext::SetArgDWord(asUINT arg, asDWORD value)
{
	// Set the value
	*(asDWORD*)&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	] = value;

	return 0;
}

int asCContext::SetArgQWord(asUINT arg, asQWORD value)
{
	// Set the value
	*(asQWORD*)(&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	]) = value;

	return 0;
}

int asCContext::SetArgFloat(asUINT arg, float value)
{
	// Set the value
	*(float*)(&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	]) = value;

	return 0;
}

int asCContext::SetArgDouble(asUINT arg, double value)
{
	// Set the value
	*(double*)(&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	]) = value;

	return 0;
}

int asCContext::SetArgAddress(asUINT arg, void *value)
{
	// Set the value
	*(asPWORD*)(&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	]) = (asPWORD)value;
	return 0;
}

int asCContext::SetArgObject(asUINT arg, void *obj)
{
	// Set the value
	*(asPWORD*)(&m_regs.stackFramePointer[
		m_initialFunction->parameterOffsets[arg]
	]) = (asPWORD)obj;

	return 0;
}

int asCContext::SetArgVarType(asUINT arg, void *ptr, int typeId)
{
	if( m_status != asEXECUTION_PREPARED )
		return asCONTEXT_NOT_PREPARED;

	if( arg >= (unsigned)m_initialFunction->parameterTypes.GetLength() )
	{
		m_status = asEXECUTION_ERROR;
		return asINVALID_ARG;
	}

	// Verify the type of the argument
	asCDataType *dt = &m_initialFunction->parameterTypes[arg];
	if( dt->GetTokenType() != ttQuestion )
	{
		m_status = asEXECUTION_ERROR;
		return asINVALID_TYPE;
	}

	// Determine the position of the argument
	int offset = 0;
	if( m_initialFunction->objectType )
		offset += AS_PTR_SIZE;

	// If function returns object by value an extra pointer is pushed on the stack
	if( m_returnValueSize )
		offset += AS_PTR_SIZE;

	for( asUINT n = 0; n < arg; n++ )
		offset += m_initialFunction->parameterTypes[n].GetSizeOnStackDWords();

	// Set the typeId and pointer
	*(asPWORD*)(&m_regs.stackFramePointer[offset]) = (asPWORD)ptr;
	offset += AS_PTR_SIZE;
	*(int*)(&m_regs.stackFramePointer[offset]) = typeId;

	return 0;
}

// TODO: Instead of GetAddressOfArg, maybe we need a SetArgValue(int arg, void *value, bool takeOwnership) instead.

// interface
void *asCContext::GetAddressOfArg(asUINT arg)
{
	return &m_regs.stackFramePointer[m_initialFunction->parameterOffsets[arg]];
}


int asCContext::Abort()
{
	return asERROR;
}

// interface
int asCContext::Suspend()
{
	return asERROR;
}

// interface
int asCContext::Execute()
{
	asASSERT( m_engine != 0 );

	if( m_status != asEXECUTION_SUSPENDED && m_status != asEXECUTION_PREPARED )
	{
		asCString str;
		str.Format(TXT_FAILED_IN_FUNC_s_s_d, "Execute", errorNames[-asCONTEXT_NOT_PREPARED], asCONTEXT_NOT_PREPARED);
		m_engine->WriteMessage("", 0, 0, asMSGTYPE_ERROR, str.AddressOf());
		return asCONTEXT_NOT_PREPARED;
	}

	m_status = asEXECUTION_ACTIVE;

	FScopeSetActiveContext MakeActive(m_regs.tld, this);

	// We don't allow recursion beyond the point of sanity.
	// This lets scripters see better exceptions when they create infinite recursion.
	if (m_callStack.GetLength() > 20000)
	{
		SetInternalException("Stack overflow: potential infinite recursion detected?");
		return asEXECUTION_EXCEPTION;
	}

	// Make sure there are not too many nested calls, as it could crash the application 
	// by filling up the thread call stack
	if( m_regs.programPointer == 0 )
	{
		if( m_currentFunction->vfTableIdx != -1 && m_executeVirtualCall )
		{
			// The currentFunction is a virtual method
			// Determine the true function from the object
			asCScriptObject *obj = *(asCScriptObject**)(asPWORD*)m_regs.stackFramePointer;
			if( obj == 0 )
			{
				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return asEXECUTION_EXCEPTION;
			}
			else
			{
				asCObjectType *objType = (asCObjectType*)obj->GetObjectType();
				asCScriptFunction *realFunc = 0;

				if( objType->virtualFunctionTable.GetLength() > (asUINT)m_currentFunction->vfTableIdx )
				{
					realFunc = objType->virtualFunctionTable[m_currentFunction->vfTableIdx];
				}

				if( realFunc )
				{
					m_currentFunction = realFunc;
				}
				else
				{
					SetInternalException(TXT_NULL_POINTER_ACCESS);
					return asEXECUTION_EXCEPTION;
				}
			}
		}

		if( m_currentFunction->funcType == asFUNC_IMPORTED )
		{
			int funcId = m_engine->importedFunctions[m_currentFunction->id & ~FUNC_IMPORTED]->boundFunctionId;
			if( funcId > 0 )
				m_currentFunction = m_engine->scriptFunctions[funcId];
			else
				SetInternalException(TXT_UNBOUND_FUNCTION);
		}

		if( m_currentFunction->funcType == asFUNC_SCRIPT )
		{
			// Try to call the JIT function directly
			auto* jitFunction = m_currentFunction->jitFunction;
			if (jitFunction != nullptr)
			{
				FScriptExecution Execution(this);

				asQWORD outValue = 0;
				jitFunction(
					Execution,
					m_regs.stackFramePointer,
					&outValue);

				if (!Execution.bExceptionThrown)
				{
					m_status = asEXECUTION_FINISHED;

					// Copy returned value into the correct register for the return type
					if (!m_currentFunction->DoesReturnOnStack())
					{
						if (m_currentFunction->returnType.IsObject() && !m_currentFunction->returnType.IsReference())
							m_regs.objectRegister = (void*)outValue;
						else
							m_regs.valueRegister = outValue;
					}

					return m_status;
				}
				else
				{
					m_status = asEXECUTION_EXCEPTION;
					return m_status;
				}
			}
			else
			{
				m_regs.programPointer = m_currentFunction->scriptData->byteCode.AddressOf();

				// Set up the internal registers for executing the script function
				PrepareScriptFunction();
			}
		}
		else if( m_currentFunction->funcType == asFUNC_SYSTEM )
		{
			// The current function is an application registered function

			// Call the function directly
			CallSystemFunction(m_currentFunction->id, this);

			// Was the call successful?
			if( m_status == asEXECUTION_ACTIVE )
			{
				m_status = asEXECUTION_FINISHED;
			}
		}
		else
		{
			// This shouldn't happen unless there was an error in which
			// case an exception should have been raised already
			asASSERT( m_status == asEXECUTION_EXCEPTION );
		}
	}

	while (m_status == asEXECUTION_ACTIVE)
		ExecuteNext();

#if WITH_AS_DEBUGSERVER
	if( CanEverRunLineCallback )
	{
		if (ShouldAlwaysRunLineCallback || (m_currentFunction->module && m_currentFunction->module->hasBreakPoints))
		{
			// Call the line callback one last time before leaving
			// so anyone listening can catch the state change
			if (m_lineCallback)
				m_lineCallback(this);
		}

		if (m_stackPopCallback && m_callStack.GetLength() == 0)
		{
			m_stackPopCallback(this, m_stackBlocks[0] + GetStackFrameSize(0), m_stackBlocks[0]);
		}
	}
#endif

#if AS_REFERENCE_DEBUGGING
	if (m_callStack.GetLength() == 0)
	{
		ensureMsgf(TrackedReferences.Num() == 0, TEXT("Tracked reference leak. Script compiler did not correctly untrack a reference variable."));
		TrackedReferences.Reset();
	}
#endif

	if( m_status == asEXECUTION_FINISHED )
	{
		m_regs.objectType = m_initialFunction->returnType.GetTypeInfo();
		return asEXECUTION_FINISHED;
	}

	if( m_doAbort )
	{
		m_doAbort = false;

		m_status = asEXECUTION_ABORTED;
		return asEXECUTION_ABORTED;
	}

	if( m_status == asEXECUTION_SUSPENDED )
		return asEXECUTION_SUSPENDED;

	if( m_status == asEXECUTION_EXCEPTION )
		return asEXECUTION_EXCEPTION;

	return asERROR;
}

int asCContext::PushState()
{
	// Only allow the state to be pushed when active
	// TODO: Can we support a suspended state too? So the reuse of
	//       the context can be done outside the Execute() call?
	if( m_status != asEXECUTION_ACTIVE )
	{
		// TODO: Write message. Wrong usage
		return asERROR;
	}

	// Push the current script function that is calling the system function
	PushCallState();

	// Push the system function too, which will serve both as a marker and
	// informing which system function that created the nested call
	if( m_callStack.GetLength() == m_callStack.GetCapacity() )
	{
		// Allocate space for 10 call states at a time to save time
		m_callStack.AllocateNoConstruct(m_callStack.GetLength() + 10*CALLSTACK_FRAME_SIZE, true);
	}
	m_callStack.SetLengthNoConstruct(m_callStack.GetLength() + CALLSTACK_FRAME_SIZE);

	// Need to push m_initialFunction as it must be restored later
	asPWORD *tmp = m_callStack.AddressOf() + m_callStack.GetLength() - CALLSTACK_FRAME_SIZE;
	tmp[0] = 0;
	tmp[1] = (asPWORD)m_callingSystemFunction;
	tmp[2] = (asPWORD)m_initialFunction;
	tmp[3] = (asPWORD)m_originalStackPointer;
	tmp[4] = (asPWORD)m_argumentsSize;

	// Need to push the value of registers so they can be restored
	tmp[5] = (asPWORD)asDWORD(m_regs.valueRegister);
	tmp[6] = (asPWORD)asDWORD(m_regs.valueRegister>>32);
	tmp[7] = (asPWORD)m_regs.objectRegister;
	tmp[8] = (asPWORD)(m_returnIsExternal);
	tmp[9] = (asPWORD)m_originalStackIndex;
	tmp[10] = (asPWORD)m_blueprintStackFrameIndex;

	// Decrease stackpointer to prevent the top value from being overwritten
	m_regs.stackPointer -= 2;

	// Clear the initial function so that Prepare() knows it must do all validations
	m_initialFunction = 0;
	m_callingSystemFunction = nullptr;

	m_regs.objectRegister = 0;
	m_regs.objectType = 0;

	// Set the status to uninitialized as application
	// should call Prepare() after this to reuse the context
	m_status = asEXECUTION_UNINITIALIZED;

	return asSUCCESS;
}

int asCContext::PopState()
{
	if( !IsNested() )
		return asERROR;

	// Clean up the current execution
	Unprepare();

	// The topmost state must be a marker for nested call
	asASSERT( m_callStack[m_callStack.GetLength() - CALLSTACK_FRAME_SIZE] == 0 );

	// Restore the previous state
	asPWORD *tmp = &m_callStack[m_callStack.GetLength() - CALLSTACK_FRAME_SIZE];
	m_callStack.SetLength(m_callStack.GetLength() - CALLSTACK_FRAME_SIZE);

	// Restore the previous initial function and the associated values
	m_initialFunction      = reinterpret_cast<asCScriptFunction*>(tmp[2]);
	m_callingSystemFunction= reinterpret_cast<asCScriptFunction*>(tmp[1]);
	m_originalStackPointer = (asDWORD*)tmp[3];
	m_originalStackIndex   = (asUINT)tmp[9];
	m_blueprintStackFrameIndex = (asUINT)tmp[10];
	m_argumentsSize        = (int)tmp[4];
	m_returnIsExternal     = (bool)tmp[8];

	m_regs.valueRegister   = asQWORD(asDWORD(tmp[5]));
	m_regs.valueRegister  |= asQWORD(tmp[6])<<32;
	m_regs.objectRegister  = (void*)tmp[7];
	m_regs.objectType = nullptr;

	// Calculate the returnValueSize
	if( m_initialFunction->DoesReturnOnStack() )
		m_returnValueSize = m_initialFunction->returnType.GetSizeInMemoryDWords();
	else
		m_returnValueSize = 0;

	// Pop the current script function. This will also restore the previous stack pointer
	PopCallState();

	m_status = asEXECUTION_ACTIVE;

	return asSUCCESS;
}

void asCContext::PushCallState()
{
	asUINT oldLength = m_callStack.GetLength();
	if( oldLength == m_callStack.GetCapacity() )
	{
		// Allocate space for 10 call states at a time to save time
		m_callStack.AllocateNoConstruct(oldLength + 10*CALLSTACK_FRAME_SIZE, true);
	}
	m_callStack.SetLengthNoAllocate(oldLength + CALLSTACK_FRAME_SIZE);

    // Separating the loads and stores limits data cache trash, and with a smart compiler
    // could turn into SIMD style loading/storing if available.
    // The compiler can't do this itself due to potential pointer aliasing between the pointers,
    // ie writing to tmp could overwrite the data contained in registers.stackFramePointer for example
    // for all the compiler knows. So introducing the local variable s, which is never referred to by
    // its address we avoid this issue.

	asPWORD s[6];
	s[0] = (asPWORD)m_regs.stackFramePointer;
	s[1] = (asPWORD)m_currentFunction;
	s[2] = (asPWORD)m_regs.programPointer;
	s[3] = (asPWORD)m_regs.stackPointer;
	s[4] = m_stackIndex;
	s[5] = (asPWORD)m_blueprintStackFrameIndex;

	asPWORD *tmp = m_callStack.AddressOf() + oldLength;
	tmp[0] = s[0];
	tmp[1] = s[1];
	tmp[2] = s[2];
	tmp[3] = s[3];
	tmp[4] = s[4];
	tmp[10] = s[5];
}

void asCContext::PopCallState()
{
	// See comments in PushCallState about pointer aliasing and data cache trashing
	asUINT newLength = m_callStack.GetLength() - CALLSTACK_FRAME_SIZE;
	asPWORD *tmp = m_callStack.AddressOf() + newLength;

	asPWORD s[6];
	s[0] = tmp[0];
	s[1] = tmp[1];
	s[2] = tmp[2];
	s[3] = tmp[3];
	s[4] = tmp[4];
	s[5] = tmp[10];

	asDWORD* oldStackPointer = m_regs.stackPointer;
	const int oldStackIndex = m_stackIndex;

	m_regs.stackFramePointer = (asDWORD*)s[0];
	m_currentFunction        = (asCScriptFunction*)s[1];
	m_regs.programPointer    = (asDWORD*)s[2];
	m_regs.stackPointer      = (asDWORD*)s[3];
	m_stackIndex             = (int)s[4];
	m_blueprintStackFrameIndex = (asUINT)s[5];

#if WITH_AS_DEBUGSERVER
	if (CanEverRunLineCallback && m_stackPopCallback)
	{
		// Entire stack frame becomes invalidated
		if (oldStackIndex != m_stackIndex)
		{
			m_stackPopCallback(this, m_stackBlocks[oldStackIndex] + GetStackFrameSize(oldStackIndex), m_stackBlocks[oldStackIndex]);
		}
		// Part of current stack becomes invalidated
		else
		{
			m_stackPopCallback(this, m_regs.stackPointer, oldStackPointer);
		}
	}
#endif

	m_callStack.SetLengthNoAllocate(newLength);
}

// interface
asUINT asCContext::GetCallstackSize() const
{
	if( m_currentFunction == 0 ) return 0;

	// The current function is accessed at stackLevel 0
	return asUINT(1 + m_callStack.GetLength() / CALLSTACK_FRAME_SIZE);
}

// interface
asIScriptFunction *asCContext::GetFunction(asUINT stackLevel)
{
	if( stackLevel >= GetCallstackSize() ) return 0;

	if( stackLevel == 0 ) return m_currentFunction;

	asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize() - stackLevel - 1)*CALLSTACK_FRAME_SIZE;
	asCScriptFunction *func = (asCScriptFunction*)s[1];

	return func;
}

asUINT asCContext::GetBlueprintCallstackFrame(asUINT stackLevel)
{
	if( stackLevel >= GetCallstackSize() ) return 0;

	if( stackLevel == 0 ) return m_blueprintStackFrameIndex;

	asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize() - stackLevel - 1)*CALLSTACK_FRAME_SIZE;
	return (asUINT)s[10];
}

// interface
int asCContext::GetLineNumber(asUINT stackLevel, int *column, const char **sectionName)
{
	if( stackLevel >= GetCallstackSize() ) return asINVALID_ARG;

	asCScriptFunction *func;
	asDWORD *bytePos;
	if( stackLevel == 0 )
	{
		func = m_currentFunction;
		if( func->scriptData == 0 ) return 0;
		bytePos = m_regs.programPointer;
	}
	else
	{
		asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize()-stackLevel-1)*CALLSTACK_FRAME_SIZE;
		func = (asCScriptFunction*)s[1];
		if( func == 0 || func->scriptData == 0 ) return 0;
		bytePos = (asDWORD*)s[2];

		// Subract 1 from the bytePos, because we want the line where
		// the call was made, and not the instruction after the call
		bytePos -= 1;
	}

	// For nested calls it is possible that func is null
	if( func == 0 )
	{
		if( column ) *column = 0;
		if( sectionName ) *sectionName = 0;
		return 0;
	}

	int sectionIdx;
	asDWORD line = func->GetLineNumber(int(bytePos - func->scriptData->byteCode.AddressOf()), &sectionIdx);
	if( column ) *column = (line >> 20);
	if( sectionName )
	{
		asASSERT( sectionIdx < int(m_engine->scriptSectionNames.GetLength()) );
		if( sectionIdx >= 0 && asUINT(sectionIdx) < m_engine->scriptSectionNames.GetLength() )
			*sectionName = m_engine->scriptSectionNames[sectionIdx]->AddressOf();
		else
			*sectionName = 0;
	}
	return (line & 0xFFFFF);
}

// internal
bool asCContext::ReserveStackSpace(asUINT size)
{
#ifdef WIP_16BYTE_ALIGN
	// Pad size to a multiple of MAX_TYPE_ALIGNMENT.
	const asUINT remainder = size % MAX_TYPE_ALIGNMENT;
	if(remainder != 0)
	{
		size = size + (MAX_TYPE_ALIGNMENT - (size % MAX_TYPE_ALIGNMENT));
	}
#endif

	// Make sure the first stack block is allocated
	if( m_stackBlocks.GetLength() == 0 )
	{
		m_stackBlockSize = m_engine->initialContextStackSize;
		asASSERT( m_stackBlockSize > 0 );

#ifndef WIP_16BYTE_ALIGN
		asDWORD *stack = asNEWARRAY(asDWORD,m_stackBlockSize);
#else
		asDWORD *stack = asNEWARRAYALIGNED(asDWORD, m_stackBlockSize, MAX_TYPE_ALIGNMENT);
#endif
		if( stack == 0 )
		{
			// Out of memory
			return false;
		}

#ifdef WIP_16BYTE_ALIGN
		asASSERT( isAligned(stack, MAX_TYPE_ALIGNMENT) );
#endif

		m_stackBlocks.PushLast(stack);
		m_stackIndex = 0;
		m_regs.stackPointer = m_stackBlocks[0] + m_stackBlockSize;

#ifdef WIP_16BYTE_ALIGN
		// Align the stack pointer. This is necessary as the m_stackBlockSize is not necessarily evenly divisable with the max alignment
		((asPWORD&)m_regs.stackPointer) &= ~(MAX_TYPE_ALIGNMENT-1);

		asASSERT( isAligned(m_regs.stackPointer, MAX_TYPE_ALIGNMENT) );
#endif
	}

	// Check if there is enough space on the current stack block, otherwise move
	// to the next one. New and larger blocks will be allocated as necessary
	while( m_regs.stackPointer - (size + RESERVE_STACK) < m_stackBlocks[m_stackIndex] )
	{
		// Make sure we don't allocate more space than allowed
		if( m_engine->ep.maximumContextStackSize )
		{
			// This test will only stop growth once it has already crossed the limit
			if( m_stackBlockSize * ((1 << (m_stackIndex+1)) - 1) > m_engine->ep.maximumContextStackSize )
			{
				m_isStackMemoryNotAllocated = true;

				// Set the stackFramePointer, even though the stackPointer wasn't updated
				m_regs.stackFramePointer = m_regs.stackPointer;

				SetInternalException(TXT_STACK_OVERFLOW);
				return false;
			}
		}

		m_stackIndex++;
		if( m_stackBlocks.GetLength() == m_stackIndex )
		{
			// Allocate the new stack block, with twice the size of the previous
			int newStackFrameSize = GetStackFrameSize(m_stackIndex);
#ifndef WIP_16BYTE_ALIGN
			asDWORD *stack = asNEWARRAY(asDWORD, newStackFrameSize);
#else
			asDWORD *stack = asNEWARRAYALIGNED(asDWORD, newStackFrameSize, MAX_TYPE_ALIGNMENT);
#endif
			if( stack == 0 )
			{
				// Out of memory
				m_isStackMemoryNotAllocated = true;

				// Set the stackFramePointer, even though the stackPointer wasn't updated
				m_regs.stackFramePointer = m_regs.stackPointer;

				SetInternalException(TXT_STACK_OVERFLOW);
				return false;
			}

#ifdef WIP_16BYTE_ALIGN
			asASSERT( isAligned(stack, MAX_TYPE_ALIGNMENT) );
#endif

			m_stackBlocks.PushLast(stack);
		}

		// Update the stack pointer to point to the new block.
		// Leave enough room above the stackpointer to copy the arguments from the previous stackblock
		m_regs.stackPointer = m_stackBlocks[m_stackIndex] +
			GetStackFrameSize(m_stackIndex) -
			m_currentFunction->totalSpaceBeforeFunction - 16;

#ifdef WIP_16BYTE_ALIGN
		// Align the stack pointer 
		(asPWORD&)m_regs.stackPointer &= ~(MAX_TYPE_ALIGNMENT-1);

		asASSERT( isAligned(m_regs.stackPointer, MAX_TYPE_ALIGNMENT) );
#endif
	}

	return true;
}

// internal
void asCContext::CallScriptFunction(asCScriptFunction *func)
{
	// Try to call the JIT function directly
	auto* jitFunction = func->jitFunction;
	if (jitFunction != nullptr)
	{
		FScriptExecution Execution(this);

		asQWORD outValue = 0;
		jitFunction(
			Execution,
			m_regs.stackPointer,
			&outValue);

		if (!Execution.bExceptionThrown)
		{
			// Copy returned value into the correct register for the return type
			if (!func->DoesReturnOnStack())
			{
				if (func->returnType.IsObject() && !func->returnType.IsReference())
					m_regs.objectRegister = (void*)outValue;
				else
					m_regs.valueRegister = outValue;
			}

			m_regs.stackPointer += func->totalSpaceBeforeFunction;
			return;
		}
		else
		{
			m_status = asEXECUTION_EXCEPTION;
			return;
		}
	}

	// We don't allow recursion beyond the point of sanity.
	// This lets scripters see better exceptions when they create infinite recursion.
	if (m_callStack.GetLength() > 10000)
	{
		SetInternalException("Stack overflow: potential infinite recursion detected?");
		return;
	}

	asASSERT( func->scriptData );

	// Push the framepointer, function id and programCounter on the stack
	PushCallState();

	// Update the current function and program position before increasing the stack
	// so the exception handler will know what to do if there is a stack overflow
	m_currentFunction = func;
	m_regs.programPointer = m_currentFunction->scriptData->byteCode.AddressOf();

	PrepareScriptFunction();
}

void asCContext::PrepareScriptFunction()
{
	asASSERT( m_currentFunction->scriptData );

	// Make sure there is space on the stack to execute the function
	asDWORD *oldStackPointer = m_regs.stackPointer;
	asUINT needSize = m_currentFunction->scriptData->stackNeeded;

	// With a quick check we know right away that we don't need to call ReserveStackSpace and do other checks inside it
	if (m_stackBlocks.GetLength() == 0 || oldStackPointer - (needSize + RESERVE_STACK) < m_stackBlocks[m_stackIndex])
	{
		if( !ReserveStackSpace(needSize) )
			return;
	}

	m_regs.stackPointer = (asDWORD*)(((asPWORD)m_regs.stackPointer) & ~(16 - 1));
	if( m_regs.stackPointer != oldStackPointer )
	{
		int numDwords = m_currentFunction->totalSpaceBeforeFunction;
		memmove(m_regs.stackPointer, oldStackPointer, sizeof(asDWORD)*numDwords);
	}

	// Update framepointer
	m_regs.stackFramePointer = m_regs.stackPointer;

	// Set all object variables to 0 to guarantee that they are null before they are used
	// Only variables on the heap should be cleared. The rest will be cleared by calling the constructor
	asUINT n = m_currentFunction->scriptData->objVariablesOnHeap;
	while( n-- > 0 )
	{
		int pos = m_currentFunction->scriptData->objVariablePos[n];
		*(asPWORD*)&m_regs.stackFramePointer[-pos] = 0;
	}

	// Initialize the stack pointer with the space needed for local variables
	m_regs.stackPointer -= m_currentFunction->scriptData->variableSpace;
}

bool asCContext::CallInterfaceMethod(asCScriptFunction *func)
{
	// Resolve the interface method using the current script type
	asCScriptObject *obj = *(asCScriptObject**)(asPWORD*)m_regs.stackPointer;
	if( obj == 0 )
	{
		// Tell the exception handler to clean up the arguments to this method
		m_needToCleanupArgs = true;
		SetInternalException(TXT_NULL_POINTER_ACCESS);
		return false;
	}

	asCObjectType *objType = (asCObjectType*)obj->GetObjectType();

	// Search the object type for a function that matches the interface function
	asCScriptFunction *realFunc = 0;
	if( func->funcType == asFUNC_INTERFACE )
	{
		// Find the offset for the interface's virtual function table chunk
		asUINT offset = 0;
		bool found = false;
		asCObjectType *findInterface = func->objectType;

		// TODO: runtime optimize: The list of interfaces should be ordered by the address
		//                         Then a binary search pattern can be used.
		asUINT intfCount = asUINT(objType->interfaces.GetLength());
		for( asUINT n = 0; n < intfCount; n++ )
		{
			if( objType->interfaces[n] == findInterface )
			{
				offset = objType->interfaceVFTOffsets[n];
				found = true;
				break;
			}
		}

		if( !found )
		{
			// Tell the exception handler to clean up the arguments to this method
			m_needToCleanupArgs = true;
			SetInternalException(TXT_NULL_POINTER_ACCESS);
			return false;
		}

		// Find the real function in the virtual table chunk with the found offset
		realFunc = objType->virtualFunctionTable[func->vfTableIdx + offset];

		// Since the interface was implemented by the class, it shouldn't
		// be possible that the real function isn't found
		asASSERT( realFunc );
	}
	else if( func->vfTableIdx != -1 )
	{
		realFunc = objType->virtualFunctionTable[func->vfTableIdx];
	}

	// Then call the true script function
	CallScriptFunction(realFunc);
	return true;
}

void asCContext::ExecuteNext()
{
	asDWORD *l_bc = m_regs.programPointer;
	asDWORD *l_sp = m_regs.stackPointer;
	asDWORD *l_fp = m_regs.stackFramePointer;

	for(;;)
	{

#ifdef AS_DEBUG
	// Gather statistics on executed bytecode
	stats.Instr(*(asBYTE*)l_bc);

	// Used to verify that the size of the instructions are correct
	asDWORD *old = l_bc;
#endif


	// Remember to keep the cases in order and without
	// gaps, because that will make the switch faster.
	// It will be faster since only one lookup will be
	// made to find the correct jump destination. If not
	// in order, the switch will make two lookups.
	switch( *(asBYTE*)l_bc )
	{
//--------------
// memory access functions

	case asBC_PopPtr:
		// Pop a pointer from the stack
		l_sp += AS_PTR_SIZE;
		l_bc++;
		break;

	case asBC_PshGPtr:
		// Replaces PGA + RDSPtr
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = *(asPWORD*)asBC_PTRARG(l_bc);
		l_bc += 1 + AS_PTR_SIZE;
		break;

	// Push a dword value on the stack
	case asBC_PshC4:
		--l_sp;
		*l_sp = asBC_DWORDARG(l_bc);
		l_bc += 2;
		break;

	// Push the dword value of a variable on the stack
	case asBC_PshV4:
		--l_sp;
		*l_sp = *(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	// Push the address of a variable on the stack
	case asBC_PSF:
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = asPWORD(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	// Swap the top 2 pointers on the stack
	case asBC_SwapPtr:
		{
			asPWORD p = *(asPWORD*)l_sp;
			*(asPWORD*)l_sp = *(asPWORD*)(l_sp+AS_PTR_SIZE);
			*(asPWORD*)(l_sp+AS_PTR_SIZE) = p;
			l_bc++;
		}
		break;

	// Do a boolean not operation, modifying the value of the variable
	case asBC_NOT:
#if AS_SIZEOF_BOOL == 1
		{
			// Set the value to true if it is equal to 0

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on the pointer.

			volatile asBYTE *ptr = (asBYTE*)(l_fp - asBC_SWORDARG0(l_bc));
			asBYTE val = (ptr[0] == 0) ? VALUE_OF_BOOLEAN_TRUE : 0;
			ptr[0] = val; // The result is stored in the lower byte
			ptr[1] = 0;   // Make sure the rest of the DWORD is 0
			ptr[2] = 0;
			ptr[3] = 0;
		}
#else
		*(l_fp - asBC_SWORDARG0(l_bc)) = (*(l_fp - asBC_SWORDARG0(l_bc)) == 0 ? VALUE_OF_BOOLEAN_TRUE : 0);
#endif
		l_bc++;
		break;

	// Push the dword value of a global variable on the stack
	case asBC_PshG4:
		--l_sp;
		*l_sp = *(asDWORD*)asBC_PTRARG(l_bc);
		l_bc += 1 + AS_PTR_SIZE;
		break;

	// Load the address of a global variable in the register, then
	// copy the value of the global variable into a local variable
	case asBC_LdGRdR4:
		*(void**)&m_regs.valueRegister = (void*)asBC_PTRARG(l_bc);
		*(l_fp - asBC_SWORDARG0(l_bc)) = **(asDWORD**)&m_regs.valueRegister;
		l_bc += 1+AS_PTR_SIZE;
		break;

//----------------
// path control instructions

	// Begin execution of a script function
	case asBC_CALL:
		{
			int i = asBC_INTARG(l_bc);
			l_bc += 2;

			asASSERT( i >= 0 );
			asASSERT( (i & FUNC_IMPORTED) == 0 );

			// Need to move the values back to the context
			m_regs.programPointer = l_bc;
			m_regs.stackPointer = l_sp;
			m_regs.stackFramePointer = l_fp;

			CallScriptFunction(m_engine->scriptFunctions[i]);

			// Extract the values from the context again
			l_bc = m_regs.programPointer;
			l_sp = m_regs.stackPointer;
			l_fp = m_regs.stackFramePointer;

			// If status isn't active anymore then we must stop
			if( m_status != asEXECUTION_ACTIVE )
				return;
		}
		break;

	// Return to the caller, and remove the arguments from the stack
	case asBC_RET:
		{
			// Return if this was the first function, or a nested execution
			if( m_callStack.GetLength() == 0 ||
				m_callStack[m_callStack.GetLength() - CALLSTACK_FRAME_SIZE] == 0 )
			{
				m_status = asEXECUTION_FINISHED;
				return;
			}

			asWORD w = asBC_WORDARG0(l_bc);

			// Read the old framepointer, functionid, and programCounter from the call stack
			PopCallState();

			// Extract the values from the context again
			l_bc = m_regs.programPointer;
			l_sp = m_regs.stackPointer;
			l_fp = m_regs.stackFramePointer;

			// Pop arguments from stack
			l_sp += w;
		}
		break;

	// Jump to a relative position
	case asBC_JMP:
		l_bc += 2 + asBC_INTARG(l_bc);
		break;

//----------------
// Conditional jumps

	// Jump to a relative position if the value in the register is 0
	case asBC_JZ:
		if( *(int*)&m_regs.valueRegister == 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;

	// Jump to a relative position if the value in the register is not 0
	case asBC_JNZ:
		if( *(int*)&m_regs.valueRegister != 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;

	// Jump to a relative position if the value in the register is negative
	case asBC_JS:
		if( *(int*)&m_regs.valueRegister < 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;

	// Jump to a relative position if the value in the register it not negative
	case asBC_JNS:
		if( *(int*)&m_regs.valueRegister >= 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;

	// Jump to a relative position if the value in the register is greater than 0
	case asBC_JP:
		if( *(int*)&m_regs.valueRegister > 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;

	// Jump to a relative position if the value in the register is not greater than 0
	case asBC_JNP:
		if( *(int*)&m_regs.valueRegister <= 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;
//--------------------
// test instructions

	// If the value in the register is 0, then set the register to 1, else to 0
	case asBC_TZ:
#if AS_SIZEOF_BOOL == 1
		{
			// Set the value to true if it is equal to 0

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on valueRegister.

			volatile int    *regPtr  = (int*)&m_regs.valueRegister;
			volatile asBYTE *regBptr = (asBYTE*)&m_regs.valueRegister;
			asBYTE val = (regPtr[0] == 0) ? VALUE_OF_BOOLEAN_TRUE : 0;
			regBptr[0] = val; // The result is stored in the lower byte
			regBptr[1] = 0;   // Make sure the rest of the register is 0
			regBptr[2] = 0;
			regBptr[3] = 0;
			regBptr[4] = 0;
			regBptr[5] = 0;
			regBptr[6] = 0;
			regBptr[7] = 0;
		}
#else
		*(int*)&m_regs.valueRegister = (*(int*)&m_regs.valueRegister == 0 ? VALUE_OF_BOOLEAN_TRUE : 0);
#endif
		l_bc++;
		break;

	// If the value in the register is not 0, then set the register to 1, else to 0
	case asBC_TNZ:
#if AS_SIZEOF_BOOL == 1
		{
			// Set the value to true if it is not equal to 0

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on valueRegister.

			volatile int    *regPtr  = (int*)&m_regs.valueRegister;
			volatile asBYTE *regBptr = (asBYTE*)&m_regs.valueRegister;
			asBYTE val = (regPtr[0] == 0) ? 0 : VALUE_OF_BOOLEAN_TRUE;
			regBptr[0] = val; // The result is stored in the lower byte
			regBptr[1] = 0;   // Make sure the rest of the register is 0
			regBptr[2] = 0;
			regBptr[3] = 0;
			regBptr[4] = 0;
			regBptr[5] = 0;
			regBptr[6] = 0;
			regBptr[7] = 0;
		}
#else
		*(int*)&m_regs.valueRegister = (*(int*)&m_regs.valueRegister == 0 ? 0 : VALUE_OF_BOOLEAN_TRUE);
#endif
		l_bc++;
		break;

	// If the value in the register is negative, then set the register to 1, else to 0
	case asBC_TS:
#if AS_SIZEOF_BOOL == 1
		{
			// Set the value to true if it is less than 0

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on valueRegister.

			volatile int    *regPtr  = (int*)&m_regs.valueRegister;
			volatile asBYTE *regBptr = (asBYTE*)&m_regs.valueRegister;
			asBYTE val = (regPtr[0] < 0) ? VALUE_OF_BOOLEAN_TRUE : 0;
			regBptr[0] = val; // The result is stored in the lower byte
			regBptr[1] = 0;   // Make sure the rest of the register is 0
			regBptr[2] = 0;
			regBptr[3] = 0;
			regBptr[4] = 0;
			regBptr[5] = 0;
			regBptr[6] = 0;
			regBptr[7] = 0;
		}
#else
		*(int*)&m_regs.valueRegister = (*(int*)&m_regs.valueRegister < 0 ? VALUE_OF_BOOLEAN_TRUE : 0);
#endif
		l_bc++;
		break;

	// If the value in the register is not negative, then set the register to 1, else to 0
	case asBC_TNS:
#if AS_SIZEOF_BOOL == 1
		{
			// Set the value to true if it is not less than 0

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on valueRegister.

			volatile int    *regPtr  = (int*)&m_regs.valueRegister;
			volatile asBYTE *regBptr = (asBYTE*)&m_regs.valueRegister;
			asBYTE val = (regPtr[0] >= 0) ? VALUE_OF_BOOLEAN_TRUE : 0;
			regBptr[0] = val; // The result is stored in the lower byte
			regBptr[1] = 0;   // Make sure the rest of the register is 0
			regBptr[2] = 0;
			regBptr[3] = 0;
			regBptr[4] = 0;
			regBptr[5] = 0;
			regBptr[6] = 0;
			regBptr[7] = 0;
		}
#else
		*(int*)&m_regs.valueRegister = (*(int*)&m_regs.valueRegister < 0 ? 0 : VALUE_OF_BOOLEAN_TRUE);
#endif
		l_bc++;
		break;

	// If the value in the register is greater than 0, then set the register to 1, else to 0
	case asBC_TP:
#if AS_SIZEOF_BOOL == 1
		{
			// Set the value to true if it is greater than 0

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on valueRegister.

			volatile int    *regPtr  = (int*)&m_regs.valueRegister;
			volatile asBYTE *regBptr = (asBYTE*)&m_regs.valueRegister;
			asBYTE val = (regPtr[0] > 0) ? VALUE_OF_BOOLEAN_TRUE : 0;
			regBptr[0] = val; // The result is stored in the lower byte
			regBptr[1] = 0;   // Make sure the rest of the register is 0
			regBptr[2] = 0;
			regBptr[3] = 0;
			regBptr[4] = 0;
			regBptr[5] = 0;
			regBptr[6] = 0;
			regBptr[7] = 0;
		}
#else
		*(int*)&m_regs.valueRegister = (*(int*)&m_regs.valueRegister > 0 ? VALUE_OF_BOOLEAN_TRUE : 0);
#endif
		l_bc++;
		break;

	// If the value in the register is not greater than 0, then set the register to 1, else to 0
	case asBC_TNP:
#if AS_SIZEOF_BOOL == 1
		{
			// Set the value to true if it is not greater than 0

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on valueRegister.

			volatile int    *regPtr  = (int*)&m_regs.valueRegister;
			volatile asBYTE *regBptr = (asBYTE*)&m_regs.valueRegister;
			asBYTE val = (regPtr[0] <= 0) ? VALUE_OF_BOOLEAN_TRUE : 0;
			regBptr[0] = val; // The result is stored in the lower byte
			regBptr[1] = 0;   // Make sure the rest of the register is 0
			regBptr[2] = 0;
			regBptr[3] = 0;
			regBptr[4] = 0;
			regBptr[5] = 0;
			regBptr[6] = 0;
			regBptr[7] = 0;
		}
#else
		*(int*)&m_regs.valueRegister = (*(int*)&m_regs.valueRegister > 0 ? 0 : VALUE_OF_BOOLEAN_TRUE);
#endif
		l_bc++;
		break;

//--------------------
// negate value

	// Negate the integer value in the variable
	case asBC_NEGi:
		*(l_fp - asBC_SWORDARG0(l_bc)) = asDWORD(-int(*(l_fp - asBC_SWORDARG0(l_bc))));
		l_bc++;
		break;

	// Negate the float value in the variable
	case asBC_NEGf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = -*(float*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	// Negate the double value in the variable
	case asBC_NEGd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = -*(double*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

//-------------------------
// Increment value pointed to by address in register

	// Increment the short value pointed to by the register
	case asBC_INCi16:
		(**(short**)&m_regs.valueRegister)++;
		l_bc++;
		break;

	// Increment the byte value pointed to by the register
	case asBC_INCi8:
		(**(char**)&m_regs.valueRegister)++;
		l_bc++;
		break;

	// Decrement the short value pointed to by the register
	case asBC_DECi16:
		(**(short**)&m_regs.valueRegister)--;
		l_bc++;
		break;

	// Decrement the byte value pointed to by the register
	case asBC_DECi8:
		(**(char**)&m_regs.valueRegister)--;
		l_bc++;
		break;

	// Increment the integer value pointed to by the register
	case asBC_INCi:
		++(**(int**)&m_regs.valueRegister);
		l_bc++;
		break;

	// Decrement the integer value pointed to by the register
	case asBC_DECi:
		--(**(int**)&m_regs.valueRegister);
		l_bc++;
		break;

	// Increment the float value pointed to by the register
	case asBC_INCf:
		++(**(float**)&m_regs.valueRegister);
		l_bc++;
		break;

	// Decrement the float value pointed to by the register
	case asBC_DECf:
		--(**(float**)&m_regs.valueRegister);
		l_bc++;
		break;

	// Increment the double value pointed to by the register
	case asBC_INCd:
		++(**(double**)&m_regs.valueRegister);
		l_bc++;
		break;

	// Decrement the double value pointed to by the register
	case asBC_DECd:
		--(**(double**)&m_regs.valueRegister);
		l_bc++;
		break;

	// Increment the local integer variable
	case asBC_IncVi:
		(*(int*)(l_fp - asBC_SWORDARG0(l_bc)))++;
		l_bc++;
		break;

	// Decrement the local integer variable
	case asBC_DecVi:
		(*(int*)(l_fp - asBC_SWORDARG0(l_bc)))--;
		l_bc++;
		break;

//--------------------
// bits instructions

	// Do a bitwise not on the value in the variable
	case asBC_BNOT:
		*(l_fp - asBC_SWORDARG0(l_bc)) = ~*(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	// Do a bitwise and of two variables and store the result in a third variable
	case asBC_BAND:
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(l_fp - asBC_SWORDARG1(l_bc)) & *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	// Do a bitwise or of two variables and store the result in a third variable
	case asBC_BOR:
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(l_fp - asBC_SWORDARG1(l_bc)) | *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	// Do a bitwise xor of two variables and store the result in a third variable
	case asBC_BXOR:
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(l_fp - asBC_SWORDARG1(l_bc)) ^ *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	// Do a logical shift left of two variables and store the result in a third variable
	case asBC_BSLL:
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(l_fp - asBC_SWORDARG1(l_bc)) << *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	// Do a logical shift right of two variables and store the result in a third variable
	case asBC_BSRL:
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(l_fp - asBC_SWORDARG1(l_bc)) >> *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	// Do an arithmetic shift right of two variables and store the result in a third variable
	case asBC_BSRA:
		*(l_fp - asBC_SWORDARG0(l_bc)) = int(*(l_fp - asBC_SWORDARG1(l_bc))) >> *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_COPY:
		{
			void *d = (void*)*(asPWORD*)l_sp; l_sp += AS_PTR_SIZE;
			void *s = (void*)*(asPWORD*)l_sp;
			if( s == 0 || d == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}
			memcpy(d, s, asBC_WORDARG0(l_bc));

			// replace the pointer on the stack with the lvalue
			*(asPWORD**)l_sp = (asPWORD*)d;
		}
		l_bc += 2;
		break;

	case asBC_PshC8:
		l_sp -= 2;
		*(asQWORD*)l_sp = asBC_QWORDARG(l_bc);
		l_bc += 3;
		break;

	case asBC_PshVPtr:
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = *(asPWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_RDSPtr:
		{
			// The pointer must not be null
			asPWORD a = *(asPWORD*)l_sp;
			if( a == 0 )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}
			// Pop an address from the stack, read a pointer from that address and push it on the stack
			*(asPWORD*)l_sp = *(asPWORD*)a;
		}
		l_bc++;
		break;

	//----------------------------
	// Comparisons
	case asBC_CMPd:
		{
			// Do a comparison of the values, rather than a subtraction
			// in order to get proper behaviour for infinity values.
			double dbl1 = *(double*)(l_fp - asBC_SWORDARG0(l_bc));
			double dbl2 = *(double*)(l_fp - asBC_SWORDARG1(l_bc));
			if( dbl1 == dbl2 )     *(int*)&m_regs.valueRegister =  0;
			else if( dbl1 < dbl2 ) *(int*)&m_regs.valueRegister = -1;
			else                   *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_CMPu:
		{
			asDWORD d1 = *(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc));
			asDWORD d2 = *(asDWORD*)(l_fp - asBC_SWORDARG1(l_bc));
			if( d1 == d2 )     *(int*)&m_regs.valueRegister =  0;
			else if( d1 < d2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_CMPf:
		{
			// Do a comparison of the values, rather than a subtraction
			// in order to get proper behaviour for infinity values.
			float f1 = *(float*)(l_fp - asBC_SWORDARG0(l_bc));
			float f2 = *(float*)(l_fp - asBC_SWORDARG1(l_bc));
			if( f1 == f2 )     *(int*)&m_regs.valueRegister =  0;
			else if( f1 < f2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_CMPi:
		{
			int i1 = *(int*)(l_fp - asBC_SWORDARG0(l_bc));
			int i2 = *(int*)(l_fp - asBC_SWORDARG1(l_bc));
			if( i1 == i2 )     *(int*)&m_regs.valueRegister =  0;
			else if( i1 < i2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	//----------------------------
	// Comparisons with constant value
	case asBC_CMPIi:
		{
			int i1 = *(int*)(l_fp - asBC_SWORDARG0(l_bc));
			int i2 = asBC_INTARG(l_bc);
			if( i1 == i2 )     *(int*)&m_regs.valueRegister =  0;
			else if( i1 < i2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_CMPIf:
		{
			// Do a comparison of the values, rather than a subtraction
			// in order to get proper behaviour for infinity values.
			float f1 = *(float*)(l_fp - asBC_SWORDARG0(l_bc));
			float f2 = asBC_FLOATARG(l_bc);
			if( f1 == f2 )     *(int*)&m_regs.valueRegister =  0;
			else if( f1 < f2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_CMPIu:
		{
			asDWORD d1 = *(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc));
			asDWORD d2 = asBC_DWORDARG(l_bc);
			if( d1 == d2 )     *(int*)&m_regs.valueRegister =  0;
			else if( d1 < d2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_JMPP:
		l_bc += 2 + (*(int*)(l_fp - asBC_SWORDARG0(l_bc)))*2;
		break;

	case asBC_PopRPtr:
		*(asPWORD*)&m_regs.valueRegister = *(asPWORD*)l_sp;
		l_sp += AS_PTR_SIZE;
		l_bc++;
		break;

	case asBC_PshRPtr:
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = *(asPWORD*)&m_regs.valueRegister;
		l_bc++;
		break;

	case asBC_STR:
		// TODO: NEWSTRING: Deprecate this instruction
		asASSERT(false);
		l_bc++;
		break;

	case asBC_CALLSYS:
		{
			// Get function ID from the argument
			asCScriptFunction* Function = (asCScriptFunction*)asBC_PTRARG(l_bc);

			// Need to move the values back to the context as the called functions
			// may use the debug interface to inspect the registers
			m_regs.programPointer    = l_bc;
			m_regs.stackPointer      = l_sp;
			m_regs.stackFramePointer = l_fp;

			l_sp += CallSystemFunction(Function->id, this);

			// Update the program position after the call so that line number is correct
			l_bc += 3;

			// An exception might have been raised
			if( m_status != asEXECUTION_ACTIVE )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				return;
			}
		}
		break;

	case asBC_CALLBND:
		{
			// TODO: Clean-up: This code is very similar to asBC_CallPtr. Create a shared method for them
			// Get the function ID from the stack
			int i = asBC_INTARG(l_bc);

			asASSERT( i >= 0 );
			asASSERT( i & FUNC_IMPORTED );

			// Need to move the values back to the context
			m_regs.programPointer    = l_bc;
			m_regs.stackPointer      = l_sp;
			m_regs.stackFramePointer = l_fp;

			int funcId = m_engine->importedFunctions[i & ~FUNC_IMPORTED]->boundFunctionId;
			if( funcId == -1 )
			{
				// Need to update the program pointer for the exception handler
				m_regs.programPointer += 2;

				// Tell the exception handler to clean up the arguments to this function
				m_needToCleanupArgs = true;
				SetInternalException(TXT_UNBOUND_FUNCTION);
				return;
			}
			else
			{
				asCScriptFunction *func = m_engine->GetScriptFunction(funcId);
				if( func->funcType == asFUNC_SCRIPT )
				{
					m_regs.programPointer += 2;
					CallScriptFunction(func);
				}
				else if( func->funcType == asFUNC_DELEGATE )
				{
					// Push the object pointer on the stack. There is always a reserved space for this so
					// we don't don't need to worry about overflowing the allocated memory buffer
					asASSERT( m_regs.stackPointer - AS_PTR_SIZE >= m_stackBlocks[m_stackIndex] );
					m_regs.stackPointer -= AS_PTR_SIZE;
					*(asPWORD*)m_regs.stackPointer = asPWORD(func->objForDelegate);

					// Call the delegated method
					if( func->funcForDelegate->funcType == asFUNC_SYSTEM )
					{
						m_regs.stackPointer += CallSystemFunction(func->funcForDelegate->id, this);

						// Update program position after the call so the line number
						// is correct in case the system function queries it
						m_regs.programPointer += 2;
					}
					else
					{
						m_regs.programPointer += 2;

						// TODO: run-time optimize: The true method could be figured out when creating the delegate
						CallInterfaceMethod(func->funcForDelegate);
					}
				}
				else
				{
					asASSERT( func->funcType == asFUNC_SYSTEM );

					m_regs.stackPointer += CallSystemFunction(func->id, this);

					// Update program position after the call so the line number
					// is correct in case the system function queries it
					m_regs.programPointer += 2;
				}
			}

			// Extract the values from the context again
			l_bc = m_regs.programPointer;
			l_sp = m_regs.stackPointer;
			l_fp = m_regs.stackFramePointer;

			// If status isn't active anymore then we must stop
			if( m_status != asEXECUTION_ACTIVE )
				return;
		}
		break;

	case asBC_SUSPEND:
		if( CanEverRunLineCallback )
		{
			if (ShouldAlwaysRunLineCallback || (m_currentFunction->module && m_currentFunction->module->hasBreakPoints))
			{
				if( m_lineCallback )
				{
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					m_lineCallback(this);
				}
			}
		}

		if (++m_loopDetectionCounter > 100000)
		{
			if (m_loopDetectionCallback)
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				m_loopDetectionCallback(this);

				// If status isn't active anymore then we must stop
				if( m_status != asEXECUTION_ACTIVE )
					return;
			}

			m_loopDetectionCounter = 0;
		}

		l_bc++;
		break;


	case asBC_ALLOC:
		{
			asCObjectType *objType = (asCObjectType*)asBC_PTRARG(l_bc);
			int func = asBC_INTARG(l_bc+AS_PTR_SIZE);

			if( objType->flags & asOBJ_SCRIPT_OBJECT )
			{
				// Need to move the values back to the context as the construction
				// of the script object may reuse the context for nested calls.
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Pre-allocate the memory
				asDWORD *mem = (asDWORD*)m_engine->AllocScriptObject(objType);

				// Pre-initialize the memory by calling the constructor for asCScriptObject
				ScriptObject_Construct(objType, (asCScriptObject*)mem);

				// Call the constructor to initalize the memory
				asCScriptFunction *f = m_engine->scriptFunctions[func];

				asDWORD **a = (asDWORD**)*(asPWORD*)(m_regs.stackPointer + f->spaceNeededForArguments);
				if( a ) *a = mem;

				// Push the object pointer on the stack
				m_regs.stackPointer -= AS_PTR_SIZE;
				*(asPWORD*)m_regs.stackPointer = (asPWORD)mem;

				m_regs.programPointer += 2+AS_PTR_SIZE;

				CallScriptFunction(f);

				// Extract the values from the context again
				l_bc = m_regs.programPointer;
				l_sp = m_regs.stackPointer;
				l_fp = m_regs.stackFramePointer;

				// If status isn't active anymore then we must stop
				if( m_status != asEXECUTION_ACTIVE )
					return;
			}
			else
			{
				// Pre-allocate the memory
				asDWORD *mem = (asDWORD*)m_engine->CallAlloc(objType);

				if( func )
				{
					// Push the object pointer on the stack (it will be popped by the function)
					l_sp -= AS_PTR_SIZE;
					*(asPWORD*)l_sp = (asPWORD)mem;

					// Need to move the values back to the context as the called functions
					// may use the debug interface to inspect the registers
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					l_sp += CallSystemFunction(func, this);
				}

				// Pop the variable address from the stack
				asDWORD **a = (asDWORD**)*(asPWORD*)l_sp;
				l_sp += AS_PTR_SIZE;
				if( a ) *a = mem;

				l_bc += 2+AS_PTR_SIZE;

				// An exception might have been raised
				if( m_status != asEXECUTION_ACTIVE )
				{
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					m_engine->CallFree(mem);
					*a = 0;

					return;
				}
			}
		}
		break;

	case asBC_FREE:
		{
			// Get the variable that holds the object handle/reference
			asPWORD *a = (asPWORD*)asPWORD(l_fp - asBC_SWORDARG0(l_bc));
			if( *a )
			{
				asCObjectType *objType = (asCObjectType*)asBC_PTRARG(l_bc);
				asSTypeBehaviour *beh = &objType->beh;

				if( objType->flags & asOBJ_REF )
				{
				}
				else if (objType->flags & asOBJ_SCRIPT_OBJECT)
				{
					// Need to move the values back to the context as the called functions
					// may use the debug interface to inspect the registers
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					asASSERT( (objType->flags & asOBJ_VALUE) );
					((asCScriptObject*)*a)->CallDestructor(objType);

					m_engine->CallFree((void*)(asPWORD)*a);
				}
				else
				{
					// Need to move the values back to the context as the called functions
					// may use the debug interface to inspect the registers
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					if( beh->destruct )
						m_engine->CallObjectMethod((void*)(asPWORD)*a, beh->destruct);
					else if( objType->flags & asOBJ_LIST_PATTERN )
						m_engine->DestroyList((asBYTE*)(asPWORD)*a, objType);

					m_engine->CallFree((void*)(asPWORD)*a);
				}

				// Clear the variable
				*a = 0;
			}
		}
		l_bc += 1+AS_PTR_SIZE;
		break;

	case asBC_LOADOBJ:
		{
			// Move the object pointer from the object variable into the object register
			void **a = (void**)(l_fp - asBC_SWORDARG0(l_bc));
			m_regs.objectType = 0;
			m_regs.objectRegister = *a;
			*a = 0;
		}
		l_bc++;
		break;

	case asBC_STOREOBJ:
		// Move the object pointer from the object register to the object variable
		*(asPWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = asPWORD(m_regs.objectRegister);
		m_regs.objectRegister = 0;
		l_bc++;
		break;

	case asBC_GETOBJ:
		{
			// Read variable index from location on stack
			asPWORD *a = (asPWORD*)(l_sp + asBC_WORDARG0(l_bc));
			// Move pointer from variable to the same location on the stack
			asPWORD *v = (asPWORD*)(l_fp - asBC_SWORDARG1(l_bc));
			*a = *v;
			// Clear variable
			*v = 0;
		}
		l_bc += 2;
		break;

	case asBC_REFCPY:
		{
			// Pop address of destination pointer from the stack
			void **d = (void**)*(asPWORD*)l_sp;
			l_sp += AS_PTR_SIZE;

			// Read wanted pointer from the stack
			void *s = (void*)*(asPWORD*)l_sp;

			// Set the new object in the destination
			*d = s;
		}
		l_bc += 1;
		break;

	case asBC_CHKREF:
		{
			// Verify if the pointer on the stack is null
			// This is used when validating a pointer that an operator will work on
			asPWORD a = *(asPWORD*)l_sp;
			if( a == 0 )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}
		}
		l_bc++;
		break;

	case asBC_GETOBJREF:
		{
			// Get the location on the stack where the reference will be placed
			asPWORD *a = (asPWORD*)(l_sp + asBC_WORDARG0(l_bc));

			// Replace the variable index with the object handle held in the variable
			*(asPWORD**)a = *(asPWORD**)(l_fp - asBC_SWORDARG1(l_bc));
		}
		l_bc += 2;
		break;

	case asBC_GETREF:
		{
			// Get the location on the stack where the reference will be placed
			asPWORD *a = (asPWORD*)(l_sp + asBC_WORDARG0(l_bc));

			// Replace the variable index with the address of the variable
			*(asPWORD**)a = (asPWORD*)(l_fp - asBC_SWORDARG1(l_bc));
		}
		l_bc += 2;
		break;

	case asBC_PshNull:
		// Push a null pointer on the stack
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = 0;
		l_bc++;
		break;

	case asBC_ClrVPtr:
		// TODO: runtime optimize: Is this instruction really necessary?
		//                         CallScriptFunction() can clear the null handles upon entry, just as is done for
		//                         all other object variables
		// Clear pointer variable
		*(asPWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = 0;
		l_bc++;
		break;

	case asBC_OBJTYPE:
		// Push the object type on the stack
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = asBC_PTRARG(l_bc);
		l_bc += 1+AS_PTR_SIZE;
		break;

	case asBC_TYPEID:
		// Equivalent to PshC4, but kept as separate instruction for bytecode serialization
		--l_sp;
		*l_sp = asBC_DWORDARG(l_bc);
		l_bc += 2;
		break;

	case asBC_SetV4:
		*(l_fp - asBC_SWORDARG0(l_bc)) = asBC_DWORDARG(l_bc);
		l_bc += 2;
		break;

	case asBC_SetV8:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = asBC_QWORDARG(l_bc);
		l_bc += 3;
		break;

	case asBC_ADDSi:
		{
			// The pointer must not be null
			asPWORD a = *(asPWORD*)l_sp;
			if( a == 0 )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}
			// Add an offset to the pointer
			*(asPWORD*)l_sp = a + asBC_SWORDARG0(l_bc);
		}
		l_bc += 2;
		break;

	case asBC_CpyVtoV4:
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(l_fp - asBC_SWORDARG1(l_bc));
		l_bc += 2;
		break;

	case asBC_CpyVtoV8:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc));
		l_bc += 2;
		break;

	case asBC_CpyVtoR4:
		*(asDWORD*)&m_regs.valueRegister = *(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_CpyVtoR8:
		*(asQWORD*)&m_regs.valueRegister = *(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_CpyVtoG4:
		*(asDWORD*)asBC_PTRARG(l_bc) = *(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc += 1 + AS_PTR_SIZE;
		break;

	case asBC_CpyRtoV4:
		*(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asDWORD*)&m_regs.valueRegister;
		l_bc++;
		break;

	case asBC_CpyRtoV8:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = m_regs.valueRegister;
		l_bc++;
		break;

	case asBC_CpyGtoV4:
		*(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asDWORD*)asBC_PTRARG(l_bc);
		l_bc += 1 + AS_PTR_SIZE;
		break;

	case asBC_WRTV1:
		// The pointer in the register points to a byte, and *(l_fp - offset) too
		**(asBYTE**)&m_regs.valueRegister = *(asBYTE*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_WRTV2:
		// The pointer in the register points to a word, and *(l_fp - offset) too
		**(asWORD**)&m_regs.valueRegister = *(asWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_WRTV4:
		**(asDWORD**)&m_regs.valueRegister = *(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_WRTV8:
		**(asQWORD**)&m_regs.valueRegister = *(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_RDR1:
		{
			// The pointer in the register points to a byte, and *(l_fp - offset) will also point to a byte
			asBYTE *bPtr = (asBYTE*)(l_fp - asBC_SWORDARG0(l_bc));
			bPtr[0] = **(asBYTE**)&m_regs.valueRegister; // read the byte
			bPtr[1] = 0;                      // 0 the rest of the DWORD
			bPtr[2] = 0;
			bPtr[3] = 0;
		}
		l_bc++;
		break;

	case asBC_RDR2:
		{
			// The pointer in the register points to a word, and *(l_fp - offset) will also point to a word
			asWORD *wPtr = (asWORD*)(l_fp - asBC_SWORDARG0(l_bc));
			wPtr[0] = **(asWORD**)&m_regs.valueRegister; // read the word
			wPtr[1] = 0;                      // 0 the rest of the DWORD
		}
		l_bc++;
		break;

	case asBC_RDR4:
		*(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = **(asDWORD**)&m_regs.valueRegister;
		l_bc++;
		break;

	case asBC_RDR8:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = **(asQWORD**)&m_regs.valueRegister;
		l_bc++;
		break;

	case asBC_LDG:
		*(asPWORD*)&m_regs.valueRegister = asBC_PTRARG(l_bc);
		l_bc += 1+AS_PTR_SIZE;
		break;

	case asBC_LDV:
		*(asDWORD**)&m_regs.valueRegister = (l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_PGA:
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = asBC_PTRARG(l_bc);
		l_bc += 1+AS_PTR_SIZE;
		break;

	case asBC_CmpPtr:
		{
			// TODO: runtime optimize: This instruction should really just be an equals, and return true or false.
			//                         The instruction is only used for is and !is tests anyway.
			asPWORD p1 = *(asPWORD*)(l_fp - asBC_SWORDARG0(l_bc));
			asPWORD p2 = *(asPWORD*)(l_fp - asBC_SWORDARG1(l_bc));
			if( p1 == p2 ) *(int*)&m_regs.valueRegister =  0;
			else *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_VAR:
		l_sp -= AS_PTR_SIZE;
		//*(asPWORD*)l_sp = (asPWORD)asBC_SWORDARG0(l_bc);
		l_bc++;
		break;

	//----------------------------
	// Type conversions
	case asBC_iTOf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = float(*(int*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_fTOi:
		*(l_fp - asBC_SWORDARG0(l_bc)) = int(*(float*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_uTOf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = float(*(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_fTOu:
		// We must cast to int first, because on some compilers the cast of a negative float value to uint result in 0
		*(l_fp - asBC_SWORDARG0(l_bc)) = asUINT(int(*(float*)(l_fp - asBC_SWORDARG1(l_bc))));
		l_bc += 2;
		break;

	case asBC_sbTOi:
		// *(l_fp - offset) points to a char, and will point to an int afterwards
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(signed char*)(l_fp - asBC_SWORDARG1(l_bc));
		l_bc += 2;
		break;

	case asBC_swTOi:
		// *(l_fp - offset) points to a short, and will point to an int afterwards
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(short*)(l_fp - asBC_SWORDARG1(l_bc));
		l_bc += 2;
		break;

	case asBC_ubTOi:
		// (l_fp - offset) points to a byte, and will point to an int afterwards
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(asBYTE*)(l_fp - asBC_SWORDARG1(l_bc));
		l_bc += 2;
		break;

	case asBC_uwTOi:
		// *(l_fp - offset) points to a word, and will point to an int afterwards
		*(l_fp - asBC_SWORDARG0(l_bc)) = *(asWORD*)(l_fp - asBC_SWORDARG1(l_bc));
		l_bc += 2;
		break;

	case asBC_dTOi:
		*(l_fp - asBC_SWORDARG0(l_bc)) = int(*(double*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_dTOu:
		// We must cast to int first, because on some compilers the cast of a negative float value to uint result in 0
		*(l_fp - asBC_SWORDARG0(l_bc)) = asUINT(int(*(double*)(l_fp - asBC_SWORDARG1(l_bc))));
		l_bc += 2;
		break;

	case asBC_dTOf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = float(*(double*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_iTOd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = double(*(int*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_uTOd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = double(*(asUINT*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_fTOd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = double(*(float*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	//------------------------------
	// Math operations
	case asBC_ADDi:
		*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) + *(int*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_SUBi:
		*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) - *(int*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_MULi:
		*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) * *(int*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_DIVi:
		{
			int divider = *(int*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			else if( divider == -1 )
			{
				// Need to check if the value that is divided is 0x80000000
				// as dividing it with -1 will cause an overflow exception
				if( *(int*)(l_fp - asBC_SWORDARG1(l_bc)) == int(0x80000000) )
				{
					// Need to move the values back to the context
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					// Raise exception
					SetInternalException(TXT_DIVIDE_OVERFLOW);
					return;
				}
			}
			*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) / divider;
		}
		l_bc += 2;
		break;

	case asBC_MODi:
		{
			int divider = *(int*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			else if( divider == -1 )
			{
				// Need to check if the value that is divided is 0x80000000
				// as dividing it with -1 will cause an overflow exception
				if( *(int*)(l_fp - asBC_SWORDARG1(l_bc)) == int(0x80000000) )
				{
					// Need to move the values back to the context
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					// Raise exception
					SetInternalException(TXT_DIVIDE_OVERFLOW);
					return;
				}
			}
			*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) % divider;
		}
		l_bc += 2;
		break;

	case asBC_ADDf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = *(float*)(l_fp - asBC_SWORDARG1(l_bc)) + *(float*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_SUBf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = *(float*)(l_fp - asBC_SWORDARG1(l_bc)) - *(float*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_MULf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = *(float*)(l_fp - asBC_SWORDARG1(l_bc)) * *(float*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_DIVf:
		{
			float divider = *(float*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = *(float*)(l_fp - asBC_SWORDARG1(l_bc)) / divider;
		}
		l_bc += 2;
		break;

	case asBC_MODf:
		{
			float divider = *(float*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = fmodf(*(float*)(l_fp - asBC_SWORDARG1(l_bc)), divider);
		}
		l_bc += 2;
		break;

	case asBC_ADDd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = *(double*)(l_fp - asBC_SWORDARG1(l_bc)) + *(double*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_SUBd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = *(double*)(l_fp - asBC_SWORDARG1(l_bc)) - *(double*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_MULd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = *(double*)(l_fp - asBC_SWORDARG1(l_bc)) * *(double*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_DIVd:
		{
			double divider = *(double*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}

			*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = *(double*)(l_fp - asBC_SWORDARG1(l_bc)) / divider;
			l_bc += 2;
		}
		break;

	case asBC_MODd:
		{
			double divider = *(double*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}

			*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = fmod(*(double*)(l_fp - asBC_SWORDARG1(l_bc)), divider);
			l_bc += 2;
		}
		break;

	//------------------------------
	// Math operations with constant value
	case asBC_ADDIi:
		*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) + asBC_INTARG(l_bc+1);
		l_bc += 3;
		break;

	case asBC_SUBIi:
		*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) - asBC_INTARG(l_bc+1);
		l_bc += 3;
		break;

	case asBC_MULIi:
		*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = *(int*)(l_fp - asBC_SWORDARG1(l_bc)) * asBC_INTARG(l_bc+1);
		l_bc += 3;
		break;

	case asBC_ADDIf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = *(float*)(l_fp - asBC_SWORDARG1(l_bc)) + asBC_FLOATARG(l_bc+1);
		l_bc += 3;
		break;

	case asBC_SUBIf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = *(float*)(l_fp - asBC_SWORDARG1(l_bc)) - asBC_FLOATARG(l_bc+1);
		l_bc += 3;
		break;

	case asBC_MULIf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = *(float*)(l_fp - asBC_SWORDARG1(l_bc)) * asBC_FLOATARG(l_bc+1);
		l_bc += 3;
		break;

	//-----------------------------------
	case asBC_SetG4:
		*(asDWORD*)asBC_PTRARG(l_bc) = asBC_DWORDARG(l_bc+AS_PTR_SIZE);
		l_bc += 2 + AS_PTR_SIZE;
		break;

	case asBC_ChkRefS:
		{
			// Verify if the pointer on the stack refers to a non-null value
			// This is used to validate a reference to a handle
			asPWORD *a = (asPWORD*)*(asPWORD*)l_sp;
			if( *a == 0 )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}
		}
		l_bc++;
		break;

	case asBC_ChkNullV:
		{
			// Verify if variable (on the stack) is not null
			asDWORD *a = *(asDWORD**)(l_fp - asBC_SWORDARG0(l_bc));
			if( a == 0 )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}
		}
		l_bc++;
		break;

	case asBC_CALLINTF:
		{
			int i = asBC_INTARG(l_bc);
			l_bc += 2;

			asASSERT( i >= 0 );
			asASSERT( (i & FUNC_IMPORTED) == 0 );

			// Need to move the values back to the context
			m_regs.programPointer    = l_bc;
			m_regs.stackPointer      = l_sp;
			m_regs.stackFramePointer = l_fp;

			CallInterfaceMethod(m_engine->GetScriptFunction(i));

			// Extract the values from the context again
			l_bc = m_regs.programPointer;
			l_sp = m_regs.stackPointer;
			l_fp = m_regs.stackFramePointer;

			// If status isn't active anymore then we must stop
			if( m_status != asEXECUTION_ACTIVE )
				return;
		}
		break;

	case asBC_iTOb:
		{
			// *(l_fp - offset) points to an int, and will point to a byte afterwards

			// We need to use volatile here to tell the compiler not to rearrange
			// read and write operations during optimizations.
			volatile asDWORD val  = *(l_fp - asBC_SWORDARG1(l_bc));
			volatile asBYTE *bPtr = (asBYTE*)(l_fp - asBC_SWORDARG0(l_bc));
			bPtr[0] = (asBYTE)val; // write the byte
			bPtr[1] = 0;           // 0 the rest of the DWORD
			bPtr[2] = 0;
			bPtr[3] = 0;
		}
		l_bc += 2;
		break;

	case asBC_iTOw:
		{
			// *(l_fp - offset) points to an int, and will point to word afterwards

			// We need to use volatile here to tell the compiler not to rearrange
			// read and write operations during optimizations.
			volatile asDWORD val  = *(l_fp - asBC_SWORDARG1(l_bc));
			volatile asWORD *wPtr = (asWORD*)(l_fp - asBC_SWORDARG0(l_bc));
			wPtr[0] = (asWORD)val; // write the word
			wPtr[1] = 0;           // 0 the rest of the DWORD
		}
		l_bc += 2;
		break;

	case asBC_SetV1:
		// TODO: This is exactly the same as SetV4. This is a left over from the time
		//       when the bytecode instructions were more tightly packed. It can now
		//       be removed. When removing it, make sure the value is correctly converted
		//       on big-endian CPUs.

		// The byte is already stored correctly in the argument
		*(l_fp - asBC_SWORDARG0(l_bc)) = asBC_DWORDARG(l_bc);
		l_bc += 2;
		break;

	case asBC_SetV2:
		// TODO: This is exactly the same as SetV4. This is a left over from the time
		//       when the bytecode instructions were more tightly packed. It can now
		//       be removed. When removing it, make sure the value is correctly converted
		//       on big-endian CPUs.

		// The word is already stored correctly in the argument
		*(l_fp - asBC_SWORDARG0(l_bc)) = asBC_DWORDARG(l_bc);
		l_bc += 2;
		break;

	case asBC_Cast:
		// Cast the handle at the top of the stack to the type in the argument
		{
			asDWORD **a = (asDWORD**)*(asPWORD*)l_sp;
			if( a && *a )
			{
				asDWORD typeId = asBC_DWORDARG(l_bc);

				asCScriptObject *obj = (asCScriptObject *)* a;
				asCObjectType *objType = (asCObjectType*)obj->GetObjectType();
				asCObjectType *to = m_engine->GetObjectTypeFromTypeId(typeId);

				// This instruction can only be used with script classes and interfaces
				asASSERT( objType->flags & asOBJ_SCRIPT_OBJECT );
				asASSERT( to->flags & asOBJ_SCRIPT_OBJECT );

				if( objType->Implements(to) || objType->DerivesFrom(to) )
				{
					m_regs.objectType = 0;
					m_regs.objectRegister = obj;
					//obj->AddRef();
				}
				else
				{
					// The object register should already be null, so there
					// is no need to clear it if the cast is unsuccessful
					asASSERT( m_regs.objectRegister == 0 );
				}
			}
			l_sp += AS_PTR_SIZE;
		}
		l_bc += 2;
		break;

	case asBC_i64TOi:
		*(l_fp - asBC_SWORDARG0(l_bc)) = int(*(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_uTOi64:
		*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = asINT64(*(asUINT*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_iTOi64:
		*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = asINT64(*(int*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_fTOi64:
		*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = asINT64(*(float*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_dTOi64:
		*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = asINT64(*(double*)(l_fp - asBC_SWORDARG0(l_bc)));
		l_bc++;
		break;

	case asBC_fTOu64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = asQWORD(asINT64(*(float*)(l_fp - asBC_SWORDARG1(l_bc))));
		l_bc += 2;
		break;

	case asBC_dTOu64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = asQWORD(asINT64(*(double*)(l_fp - asBC_SWORDARG0(l_bc))));
		l_bc++;
		break;

	case asBC_i64TOf:
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = float(*(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_u64TOf:
#if defined(_MSC_VER) && _MSC_VER <= 1200 // MSVC6
		{
			// MSVC6 doesn't permit UINT64 to double
			asINT64 v = *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc));
			if( v < 0 )
				*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = 18446744073709551615.0f+float(v);
			else
				*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = float(v);
		}
#else
		*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = float(*(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)));
#endif
		l_bc += 2;
		break;

	case asBC_i64TOd:
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = double(*(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)));
		l_bc += 2;
		break;

	case asBC_u64TOd:
#if defined(_MSC_VER) && _MSC_VER <= 1200 // MSVC6
		{
			// MSVC6 doesn't permit UINT64 to double
			asINT64 v = *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc));
			if( v < 0 )
				*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = 18446744073709551615.0+double(v);
			else
				*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = double(v);
		}
#else
		*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = double(*(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)));
#endif
		l_bc += 2;
		break;

	case asBC_NEGi64:
		*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = -*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_INCi64:
		++(**(asQWORD**)&m_regs.valueRegister);
		l_bc++;
		break;

	case asBC_DECi64:
		--(**(asQWORD**)&m_regs.valueRegister);
		l_bc++;
		break;

	case asBC_BNOT64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = ~*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_ADDi64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) + *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_SUBi64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) - *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_MULi64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) * *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_DIVi64:
		{
			asINT64 divider = *(asINT64*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			else if( divider == -1 )
			{
				// Need to check if the value that is divided is 1<<63
				// as dividing it with -1 will cause an overflow exception
				if( *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)) == (asINT64(1)<<63) )
				{
					// Need to move the values back to the context
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					// Raise exception
					SetInternalException(TXT_DIVIDE_OVERFLOW);
					return;
				}
			}

			*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)) / divider;
		}
		l_bc += 2;
		break;

	case asBC_MODi64:
		{
			asINT64 divider = *(asINT64*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			else if( divider == -1 )
			{
				// Need to check if the value that is divided is 1<<63
				// as dividing it with -1 will cause an overflow exception
				if( *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)) == (asINT64(1)<<63) )
				{
					// Need to move the values back to the context
					m_regs.programPointer    = l_bc;
					m_regs.stackPointer      = l_sp;
					m_regs.stackFramePointer = l_fp;

					// Raise exception
					SetInternalException(TXT_DIVIDE_OVERFLOW);
					return;
				}
			}
			*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)) % divider;
		}
		l_bc += 2;
		break;

	case asBC_BAND64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) & *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_BOR64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) | *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_BXOR64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) ^ *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_BSLL64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) << *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_BSRL64:
		*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) >> *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_BSRA64:
		*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)) >> *(l_fp - asBC_SWORDARG2(l_bc));
		l_bc += 2;
		break;

	case asBC_CMPi64:
		{
			asINT64 i1 = *(asINT64*)(l_fp - asBC_SWORDARG0(l_bc));
			asINT64 i2 = *(asINT64*)(l_fp - asBC_SWORDARG1(l_bc));
			if( i1 == i2 )     *(int*)&m_regs.valueRegister =  0;
			else if( i1 < i2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_CMPu64:
		{
			asQWORD d1 = *(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc));
			asQWORD d2 = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc));
			if( d1 == d2 )     *(int*)&m_regs.valueRegister =  0;
			else if( d1 < d2 ) *(int*)&m_regs.valueRegister = -1;
			else               *(int*)&m_regs.valueRegister =  1;
			l_bc += 2;
		}
		break;

	case asBC_ChkNullS:
		{
			// Verify if the pointer on the stack is null
			// This is used for example when validating handles passed as function arguments
			asPWORD a = *(asPWORD*)(l_sp + asBC_WORDARG0(l_bc));
			if( a == 0 )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}
		}
		l_bc++;
		break;

	case asBC_ClrHi:
#if AS_SIZEOF_BOOL == 1
		{
			// Clear the upper bytes, so that trash data don't interfere with boolean operations

			// We need to use volatile here to tell the compiler it cannot
			// change the order of read and write operations on the pointer.

			volatile asBYTE *ptr = (asBYTE*)&m_regs.valueRegister;
			ptr[1] = 0;   // The boolean value is stored in the lower byte, so we clear the rest
			ptr[2] = 0;
			ptr[3] = 0;
		}
#else
		// We don't have anything to do here
#endif
		l_bc++;
		break;

	case asBC_JitEntry:
		{
			// Not a JIT resume point, treat as nop
			l_bc += 1+AS_PTR_SIZE;
		}
		break;

	case asBC_CallPtr:
		{
			// Get the function pointer from the local variable
			asCScriptFunction *func = *(asCScriptFunction**)(l_fp - asBC_SWORDARG0(l_bc));

			// Need to move the values back to the context
			m_regs.programPointer    = l_bc;
			m_regs.stackPointer      = l_sp;
			m_regs.stackFramePointer = l_fp;

			if( func == 0 )
			{
				// Need to update the program pointer anyway for the exception handler
				m_regs.programPointer++;

				// Tell the exception handler to clean up the arguments to this method
				m_needToCleanupArgs = true;

				// TODO: funcdef: Should we have a different exception string?
				SetInternalException(TXT_UNBOUND_FUNCTION);
				return;
			}
			else
			{
				if( func->funcType == asFUNC_SCRIPT )
				{
					m_regs.programPointer++;
					CallScriptFunction(func);
				}
				else if( func->funcType == asFUNC_DELEGATE )
				{
					// Push the object pointer on the stack. There is always a reserved space for this so
					// we don't don't need to worry about overflowing the allocated memory buffer
					asASSERT( m_regs.stackPointer - AS_PTR_SIZE >= m_stackBlocks[m_stackIndex] );
					m_regs.stackPointer -= AS_PTR_SIZE;
					*(asPWORD*)m_regs.stackPointer = asPWORD(func->objForDelegate);

					// Call the delegated method
					if( func->funcForDelegate->funcType == asFUNC_SYSTEM )
					{
						m_regs.stackPointer += CallSystemFunction(func->funcForDelegate->id, this);

						// Update program position after the call so the line number
						// is correct in case the system function queries it
						m_regs.programPointer++;
					}
					else
					{
						m_regs.programPointer++;

						// TODO: run-time optimize: The true method could be figured out when creating the delegate
						CallInterfaceMethod(func->funcForDelegate);
					}
				}
				else
				{
					asASSERT( func->funcType == asFUNC_SYSTEM );

					m_regs.stackPointer += CallSystemFunction(func->id, this);

					// Update program position after the call so the line number
					// is correct in case the system function queries it
					m_regs.programPointer++;
				}
			}

			// Extract the values from the context again
			l_bc = m_regs.programPointer;
			l_sp = m_regs.stackPointer;
			l_fp = m_regs.stackFramePointer;

			// If status isn't active anymore then we must stop
			if( m_status != asEXECUTION_ACTIVE )
				return;
		}
		break;

	case asBC_FuncPtr:
		// Push the function pointer on the stack. The pointer is in the argument
		l_sp -= AS_PTR_SIZE;
		*(asPWORD*)l_sp = asBC_PTRARG(l_bc);
		l_bc += 1+AS_PTR_SIZE;
		break;

	case asBC_LoadThisR:
		{
			// PshVPtr 0
			asPWORD tmp = *(asPWORD*)l_fp;

			// Make sure the pointer is not null
			if( tmp == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}

			// ADDSi
			tmp = tmp + asBC_SWORDARG0(l_bc);

			// PopRPtr
			*(asPWORD*)&m_regs.valueRegister = tmp;
			l_bc += 2;
		}
		break;

	// Push the qword value of a variable on the stack
	case asBC_PshV8:
		l_sp -= 2;
		*(asQWORD*)l_sp = *(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_DIVu:
		{
			asUINT divider = *(asUINT*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			*(asUINT*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asUINT*)(l_fp - asBC_SWORDARG1(l_bc)) / divider;
		}
		l_bc += 2;
		break;

	case asBC_MODu:
		{
			asUINT divider = *(asUINT*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			*(asUINT*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asUINT*)(l_fp - asBC_SWORDARG1(l_bc)) % divider;
		}
		l_bc += 2;
		break;

	case asBC_DIVu64:
		{
			asQWORD divider = *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) / divider;
		}
		l_bc += 2;
		break;

	case asBC_MODu64:
		{
			asQWORD divider = *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc));
			if( divider == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_DIVIDE_BY_ZERO);
				return;
			}
			*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = *(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)) % divider;
		}
		l_bc += 2;
		break;

	case asBC_LoadRObjR:
		{
			// PshVPtr x
			asPWORD tmp = *(asPWORD*)(l_fp - asBC_SWORDARG0(l_bc));

			// Make sure the pointer is not null
			if( tmp == 0 )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_NULL_POINTER_ACCESS);
				return;
			}

			// ADDSi y
			tmp = tmp + asBC_SWORDARG1(l_bc);

			// PopRPtr
			*(asPWORD*)&m_regs.valueRegister = tmp;
			l_bc += 3;
		}
		break;

	case asBC_LoadVObjR:
		{
			// PSF x
			asPWORD tmp = (asPWORD)(l_fp - asBC_SWORDARG0(l_bc));

			// ADDSi y
			tmp = tmp + asBC_SWORDARG1(l_bc);

			// PopRPtr
			*(asPWORD*)&m_regs.valueRegister = tmp;
			l_bc += 3;
		}
		break;

	case asBC_RefCpyV:
		// Same as PSF v, REFCPY, PopPtr
		{
			// Determine destination from argument
			void **d = (void**)asPWORD(l_fp - asBC_SWORDARG0(l_bc));

			// Read wanted pointer from the stack
			void *s = (void*)*(asPWORD*)l_sp;

			// Set the new object in the destination
			*d = s;

			// Pop the pointer from the stack
			l_sp += AS_PTR_SIZE;
		}
		l_bc += 1;
		break;

	case asBC_JLowZ:
		if( *(asBYTE*)&m_regs.valueRegister == 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;

	case asBC_JLowNZ:
		if( *(asBYTE*)&m_regs.valueRegister != 0 )
			l_bc += asBC_INTARG(l_bc) + 2;
		else
			l_bc += 2;
		break;

	case asBC_AllocMem:
		// Allocate a buffer and store the pointer in the local variable
		{
			// TODO: runtime optimize: As the list buffers are going to be short lived, it may be interesting
			//                         to use a memory pool to avoid reallocating the memory all the time

			asUINT size = asBC_DWORDARG(l_bc);
			asBYTE **var = (asBYTE**)(l_fp - asBC_SWORDARG0(l_bc));
#ifndef WIP_16BYTE_ALIGN
			*var = asNEWARRAY(asBYTE, size);
#else
			*var = asNEWARRAYALIGNED(asBYTE, size, MAX_TYPE_ALIGNMENT);
#endif

			// Clear the buffer for the pointers that will be placed in it
			memset(*var, 0, size);
		}
		l_bc += 2;
		break;

	case asBC_SetListSize:
		{
			// Set the size element in the buffer
			asBYTE *var = *(asBYTE**)(l_fp - asBC_SWORDARG0(l_bc));
			asUINT off  = asBC_DWORDARG(l_bc);
			asUINT size = asBC_DWORDARG(l_bc+1);

			asASSERT( var );

			*(asUINT*)(var+off) = size;
		}
		l_bc += 3;
		break;

	case asBC_PshListElmnt:
		{
			// Push the pointer to the list element on the stack
			// In essence it does the same as PSF, RDSPtr, ADDSi
			asBYTE *var = *(asBYTE**)(l_fp - asBC_SWORDARG0(l_bc));
			asUINT off = asBC_DWORDARG(l_bc);

			asASSERT( var );

			l_sp -= AS_PTR_SIZE;
			*(asPWORD*)l_sp = asPWORD(var+off);
		}
		l_bc += 2;
		break;

	case asBC_SetListType:
		{
			// Set the type id in the buffer
			asBYTE *var = *(asBYTE**)(l_fp - asBC_SWORDARG0(l_bc));
			asUINT off  = asBC_DWORDARG(l_bc);
			asUINT type = asBC_DWORDARG(l_bc+1);

			asASSERT( var );

			*(asUINT*)(var+off) = type;
		}
		l_bc += 3;
		break;

	//------------------------------
	// Exponent operations
	case asBC_POWi:
		{
			bool isOverflow;
			*(int*)(l_fp - asBC_SWORDARG0(l_bc)) = as_powi(*(int*)(l_fp - asBC_SWORDARG1(l_bc)), *(int*)(l_fp - asBC_SWORDARG2(l_bc)), isOverflow);
			if( isOverflow )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_POW_OVERFLOW);
				return;
			}
		}
		l_bc += 2;
		break;

	case asBC_POWu:
		{
			bool isOverflow;
			*(asDWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = as_powu(*(asDWORD*)(l_fp - asBC_SWORDARG1(l_bc)), *(asDWORD*)(l_fp - asBC_SWORDARG2(l_bc)), isOverflow);
			if( isOverflow )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_POW_OVERFLOW);
				return;
			}
		}
		l_bc += 2;
		break;

	case asBC_POWf:
		{
			float r = FMath::Pow(*(float*)(l_fp - asBC_SWORDARG1(l_bc)), *(float*)(l_fp - asBC_SWORDARG2(l_bc)));
			*(float*)(l_fp - asBC_SWORDARG0(l_bc)) = r;
			if( r == float(HUGE_VAL) )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_POW_OVERFLOW);
				return;
			}
		}
		l_bc += 2;
		break;

	case asBC_POWd:
		{
			double r = FMath::Pow(*(double*)(l_fp - asBC_SWORDARG1(l_bc)), *(double*)(l_fp - asBC_SWORDARG2(l_bc)));
			*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = r;
			if( r == HUGE_VAL )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_POW_OVERFLOW);
				return;
			}
		}
		l_bc += 2;
		break;

	case asBC_POWdi:
		{
			double r = FMath::Pow(*(double*)(l_fp - asBC_SWORDARG1(l_bc)), (double)*(int*)(l_fp - asBC_SWORDARG2(l_bc)));
			*(double*)(l_fp - asBC_SWORDARG0(l_bc)) = r;
			if( r == HUGE_VAL )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_POW_OVERFLOW);
				return;
			}
			l_bc += 2;
		}
		break;

	case asBC_POWi64:
		{
			bool isOverflow;
			*(asINT64*)(l_fp - asBC_SWORDARG0(l_bc)) = as_powi64(*(asINT64*)(l_fp - asBC_SWORDARG1(l_bc)), *(asINT64*)(l_fp - asBC_SWORDARG2(l_bc)), isOverflow);
			if( isOverflow )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_POW_OVERFLOW);
				return;
			}
		}
		l_bc += 2;
		break;

	case asBC_POWu64:
		{
			bool isOverflow;
			*(asQWORD*)(l_fp - asBC_SWORDARG0(l_bc)) = as_powu64(*(asQWORD*)(l_fp - asBC_SWORDARG1(l_bc)), *(asQWORD*)(l_fp - asBC_SWORDARG2(l_bc)), isOverflow);
			if( isOverflow )
			{
				// Need to move the values back to the context
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				// Raise exception
				SetInternalException(TXT_POW_OVERFLOW);
				return;
			}
		}
		l_bc += 2;
		break;
	case asBC_Thiscall1:
		// This instruction is a faster version of asBC_CALLSYS. It is faster because
		// it has much less runtime overhead with determining the calling convention 
		// and no dynamic code for loading the parameters. The instruction can only
		// be used to call functions with the following signatures:
		//
		//  type &obj::func(int)
		//  type &obj::func(uint)
		//  void  obj::func(int)
		//  void  obj::func(uint)
		{
			// Get function ID from the argument
			asCScriptFunction* Function = (asCScriptFunction*)asBC_PTRARG(l_bc);

			// Need to move the values back to the context as the called functions
			// may use the debug interface to inspect the registers
			m_regs.programPointer    = l_bc;
			m_regs.stackPointer      = l_sp;
			m_regs.stackFramePointer = l_fp;

			// Pop the thispointer from the stack
			void *obj = *(void**)l_sp;
			if (obj == 0)
				SetInternalException(TXT_NULL_POINTER_ACCESS);
			else
			{
				// Only update the stack pointer if all is OK so the 
				// exception handler can properly clean up the stack
				l_sp += AS_PTR_SIZE;

				// Pop the int arg from the stack
				int arg = *(int*)l_sp;
				l_sp++;

				auto* tld = m_regs.tld;
				auto* prevActiveFunction = tld->activeFunction;
				tld->activeFunction = Function;

				void *ptr = m_engine->CallObjectMethodRetPtr(obj, arg, Function);

				tld->activeFunction = prevActiveFunction;

				*(asPWORD*)&m_regs.valueRegister = (asPWORD)ptr;
			}

			// Update the program position after the call so that line number is correct
			l_bc += 1 + AS_PTR_SIZE;

			// An exception might have been raised
			if( m_status != asEXECUTION_ACTIVE )
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				return;
			}
		}
		break;

	case asBC_FinConstruct:
		{
			asPWORD Object = *(asPWORD*)l_sp;
			l_sp += AS_PTR_SIZE;

			m_regs.programPointer    = l_bc;
			m_regs.stackPointer      = l_sp;
			m_regs.stackFramePointer = l_fp;

			m_engine->FinishConstructObject((asIScriptObject*)Object, (asITypeInfo*)asBC_PTRARG(l_bc));

			l_bc = m_regs.programPointer;
			l_sp = m_regs.stackPointer;
			l_fp = m_regs.stackFramePointer;

			// If status isn't active anymore then we must stop
			if( m_status != asEXECUTION_ACTIVE )
				return;

			l_bc += 1+AS_PTR_SIZE;
			break;
		}

	case asBC_DestructScript:
		/*{
			asCObjectType *objType = (asCObjectType*)asBC_PTRARG(l_bc);
			asSTypeBehaviour *beh = &objType->beh;

			// Get the variable that holds the object handle/reference
			asCScriptObject* obj = (asCScriptObject*)asPWORD(l_fp - asBC_SWORDARG0(l_bc));
			obj->CallDestructor(objType);
		}
		l_bc += 1+AS_PTR_SIZE;*/
		break;

	case asBC_CopyScript:
	{
		asCObjectType *objType = (asCObjectType*)asBC_PTRARG(l_bc);
		asCScriptObject* d = (asCScriptObject*)*(asPWORD*)l_sp; l_sp += AS_PTR_SIZE;
		asCScriptObject* s = (asCScriptObject*)*(asPWORD*)l_sp;
		if( s == 0 || d == 0 )
		{
			// Need to move the values back to the context
			m_regs.programPointer    = l_bc;
			m_regs.stackPointer      = l_sp;
			m_regs.stackFramePointer = l_fp;

			// Raise exception
			SetInternalException(TXT_NULL_POINTER_ACCESS);
			return;
		}

		d->CopyStruct(s, objType);

		// replace the pointer on the stack with the lvalue
		*(asPWORD**)l_sp = (asPWORD*)d;

		l_bc += 1+AS_PTR_SIZE;
	}
	break;

	case asBC_ResolveObjectPtr:
	{
		if (userResolveObjectPtr != nullptr)
		{
			asPWORD pointerToObjectPtr = *(asPWORD*)l_sp;
			userResolveObjectPtr(
				(void**)pointerToObjectPtr
			);
		}

		l_bc++;
	}
	break;

	case asBC_FreeNullV8:
		{
			// Get the variable that holds the object handle/reference
			asPWORD *a = (asPWORD*)asPWORD(l_fp - asBC_SWORDARG0(l_bc));
			*a = 0;
		}
		l_bc += 1;
		break;

	case asBC_TrackRef:
#if AS_REFERENCE_DEBUGGING
		{
			void** RefPtr = (void**)asPWORD(l_fp - asBC_SWORDARG0(l_bc));
			TrackedReferences.Add(RefPtr);
		}
#endif
		l_bc += 1;
		break;

	case asBC_UntrackRef:
#if AS_REFERENCE_DEBUGGING
		{
			void** RefPtr = (void**)asPWORD(l_fp - asBC_SWORDARG0(l_bc));
			TrackedReferences.RemoveSingleSwap(RefPtr);
		}
#endif
		l_bc += 1;
		break;

	case asBC_ValidateRef:
#if AS_REFERENCE_DEBUGGING
		{
			void** RefPtr = (void**)asPWORD(l_fp - asBC_SWORDARG0(l_bc));
			if (*RefPtr == nullptr)
			{
				m_regs.programPointer    = l_bc;
				m_regs.stackPointer      = l_sp;
				m_regs.stackFramePointer = l_fp;

				SetInternalException("Reference has been invalidated because the referenced container was modified");
				return;
			}
		}
#endif
		l_bc += 1;
		break;

	case asBC_CpyVtoR1:
		*(asBYTE*)&m_regs.valueRegister = *(asBYTE*)(l_fp - asBC_SWORDARG0(l_bc));
		l_bc++;
		break;

	case asBC_SaveReturnValue:
		l_bc++;
		break;

	case asBC_CmpPtrNull:
		{
			asPWORD p1 = *(asPWORD*)(l_fp - asBC_SWORDARG0(l_bc));
			if( p1 == 0 ) *(int*)&m_regs.valueRegister =  0;
			else *(int*)&m_regs.valueRegister = -1;
			l_bc++;
		}
		break;

	case asBC_ThrowException:
		{
			int Type = asBC_WORDARG0(l_bc);
			if (Type == 0)
				SetInternalException("Invalid enum value passed to switch");
			else
				SetInternalException("Unknown exception");
			l_bc++;
		}
		break;

	// Don't let the optimizer optimize for size,
	// since it requires extra conditions and jumps
	case 213: l_bc = (asDWORD*)213; break;
	case 214: l_bc = (asDWORD*)214; break;
	case 215: l_bc = (asDWORD*)215; break;
	case 216: l_bc = (asDWORD*)216; break;
	case 217: l_bc = (asDWORD*)217; break;
	case 218: l_bc = (asDWORD*)218; break;
	case 219: l_bc = (asDWORD*)219; break;
	case 220: l_bc = (asDWORD*)220; break;
	case 221: l_bc = (asDWORD*)221; break;
	case 222: l_bc = (asDWORD*)222; break;
	case 223: l_bc = (asDWORD*)223; break;
	case 224: l_bc = (asDWORD*)224; break;
	case 225: l_bc = (asDWORD*)225; break;
	case 226: l_bc = (asDWORD*)226; break;
	case 227: l_bc = (asDWORD*)227; break;
	case 228: l_bc = (asDWORD*)228; break;
	case 229: l_bc = (asDWORD*)229; break;
	case 230: l_bc = (asDWORD*)230; break;
	case 231: l_bc = (asDWORD*)231; break;
	case 232: l_bc = (asDWORD*)232; break;
	case 233: l_bc = (asDWORD*)233; break;
	case 234: l_bc = (asDWORD*)234; break;
	case 235: l_bc = (asDWORD*)235; break;
	case 236: l_bc = (asDWORD*)236; break;
	case 237: l_bc = (asDWORD*)237; break;
	case 238: l_bc = (asDWORD*)238; break;
	case 239: l_bc = (asDWORD*)239; break;
	case 240: l_bc = (asDWORD*)240; break;
	case 241: l_bc = (asDWORD*)241; break;
	case 242: l_bc = (asDWORD*)242; break;
	case 243: l_bc = (asDWORD*)243; break;
	case 244: l_bc = (asDWORD*)244; break;
	case 245: l_bc = (asDWORD*)245; break;
	case 246: l_bc = (asDWORD*)246; break;
	case 247: l_bc = (asDWORD*)247; break;
	case 248: l_bc = (asDWORD*)248; break;
	case 249: l_bc = (asDWORD*)249; break;
	case 250: l_bc = (asDWORD*)250; break;
	case 251: l_bc = (asDWORD*)251; break;
	case 252: l_bc = (asDWORD*)252; break;
	case 253: l_bc = (asDWORD*)253; break;
	case 254: l_bc = (asDWORD*)254; break;
	case 255: l_bc = (asDWORD*)255; break;

#ifdef AS_DEBUG
	default:
		asASSERT(false);
		SetInternalException(TXT_UNRECOGNIZED_BYTE_CODE);
#endif
#if defined(_MSC_VER) && !defined(AS_DEBUG)
	default:
		// This Microsoft specific code allows the
		// compiler to optimize the switch case as
		// it will know that the code will never
		// reach this point
		__assume(0);
#endif
	}

#ifdef AS_DEBUG
		asDWORD instr = *(asBYTE*)old;
		if( instr != asBC_JMP && instr != asBC_JMPP && (instr < asBC_JZ || instr > asBC_JNP) && instr != asBC_JLowZ && instr != asBC_JLowNZ &&
			instr != asBC_CALL && instr != asBC_CALLBND && instr != asBC_CALLINTF && instr != asBC_RET && instr != asBC_ALLOC && instr != asBC_CallPtr &&
			instr != asBC_JitEntry )
		{
			asASSERT( (l_bc - old) == asBCTypeSize[asBCInfo[instr].type] );
		}
#endif
	}
}

int asCContext::SetException(const char *descr)
{
	SetInternalException(descr);

	return 0;
}

void asCContext::SetInternalException(const char *descr)
{
	if( m_inExceptionHandler )
	{
		asASSERT(false); // Shouldn't happen
		return; // but if it does, at least this will not crash the application
	}

	m_status                = asEXECUTION_EXCEPTION;

	m_exceptionString       = descr;
	m_exceptionFunction     = m_currentFunction->id;

#if AS_REFERENCE_DEBUGGING
	TrackedReferences.Reset();
#endif

	if( m_currentFunction->scriptData )
	{
		m_exceptionLine    = m_currentFunction->GetLineNumber(int(m_regs.programPointer - m_currentFunction->scriptData->byteCode.AddressOf()), &m_exceptionSectionIdx);
		m_exceptionColumn  = m_exceptionLine >> 20;
		m_exceptionLine   &= 0xFFFFF;
	}
	else
	{
		m_exceptionSectionIdx = 0;
		m_exceptionLine       = 0;
		m_exceptionColumn     = 0;
	}

	if( m_exceptionCallback )
		CallExceptionCallback();
}

void asCContext::CleanReturnObject()
{
	if( m_initialFunction && m_initialFunction->DoesReturnOnStack() && m_status == asEXECUTION_FINISHED && !m_returnIsExternal)
	{
		// If function returns on stack we need to call the destructor on the returned object
		auto* retType = CastToObjectType(m_initialFunction->returnType.GetTypeInfo());
		if (retType->beh.destruct)
		{
			if (retType->GetFlags() & asOBJ_SCRIPT_OBJECT)
			{
				((asIScriptObject*)GetReturnObject())->CallDestructor(retType);
			}
			else
			{
				m_engine->CallObjectMethod(GetReturnObject(), retType->beh.destruct);
			}
		}

		return;
	}

	m_regs.objectRegister = 0;
	m_returnIsExternal = false;
}

void asCContext::CleanStack()
{
	m_inExceptionHandler = true;

	// Run the clean up code for each of the functions called
	CleanStackFrame();

	// Set the status to exception so that the stack unwind is done correctly.
	// This shouldn't be done for the current function, which is why we only
	// do this after the first CleanStackFrame() is done.
	m_status = asEXECUTION_EXCEPTION;

	while( m_callStack.GetLength() > 0 )
	{
		// Only clean up until the top most marker for a nested call
		asPWORD *s = m_callStack.AddressOf() + m_callStack.GetLength() - CALLSTACK_FRAME_SIZE;
		if( s[0] == 0 )
			break;

		PopCallState();

		CleanStackFrame();
	}

	m_inExceptionHandler = false;
}

// Interface
bool asCContext::IsVarInScope(asUINT varIndex, asUINT stackLevel)
{
	// Don't return anything if there is no bytecode, e.g. before calling Execute()
	if( m_regs.programPointer == 0 ) return false;

	if( stackLevel >= GetCallstackSize() ) return false;

	asCScriptFunction *func;
	asUINT pos;

	if( stackLevel == 0 )
	{
		func = m_currentFunction;
		if( func->scriptData == 0 ) return false;
		pos = asUINT(m_regs.programPointer - func->scriptData->byteCode.AddressOf());
	}
	else
	{
		asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize()-stackLevel-1)*CALLSTACK_FRAME_SIZE;
		func = (asCScriptFunction*)s[1];
		if( func->scriptData == 0 ) return false;
		pos = asUINT((asDWORD*)s[2] - func->scriptData->byteCode.AddressOf());
	}

	// First determine if the program position is after the variable declaration
	if( func->scriptData->variables.GetLength() <= varIndex ) return false;
	if( func->scriptData->variables[varIndex]->declaredAtProgramPos > pos ) return false;

	// Reference variables must have been declared on a previous line.
	// We don't consider them alive if we're on the current line.
	if (func->scriptData->variables[varIndex]->type.IsReference())
	{
		asUINT currentLine = GetLineNumber(stackLevel, nullptr ,nullptr);
		asUINT referenceLine = func->GetLineNumber(func->scriptData->variables[varIndex]->declaredAtProgramPos, nullptr) & 0xFFFFF;
		
		int paramOffset = func->scriptData->variables[varIndex]->stackOffset;
		bool bIsParameter = false;
		if (paramOffset < 0 || func->parameterOffsets.IndexOf(-paramOffset) != -1)
			bIsParameter = true;
		
		if (!bIsParameter && referenceLine >= currentLine)
			return false;
	}

	asUINT declaredAt = func->scriptData->variables[varIndex]->declaredAtProgramPos;

	// If the program position is after the variable declaration it is necessary
	// determine if the program position is still inside the statement block where
	// the variable was delcared.
	for( int n = 0; n < (int)func->scriptData->objVariableInfo.GetLength(); n++ )
	{
		if( func->scriptData->objVariableInfo[n].programPos >= declaredAt )
		{
			// If the current block ends between the declaredAt and current
			// program position, then we know the variable is no longer visible
			int level = 0;
			for( ; n < (int)func->scriptData->objVariableInfo.GetLength(); n++ )
			{
				if( func->scriptData->objVariableInfo[n].programPos > pos )
					break;

				if( func->scriptData->objVariableInfo[n].option == asBLOCK_BEGIN ) level++;
				if( func->scriptData->objVariableInfo[n].option == asBLOCK_END && --level < 0 )
					return false;
			}

			break;
		}
	}

	// Variable is visible
	return true;
}

// Internal
void asCContext::DetermineLiveObjects(asCArray<int> &liveObjects, asUINT stackLevel)
{
	asASSERT( stackLevel < GetCallstackSize() );

	asCScriptFunction *func;
	asUINT pos;

	if( stackLevel == 0 )
	{
		func = m_currentFunction;
		if( func->scriptData == 0 )
			return;

		pos = asUINT(m_regs.programPointer - func->scriptData->byteCode.AddressOf());

		if( m_status == asEXECUTION_EXCEPTION )
		{
			// Don't consider the last instruction as executed, as it failed with an exception
			// It's not actually necessary to decrease the exact size of the instruction. Just
			// before the current position is enough to disconsider it.
			pos--;
		}
	}
	else
	{
		asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize()-stackLevel-1)*CALLSTACK_FRAME_SIZE;
		func = (asCScriptFunction*)s[1];
		if( func->scriptData == 0 )
			return;

		pos = asUINT((asDWORD*)s[2] - func->scriptData->byteCode.AddressOf());

		// Don't consider the last instruction as executed, as the function that was called by it
		// is still being executed. If we consider it as executed already, then a value object
		// returned by value would be considered alive, which it is not.
		pos--;
	}

	// Determine which object variables that are really live ones
	liveObjects.SetLength(func->scriptData->objVariablePos.GetLength());
	memset(liveObjects.AddressOf(), 0, sizeof(int)*liveObjects.GetLength());
	for( int n = 0; n < (int)func->scriptData->objVariableInfo.GetLength(); n++ )
	{
		// Find the first variable info with a larger position than the current
		// As the variable info are always placed on the instruction right after the
		// one that initialized or freed the object, the current position needs to be
		// considered as valid.
		if( func->scriptData->objVariableInfo[n].programPos > pos )
		{
			// We've determined how far the execution ran, now determine which variables are alive
			for( --n; n >= 0; n-- )
			{
				switch( func->scriptData->objVariableInfo[n].option )
				{
				case asOBJ_UNINIT: // Object was destroyed
					{
						// TODO: optimize: This should have been done by the compiler already
						// Which variable is this?
						asUINT var = 0;
						for( asUINT v = 0; v < func->scriptData->objVariablePos.GetLength(); v++ )
							if( func->scriptData->objVariablePos[v] == func->scriptData->objVariableInfo[n].variableOffset )
							{
								var = v;
								break;
							}
						liveObjects[var] -= 1;
					}
					break;
				case asOBJ_INIT: // Object was created
					{
						// Which variable is this?
						asUINT var = 0;
						for( asUINT v = 0; v < func->scriptData->objVariablePos.GetLength(); v++ )
							if( func->scriptData->objVariablePos[v] == func->scriptData->objVariableInfo[n].variableOffset )
							{
								var = v;
								break;
							}
						liveObjects[var] += 1;
					}
					break;
				case asBLOCK_BEGIN: // Start block
					// We should ignore start blocks, since it just means the
					// program was within the block when the exception ocurred
					break;
				case asBLOCK_END: // End block
					// We need to skip the entire block, as the objects created
					// and destroyed inside this block are already out of scope
					{
						int nested = 1;
						while( nested > 0 )
						{
							int option = func->scriptData->objVariableInfo[--n].option;
							if( option == 3 )
								nested++;
							if( option == 2 )
								nested--;
						}
					}
					break;
				}
			}

			// We're done with the investigation
			break;
		}
	}
}

void asCContext::CleanArgsOnStack()
{
	if( !m_needToCleanupArgs )
		return;

	asASSERT( m_currentFunction->scriptData );

	// Find the instruction just before the current program pointer
	asDWORD *instr = m_currentFunction->scriptData->byteCode.AddressOf();
	asDWORD *prevInstr = 0;
	while( instr < m_regs.programPointer )
	{
		prevInstr = instr;
		instr += asBCTypeSize[asBCInfo[*(asBYTE*)(instr)].type];
	}

	// Determine what function was being called
	asCScriptFunction *func = 0;
	asBYTE bc = *(asBYTE*)prevInstr;
	if( bc == asBC_CALLSYS || bc == asBC_Thiscall1 )
	{
		func = (asCScriptFunction*)asBC_PTRARG(prevInstr);
	}
	else if( bc == asBC_CALL || bc == asBC_CALLINTF )
	{
		int funcId = asBC_INTARG(prevInstr);
		func = m_engine->scriptFunctions[funcId];
	}
	else if( bc == asBC_CALLBND )
	{
		int funcId = asBC_INTARG(prevInstr);
		func = m_engine->importedFunctions[funcId & ~FUNC_IMPORTED]->importedFunctionSignature;
	}
	else if( bc == asBC_CallPtr )
	{
		asUINT v;
		int var = asBC_SWORDARG0(prevInstr);

		// Find the funcdef from the local variable
		for( v = 0; v < m_currentFunction->scriptData->objVariablePos.GetLength(); v++ )
			if( m_currentFunction->scriptData->objVariablePos[v] == var )
			{
				func = CastToFuncdefType(m_currentFunction->scriptData->objVariableTypes[v])->funcdef;
				break;
			}

		if( func == 0 )
		{
			// Look in parameters
			int paramPos = 0;
			if( m_currentFunction->objectType )
				paramPos -= AS_PTR_SIZE;
			if( m_currentFunction->DoesReturnOnStack() )
				paramPos -= AS_PTR_SIZE;
			for( v = 0; v < m_currentFunction->parameterTypes.GetLength(); v++ )
			{
				if( var == paramPos )
				{
					if (m_currentFunction->parameterTypes[v].IsFuncdef())
						func = CastToFuncdefType(m_currentFunction->parameterTypes[v].GetTypeInfo())->funcdef;
					break;
				}
				paramPos -= m_currentFunction->parameterTypes[v].GetSizeOnStackDWords();
			}
		}
	}
	else
		asASSERT( false );

	asASSERT( func );

	// Clean parameters
	int offset = 0;
	if( func->objectType )
		offset += AS_PTR_SIZE;
	if( func->DoesReturnOnStack() )
		offset += AS_PTR_SIZE;
	for( asUINT n = 0; n < func->parameterTypes.GetLength(); n++ )
	{
		if( (func->parameterTypes[n].IsObject() || func->parameterTypes[n].IsFuncdef()) && !func->parameterTypes[n].IsReference() )
		{
			// TODO: cleanup: This logic is repeated twice in CleanStackFrame too. Should create a common function to share the code
			if( *(asPWORD*)&m_regs.stackPointer[offset] )
			{
				// Call the object's destructor
				asSTypeBehaviour *beh = func->parameterTypes[n].GetBehaviour();
				if (func->parameterTypes[n].GetTypeInfo()->flags & asOBJ_FUNCDEF)
				{
					(*(asCScriptFunction**)&m_regs.stackPointer[offset])->Release();
				}
				else if( func->parameterTypes[n].GetTypeInfo()->flags & asOBJ_REF )
				{
				}
				else if( func->parameterTypes[n].GetTypeInfo()->flags & asOBJ_SCRIPT_OBJECT )
				{
					asASSERT( (func->parameterTypes[n].GetTypeInfo()->flags & asOBJ_VALUE) );
					asCScriptObject* ScriptObject = *(asCScriptObject**)&m_regs.stackPointer[offset];
					ScriptObject->CallDestructor((asCObjectType*)func->parameterTypes[n].GetTypeInfo());
				}
				else
				{
					if( beh->destruct )
						m_engine->CallObjectMethod((void*)*(asPWORD*)&m_regs.stackPointer[offset], beh->destruct);

					// Free the memory
					m_engine->CallFree((void*)*(asPWORD*)&m_regs.stackPointer[offset]);
				}
				*(asPWORD*)&m_regs.stackPointer[offset] = 0;
			}
		}

		offset += func->parameterTypes[n].GetSizeOnStackDWords();
	}

	m_needToCleanupArgs = false;
}

void asCContext::CleanStackFrame()
{
	// Clean object variables on the stack
	// If the stack memory is not allocated or the program pointer
	// is not set, then there is nothing to clean up on the stack frame
	if( !m_isStackMemoryNotAllocated && m_regs.programPointer && m_currentFunction->scriptData )
	{
		// If the exception occurred while calling a function it is necessary
		// to clean up the arguments that were put on the stack.
		CleanArgsOnStack();

		// Restore the stack pointer
		asASSERT( m_currentFunction->scriptData );
		m_regs.stackPointer += m_currentFunction->scriptData->variableSpace;

		// Determine which object variables that are really live ones
		asCArray<int> liveObjects;
		DetermineLiveObjects(liveObjects, 0);

		for( asUINT n = 0; n < m_currentFunction->scriptData->objVariablePos.GetLength(); n++ )
		{
			int pos = m_currentFunction->scriptData->objVariablePos[n];
			if( n < m_currentFunction->scriptData->objVariablesOnHeap )
			{
				// Check if the pointer is initialized
				if( *(asPWORD*)&m_regs.stackFramePointer[-pos] )
				{
					// Call the object's destructor
					if (m_currentFunction->scriptData->objVariableTypes[n]->flags & asOBJ_FUNCDEF)
					{
						(*(asCScriptFunction**)&m_regs.stackFramePointer[-pos])->Release();
					}
					else if( m_currentFunction->scriptData->objVariableTypes[n]->flags & asOBJ_REF )
					{
					}
					else if( m_currentFunction->scriptData->objVariableTypes[n]->flags & asOBJ_SCRIPT_OBJECT )
					{
						auto* objType = m_currentFunction->scriptData->objVariableTypes[n];
						auto* scriptObj = *(asCScriptObject**)&m_regs.stackFramePointer[-pos];
						scriptObj->CallDestructor((asCObjectType*)objType);
					}
					else
					{
						asSTypeBehaviour *beh = &CastToObjectType(m_currentFunction->scriptData->objVariableTypes[n])->beh;
						if( beh->destruct )
							m_engine->CallObjectMethod((void*)*(asPWORD*)&m_regs.stackFramePointer[-pos], beh->destruct);
						else if( m_currentFunction->scriptData->objVariableTypes[n]->flags & asOBJ_LIST_PATTERN )
							m_engine->DestroyList((asBYTE*)*(asPWORD*)&m_regs.stackFramePointer[-pos], CastToObjectType(m_currentFunction->scriptData->objVariableTypes[n]));

						// Free the memory
						m_engine->CallFree((void*)*(asPWORD*)&m_regs.stackFramePointer[-pos]);
					}
					*(asPWORD*)&m_regs.stackFramePointer[-pos] = 0;
				}
			}
			else
			{
				asASSERT( m_currentFunction->scriptData->objVariableTypes[n]->GetFlags() & asOBJ_VALUE );

				// Only destroy the object if it is truly alive
				if( liveObjects[n] > 0 )
				{
					auto* objType = m_currentFunction->scriptData->objVariableTypes[n];
					if (objType->GetFlags() & asOBJ_SCRIPT_OBJECT)
					{
						auto* scriptObj = (asCScriptObject*)&m_regs.stackFramePointer[-pos];
						scriptObj->CallDestructor((asCObjectType*)objType);
					}
					else
					{
						asSTypeBehaviour *beh = &CastToObjectType(objType)->beh;
						if( beh->destruct )
							m_engine->CallObjectMethod((void*)(asPWORD*)&m_regs.stackFramePointer[-pos], beh->destruct);
					}
				}
			}
		}
	}
	else
		m_isStackMemoryNotAllocated = false;

	// Functions that do not own the object and parameters shouldn't do any clean up
	if( m_currentFunction->dontCleanUpOnException )
		return;

	// Clean object and parameters
	int offset = 0;
	if( m_currentFunction->objectType )
		offset += AS_PTR_SIZE;
	if( m_currentFunction->DoesReturnOnStack() )
		offset += AS_PTR_SIZE;
	for( asUINT n = 0; n < m_currentFunction->parameterTypes.GetLength(); n++ )
	{
		if( (m_currentFunction->parameterTypes[n].IsObject() ||m_currentFunction->parameterTypes[n].IsFuncdef()) && !m_currentFunction->parameterTypes[n].IsReference() )
		{
			if( *(asPWORD*)&m_regs.stackFramePointer[offset] )
			{
				// Call the object's destructor
				asSTypeBehaviour *beh = m_currentFunction->parameterTypes[n].GetBehaviour();
				if (m_currentFunction->parameterTypes[n].GetTypeInfo()->flags & asOBJ_FUNCDEF)
				{
					(*(asCScriptFunction**)&m_regs.stackFramePointer[offset])->Release();
				}
				else if( m_currentFunction->parameterTypes[n].GetTypeInfo()->flags & asOBJ_REF )
				{
				}
				else if( m_currentFunction->parameterTypes[n].GetTypeInfo()->flags & asOBJ_SCRIPT_OBJECT )
				{
					asASSERT( (m_currentFunction->parameterTypes[n].GetTypeInfo()->flags & asOBJ_VALUE) );
					asCScriptObject* ScriptObject = *(asCScriptObject**)&m_regs.stackFramePointer[offset];
					ScriptObject->CallDestructor((asCObjectType*)m_currentFunction->parameterTypes[n].GetTypeInfo());
				}
				else
				{
					if( beh->destruct )
						m_engine->CallObjectMethod((void*)*(asPWORD*)&m_regs.stackFramePointer[offset], beh->destruct);

					// Free the memory
					m_engine->CallFree((void*)*(asPWORD*)&m_regs.stackFramePointer[offset]);
				}
				*(asPWORD*)&m_regs.stackFramePointer[offset] = 0;
			}
		}

		offset += m_currentFunction->parameterTypes[n].GetSizeOnStackDWords();
	}
}

// interface
int asCContext::GetExceptionLineNumber(int *column, const char **sectionName)
{
	if( GetState() != asEXECUTION_EXCEPTION ) return asERROR;

	if( column ) *column = m_exceptionColumn;

	if( sectionName )
	{
		// The section index can be -1 if the exception was raised in a generated function, e.g. $fact for templates
		if( m_exceptionSectionIdx >= 0 )
			*sectionName = m_engine->scriptSectionNames[m_exceptionSectionIdx]->AddressOf();
		else
			*sectionName = 0;
	}

	return m_exceptionLine;
}

// interface
asIScriptFunction *asCContext::GetExceptionFunction()
{
	if( GetState() != asEXECUTION_EXCEPTION ) return 0;

	return m_engine->scriptFunctions[m_exceptionFunction];
}

// interface
const char *asCContext::GetExceptionString()
{
	if( GetState() != asEXECUTION_EXCEPTION ) return 0;

	return m_exceptionString.AddressOf();
}

// interface
asEContextState asCContext::GetState() const
{
	return m_status;
}

// interface
int asCContext::SetLineCallback(asLineCallback LineCallback)
{
	m_lineCallback = LineCallback;
	return 0;
}

int asCContext::SetLoopDetectionCallback(asLineCallback Callback)
{
	m_loopDetectionCallback = Callback;
	return 0;
}

int asCContext::SetStackPopCallback(asStackPopCallback StackPopCallback)
{
	m_stackPopCallback = StackPopCallback;
	return 0;
}

// interface
int asCContext::SetExceptionCallback(asSFuncPtr callback, void *obj, int callConv)
{
	m_exceptionCallback = true;
	m_exceptionCallbackObj = obj;
	bool isObj = false;
	if( (unsigned)callConv == asCALL_GENERIC || (unsigned)callConv == asCALL_THISCALL_OBJFIRST || (unsigned)callConv == asCALL_THISCALL_OBJLAST )
		return asNOT_SUPPORTED;
	if( (unsigned)callConv >= asCALL_THISCALL )
	{
		isObj = true;
		if( obj == 0 )
		{
			m_exceptionCallback = false;
			return asINVALID_ARG;
		}
	}
	int r = DetectCallingConvention(isObj, callback, callConv, 0, nullptr, &m_exceptionCallbackFunc);
	if( r < 0 ) m_exceptionCallback = false;
	return r;
}

void asCContext::CallExceptionCallback()
{
	if( m_exceptionCallbackFunc.callConv < ICC_THISCALL )
		m_engine->CallGlobalFunction(this, m_exceptionCallbackObj, &m_exceptionCallbackFunc, 0);
	else
		m_engine->CallObjectMethod(m_exceptionCallbackObj, this, &m_exceptionCallbackFunc, 0);
}

#ifndef AS_NO_EXCEPTIONS
// internal
void asCContext::HandleAppException()
{
	// This method is called from within a catch(...) block
	if (m_engine->translateExceptionCallback)
	{
		// Allow the application to translate the application exception to a proper exception string
		if (m_engine->translateExceptionCallbackFunc.callConv < ICC_THISCALL)
			m_engine->CallGlobalFunction(this, m_engine->translateExceptionCallbackObj, &m_engine->translateExceptionCallbackFunc, 0);
		else
			m_engine->CallObjectMethod(m_engine->translateExceptionCallbackObj, this, &m_engine->translateExceptionCallbackFunc, 0);
	}

	// Make sure an exception is set even if the application decides not to do any specific translation
	if( m_status != asEXECUTION_EXCEPTION )
		SetException(TXT_EXCEPTION_CAUGHT);
}
#endif

// interface
void asCContext::ClearLineCallback()
{
	m_lineCallback = nullptr;
}

// interface
void asCContext::ClearStackPopCallback()
{
	 m_stackPopCallback = nullptr;
}

// interface
void asCContext::ClearExceptionCallback()
{
	m_exceptionCallback = false;
}

int asCContext::CallGeneric(asCScriptFunction *descr)
{
	asSSystemFunctionInterface *sysFunc = descr->sysFuncIntf;
	void (*func)(asIScriptGeneric*) = (void (*)(asIScriptGeneric*))sysFunc->func;
	int popSize = sysFunc->paramSize;
	asDWORD *args = m_regs.stackPointer;

	// Verify the object pointer if it is a class method
	void *currentObject = 0;
	asASSERT( sysFunc->callConv == ICC_GENERIC_FUNC || sysFunc->callConv == ICC_GENERIC_METHOD );
	if( sysFunc->callConv == ICC_GENERIC_METHOD )
	{
		// The object pointer should be popped from the context stack
		popSize += AS_PTR_SIZE;

		// Check for null pointer
		currentObject = (void*)*(asPWORD*)(args);
		if( currentObject == 0 )
		{
			SetInternalException(TXT_NULL_POINTER_ACCESS);
			return 0;
		}

		asASSERT( sysFunc->baseOffset == 0 );
	}

	if( descr->DoesReturnOnStack() )
	{
		// Skip the address where the return value will be stored
		popSize += AS_PTR_SIZE;
	}

	asCGeneric gen(m_engine, descr, currentObject, args);

	auto* tld = m_regs.tld;
	auto* prevActiveFunction = tld->activeFunction;
	tld->activeFunction = descr;

#ifdef AS_NO_EXCEPTIONS
	func(&gen);
#else
	// This try/catch block is to catch potential exception that may 
	// be thrown by the registered function. 
	try
	{
		func(&gen);
	}
	catch (...)
	{
		// Convert the exception to a script exception so the VM can 
		// properly report the error to the application and then clean up
		HandleAppException();
	}
#endif

	tld->activeFunction = prevActiveFunction;

	m_regs.valueRegister = gen.returnVal;
	m_regs.objectRegister = gen.objectRegister;
	m_regs.objectType = descr->returnType.GetTypeInfo();

	// Return how much should be popped from the stack
	return popSize;
}

int asCContext::CallFunctionCaller(asCScriptFunction* descr)
{
	asSSystemFunctionInterface *sysFunc = descr->sysFuncIntf;
	void (*func)(asIScriptGeneric*) = (void (*)(asIScriptGeneric*))sysFunc->func;

	asDWORD* StackArgs = m_regs.stackPointer;
	m_callingSystemFunction = descr;

	void* FunctionArgs[32];
	void* ReturnAddress;
	unsigned int ArgIndex = 0;

#if WITH_EDITOR
	if (descr->traits.GetTrait(asTRAIT_USES_WORLDCONTEXT))
	{
		extern thread_local bool GIsInAngelscriptThreadSafeFunction;
		if (GIsInAngelscriptThreadSafeFunction)
		{
			SetInternalException("Calling a function that requires WorldContext from a BlueprintThreadSafe function is not allowed and can crash!");
			return 0;
		}

		extern thread_local bool GIsAngelscriptWorldContextAvailable;
		if (!GIsAngelscriptWorldContextAvailable && FAngelscriptManager::Get().ConfigSettings->bErrorWhenUsingInvalidWorldContext)
		{
			SetInternalException("Calling a function that requires WorldContext, but the current object is not in a world.");
			return 0;
		}
	}
#endif

	// Verify the object pointer if it is a class method
	void *currentObject = 0;
	if( sysFunc->callConv >= ICC_THISCALL )
	{
		// The object pointer should be popped from the context stack
		FunctionArgs[ArgIndex] = *(void**)&StackArgs[0];

		if( FunctionArgs[ArgIndex] == nullptr )
		{
			SetInternalException(TXT_NULL_POINTER_ACCESS);
			return 0;
		}

		// Skip object pointer
		ArgIndex++;
	}

	// Some system functions want to know the script function that is being called
	if (sysFunc->passFirstParamMetaData != asEFirstParamMetaData::None)
	{
		if (sysFunc->passFirstParamMetaData == asEFirstParamMetaData::ScriptFunction)
		{
			FunctionArgs[ArgIndex] = descr;
			ArgIndex++;
		}
		else
		{
			FunctionArgs[ArgIndex] = descr->objectType;
			ArgIndex++;
		}
	}

	if( descr->DoesReturnOnStack() )
	{
		// Skip the address where the return value will be stored
		if (sysFunc->callConv >= ICC_THISCALL)
			ReturnAddress = *(void**)(StackArgs + AS_PTR_SIZE);
		else
			ReturnAddress = *(void**)StackArgs;

		// TODO: Do we need to destroy the object here in case of exception?
	}
	else if (descr->returnType.IsObjectHandle() && !descr->returnType.IsReference())
	{
		ReturnAddress = &m_regs.objectRegister;
	}
	else
	{
		ReturnAddress = &m_regs.valueRegister;
	}

	for (int i = 0, paramCount = descr->parameterOffsets.GetLength(); i < paramCount; ++i)
	{
		auto& paramType = descr->parameterTypes[i];
		auto paramOffset = descr->parameterOffsets[i];

		if (paramType.GetTokenType() == ttQuestion)
		{
			FunctionArgs[ArgIndex] = *(void**)&StackArgs[paramOffset];
			++ArgIndex;
			FunctionArgs[ArgIndex] = &StackArgs[paramOffset+2];
		}
		else if (paramType.IsObject() || paramType.IsReference())
		{
			FunctionArgs[ArgIndex] = *(void**)&StackArgs[paramOffset];
		}
		else
		{
			FunctionArgs[ArgIndex] = &StackArgs[paramOffset];
		}

		++ArgIndex;
	}

	auto* tld = m_regs.tld;
	auto* prevActiveFunction = tld->activeFunction;
	tld->activeFunction = descr;

	if (sysFunc->caller.type == 1)
		sysFunc->caller.FunctionCaller(sysFunc->func, &FunctionArgs[0], ReturnAddress);
	else //if (sysFunc->caller.type == 2)
		sysFunc->caller.MethodCaller(sysFunc->method, &FunctionArgs[0], ReturnAddress);

	tld->activeFunction = prevActiveFunction;
	m_regs.objectType = descr->returnType.GetTypeInfo();
	m_callingSystemFunction = nullptr;

	// Return how much should be popped from the stack
	return descr->totalSpaceBeforeFunction;
}

// interface
int asCContext::GetVarCount(asUINT stackLevel)
{
	asIScriptFunction *func = GetFunction(stackLevel);
	if( func == 0 ) return asINVALID_ARG;

	return func->GetVarCount();
}

// interface
const char *asCContext::GetVarName(asUINT varIndex, asUINT stackLevel)
{
	asIScriptFunction *func = GetFunction(stackLevel);
	if( func == 0 ) return 0;

	const char *name = 0;
	int r = func->GetVar(varIndex, &name);
	return r >= 0 ? name : 0;
}

// interface
const char *asCContext::GetVarDeclaration(asUINT varIndex, asUINT stackLevel, bool includeNamespace)
{
	asIScriptFunction *func = GetFunction(stackLevel);
	if( func == 0 ) return 0;

	return func->GetVarDecl(varIndex, includeNamespace);
}

// interface
int asCContext::GetVarTypeId(asUINT varIndex, asUINT stackLevel)
{
	asIScriptFunction *func = GetFunction(stackLevel);
	if( func == 0 ) return asINVALID_ARG;

	int typeId;
	int r = func->GetVar(varIndex, 0, &typeId);
	return r < 0 ? r : typeId;
}

// interface
void *asCContext::GetAddressOfVar(asUINT varIndex, asUINT stackLevel)
{
	// Don't return anything if there is no bytecode, e.g. before calling Execute()
	if( m_regs.programPointer == 0 ) return 0;

	if( stackLevel >= GetCallstackSize() ) return 0;

	asCScriptFunction *func;
	asDWORD *sf;
	if( stackLevel == 0 )
	{
		func = m_currentFunction;
		sf = m_regs.stackFramePointer;
	}
	else
	{
		asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize()-stackLevel-1)*CALLSTACK_FRAME_SIZE;
		func = (asCScriptFunction*)s[1];
		sf = (asDWORD*)s[0];
	}

	if( func == 0 )
		return 0;

	if( func->scriptData == 0 )
		return 0;

	if( varIndex >= func->scriptData->variables.GetLength() )
		return 0;

	// For object variables it's necessary to dereference the pointer to get the address of the value
	// Reference parameters must also be dereferenced to give the address of the value
	int pos = func->scriptData->variables[varIndex]->stackOffset;
	if( (func->scriptData->variables[varIndex]->type.IsObject() && !func->scriptData->variables[varIndex]->type.IsObjectHandle()) || (pos <= 0) )
	{
		// Determine if the object is really on the heap
		bool onHeap = false;
		if( func->scriptData->variables[varIndex]->type.IsObject() &&
			!func->scriptData->variables[varIndex]->type.IsObjectHandle() )
		{
			onHeap = true;
			if( func->scriptData->variables[varIndex]->type.GetTypeInfo()->GetFlags() & asOBJ_VALUE )
			{
				for( asUINT n = 0; n < func->scriptData->objVariablePos.GetLength(); n++ )
				{
					if( func->scriptData->objVariablePos[n] == pos )
					{
						onHeap = n < func->scriptData->objVariablesOnHeap;

						if( !onHeap )
						{
							// If the object on the stack is not initialized return a null pointer instead
							asCArray<int> liveObjects;
							DetermineLiveObjects(liveObjects, stackLevel);

							if( liveObjects[n] <= 0 )
								return 0;
						}

						break;
					}
				}
			}
		}

		// If it wasn't an object on the heap, then check if it is a reference parameter
		if( !onHeap && pos <= 0 )
		{
			// Determine what function argument this position matches
			int stackPos = 0;
			if( func->objectType )
				stackPos -= AS_PTR_SIZE;

			if( func->DoesReturnOnStack() )
				stackPos -= AS_PTR_SIZE;

			for( asUINT n = 0; n < func->parameterTypes.GetLength(); n++ )
			{
				if( stackPos == pos )
				{
					// The right argument was found. Is this a reference parameter?
					if( func->inOutFlags[n] != asTM_NONE )
						onHeap = true;

					break;
				}
				stackPos -= func->parameterTypes[n].GetSizeOnStackDWords();
			}
		}

		if( onHeap )
			return *(void**)(sf - func->scriptData->variables[varIndex]->stackOffset);
	}
	else if (func->scriptData->variables[varIndex]->type.IsReference())
	{
		return *(void**)(sf - func->scriptData->variables[varIndex]->stackOffset);
	}

	return sf - func->scriptData->variables[varIndex]->stackOffset;
}

// interface
// returns the typeId of the 'this' object at the given call stack level (0 for current)
// returns 0 if the function call at the given stack level is not a method
int asCContext::GetThisTypeId(asUINT stackLevel)
{
	asIScriptFunction *func = GetFunction(stackLevel);
	if( func == 0 ) return asINVALID_ARG;

	if( func->GetObjectType() == 0 )
		return 0; // not in a method

	// create a datatype
	asCDataType dt = asCDataType::CreateType((asCObjectType*)func->GetObjectType(), false);

	// return a typeId from the data type
	return m_engine->GetTypeIdFromDataType(dt);
}

// interface
// returns the 'this' object pointer at the given call stack level (0 for current)
// returns 0 if the function call at the given stack level is not a method
void *asCContext::GetThisPointer(asUINT stackLevel)
{
	if( stackLevel >= GetCallstackSize() )
		return 0;

	asCScriptFunction *func;
	asDWORD *sf;
	if( stackLevel == 0 )
	{
		func = m_currentFunction;
		sf = m_regs.stackFramePointer;
	}
	else
	{
		asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize()-stackLevel-1)*CALLSTACK_FRAME_SIZE;
		func = (asCScriptFunction*)s[1];
		sf = (asDWORD*)s[0];
	}

	// Frames that were pushed by PushState() and contain system function calls
	// do not have a stack frame pointer, so this will not work.
	if (sf == 0)
		return 0;

	if( func == 0 )
		return 0;

	if( func->objectType == 0 )
		return 0; // not in a method

	void *thisPointer = (void*)*(asPWORD*)(sf);
	if( thisPointer == 0 )
	{
		return 0;
	}

	// NOTE: this returns the pointer to the 'this' while the GetVarPointer functions return
	// a pointer to a pointer. I can't imagine someone would want to change the 'this'
	return thisPointer;
}

void *asCContext::GetStackFrame(asUINT stackLevel)
{
	if( stackLevel >= GetCallstackSize() )
		return 0;

	asDWORD *sf;
	if( stackLevel == 0 )
	{
		sf = m_regs.stackFramePointer;
	}
	else
	{
		asPWORD *s = m_callStack.AddressOf() + (GetCallstackSize()-stackLevel-1)*CALLSTACK_FRAME_SIZE;
		sf = (asDWORD*)s[0];
	}

	return sf;
}

int asCContext::GetStackFrameSize(asUINT stackLevel) const
{
	return m_stackBlockSize << stackLevel;
}




// TODO: Move these to as_utils.cpp

struct POW_INFO
{
	asQWORD MaxBaseu64;
	asDWORD MaxBasei64;
	asWORD  MaxBaseu32;
	asWORD  MaxBasei32;
	char    HighBit;
};

const POW_INFO pow_info[] =
{
	{          0ULL,          0UL,     0,     0, 0 },  // 0 is a special case
	{          0ULL,          0UL,     0,     0, 1 },  // 1 is a special case
    { 3037000499ULL, 2147483647UL, 65535, 46340, 2 },  // 2
    {    2097152ULL,    1664510UL,  1625,  1290, 2 },  // 3
    {      55108ULL,      46340UL,   255,   215, 3 },  // 4
    {       6208ULL,       5404UL,    84,    73, 3 },  // 5
    {       1448ULL,       1290UL,    40,    35, 3 },  // 6
    {        511ULL,        463UL,    23,    21, 3 },  // 7
    {        234ULL,        215UL,    15,    14, 4 },  // 8
    {        128ULL,        118UL,    11,    10, 4 },  // 9
    {         78ULL,         73UL,     9,     8, 4 },  // 10
    {         52ULL,         49UL,     7,     7, 4 },  // 11
    {         38ULL,         35UL,     6,     5, 4 },  // 12
    {         28ULL,         27UL,     5,     5, 4 },  // 13
    {         22ULL,         21UL,     4,     4, 4 },  // 14
    {         18ULL,         17UL,     4,     4, 4 },  // 15
    {         15ULL,         14UL,     3,     3, 5 },  // 16
    {         13ULL,         12UL,     3,     3, 5 },  // 17
    {         11ULL,         10UL,     3,     3, 5 },  // 18
    {          9ULL,          9UL,     3,     3, 5 },  // 19
    {          8ULL,          8UL,     3,     2, 5 },  // 20
    {          8ULL,          7UL,     2,     2, 5 },  // 21
    {          7ULL,          7UL,     2,     2, 5 },  // 22
    {          6ULL,          6UL,     2,     2, 5 },  // 23
    {          6ULL,          5UL,     2,     2, 5 },  // 24
    {          5ULL,          5UL,     2,     2, 5 },  // 25
    {          5ULL,          5UL,     2,     2, 5 },  // 26
    {          5ULL,          4UL,     2,     2, 5 },  // 27
    {          4ULL,          4UL,     2,     2, 5 },  // 28
    {          4ULL,          4UL,     2,     2, 5 },  // 29
    {          4ULL,          4UL,     2,     2, 5 },  // 30
    {          4ULL,          4UL,     2,     1, 5 },  // 31
    {          3ULL,          3UL,     1,     1, 6 },  // 32
    {          3ULL,          3UL,     1,     1, 6 },  // 33
    {          3ULL,          3UL,     1,     1, 6 },  // 34
    {          3ULL,          3UL,     1,     1, 6 },  // 35
    {          3ULL,          3UL,     1,     1, 6 },  // 36
    {          3ULL,          3UL,     1,     1, 6 },  // 37
    {          3ULL,          3UL,     1,     1, 6 },  // 38
    {          3ULL,          3UL,     1,     1, 6 },  // 39
    {          2ULL,          2UL,     1,     1, 6 },  // 40
    {          2ULL,          2UL,     1,     1, 6 },  // 41
    {          2ULL,          2UL,     1,     1, 6 },  // 42
    {          2ULL,          2UL,     1,     1, 6 },  // 43
    {          2ULL,          2UL,     1,     1, 6 },  // 44
    {          2ULL,          2UL,     1,     1, 6 },  // 45
    {          2ULL,          2UL,     1,     1, 6 },  // 46
    {          2ULL,          2UL,     1,     1, 6 },  // 47
    {          2ULL,          2UL,     1,     1, 6 },  // 48
    {          2ULL,          2UL,     1,     1, 6 },  // 49
    {          2ULL,          2UL,     1,     1, 6 },  // 50
    {          2ULL,          2UL,     1,     1, 6 },  // 51
    {          2ULL,          2UL,     1,     1, 6 },  // 52
    {          2ULL,          2UL,     1,     1, 6 },  // 53
    {          2ULL,          2UL,     1,     1, 6 },  // 54
    {          2ULL,          2UL,     1,     1, 6 },  // 55
    {          2ULL,          2UL,     1,     1, 6 },  // 56
    {          2ULL,          2UL,     1,     1, 6 },  // 57
    {          2ULL,          2UL,     1,     1, 6 },  // 58
    {          2ULL,          2UL,     1,     1, 6 },  // 59
    {          2ULL,          2UL,     1,     1, 6 },  // 60
    {          2ULL,          2UL,     1,     1, 6 },  // 61
    {          2ULL,          2UL,     1,     1, 6 },  // 62
	{          2ULL,          1UL,     1,     1, 6 },  // 63
};

int as_powi(int base, int exponent, bool& isOverflow)
{
	if( exponent < 0 )
	{
		if( base == 0 )
			// Divide by zero
			isOverflow = true;
		else
			// Result is less than 1, so it truncates to 0
			isOverflow = false;

		return 0;
	}
	else if( exponent == 0 && base == 0 )
	{
		// Domain error
		isOverflow = true;
		return 0;
	}
	else if( exponent >= 31 )
	{
		switch( base )
		{
		case -1:
			isOverflow = false;
			return exponent & 1 ? -1 : 1;
		case 0:
			isOverflow = false;
			break;
		case 1:
			isOverflow = false;
			return 1;
		default:
			isOverflow = true;
			break;
		}
		return 0;
	}
	else
	{
		const asWORD max_base = pow_info[exponent].MaxBasei32;
		const char high_bit = pow_info[exponent].HighBit;
		if( max_base != 0 && max_base < (base < 0 ? -base : base) )
		{
			isOverflow = true;
			return 0;  // overflow
		}

		int result = 1;
		switch( high_bit )
		{
		case 5:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 4:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 3:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 2:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 1:
			if( exponent ) result *= base;
		default:
			isOverflow = false;
			return result;
		}
	}
}

asDWORD as_powu(asDWORD base, asDWORD exponent, bool& isOverflow)
{
	if( exponent == 0 && base == 0 )
	{
		// Domain error
		isOverflow = true;
		return 0;
	}
	else if( exponent >= 32 )
	{
		switch( base )
		{
		case 0:
			isOverflow = false;
			break;
		case 1:
			isOverflow = false;
			return 1;
		default:
			isOverflow = true;
			break;
		}
		return 0;
	}
	else
	{
		const asWORD max_base = pow_info[exponent].MaxBaseu32;
		const char high_bit = pow_info[exponent].HighBit;
		if( max_base != 0 && max_base < base )
		{
			isOverflow = true;
			return 0;  // overflow
		}

		asDWORD result = 1;
		switch( high_bit )
		{
		case 5:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 4:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 3:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 2:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 1:
			if( exponent ) result *= base;
		default:
			isOverflow = false;
			return result;
		}
	}
}

asINT64 as_powi64(asINT64 base, asINT64 exponent, bool& isOverflow)
{
	if( exponent < 0 )
	{
		if( base == 0 )
			// Divide by zero
			isOverflow = true;
		else
			// Result is less than 1, so it truncates to 0
			isOverflow = false;

		return 0;
	}
	else if( exponent == 0 && base == 0 )
	{
		// Domain error
		isOverflow = true;
		return 0;
	}
	else if( exponent >= 63 )
	{
		switch( base )
		{
		case -1:
			isOverflow = false;
			return exponent & 1 ? -1 : 1;
		case 0:
			isOverflow = false;
			break;
		case 1:
			isOverflow = false;
			return 1;
		default:
			isOverflow = true;
			break;
		}
		return 0;
	}
	else
	{
		const asDWORD max_base = pow_info[exponent].MaxBasei64;
		const char high_bit = pow_info[exponent].HighBit;
		if( max_base != 0 && max_base < (base < 0 ? -base : base) )
		{
			isOverflow = true;
			return 0;  // overflow
		}

		asINT64 result = 1;
		switch( high_bit )
		{
		case 6:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 5:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 4:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 3:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 2:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 1:
			if( exponent ) result *= base;
		default:
			isOverflow = false;
			return result;
		}
	}
}

asQWORD as_powu64(asQWORD base, asQWORD exponent, bool& isOverflow)
{
	if( exponent == 0 && base == 0 )
	{
		// Domain error
		isOverflow = true;
		return 0;
	}
	else if( exponent >= 64 )
	{
		switch( base )
		{
		case 0:
			isOverflow = false;
			break;
		case 1:
			isOverflow = false;
			return 1;
		default:
			isOverflow = true;
			break;
		}
		return 0;
	}
	else
	{
		const asQWORD max_base = pow_info[exponent].MaxBaseu64;
		const char high_bit = pow_info[exponent].HighBit;
		if( max_base != 0 && max_base < base )
		{
			isOverflow = true;
			return 0;  // overflow
		}

		asQWORD result = 1;
		switch( high_bit )
		{
		case 6:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 5:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 4:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 3:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 2:
			if( exponent & 1 ) result *= base;
			exponent >>= 1;
			base *= base;
		case 1:
			if( exponent ) result *= base;
		default:
			isOverflow = false;
			return result;
		}
	}
}

#if AS_REFERENCE_DEBUGGING
void asCContext::InvalidateReferencesToMemoryBlock(void* Address, SIZE_T Size)
{
	SIZE_T LowerBound = (SIZE_T)Address;
	SIZE_T UpperBound = LowerBound + Size;

	for (void** Reference : TrackedReferences)
	{
		SIZE_T TrackedAddress = (SIZE_T)*Reference;
		if (TrackedAddress >= LowerBound && TrackedAddress < UpperBound)
		{
			// This reference points to the invalidated memory block, so null it out
			*Reference = nullptr;
		}
	}
}
#endif

END_AS_NAMESPACE

