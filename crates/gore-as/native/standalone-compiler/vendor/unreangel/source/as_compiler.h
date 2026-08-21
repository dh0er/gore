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
// as_compiler.h
//
// The class that does the actual compilation of the functions
//



#ifndef AS_COMPILER_H
#define AS_COMPILER_H

#include "as_config.h"

#ifndef AS_NO_COMPILER

#include "as_builder.h"
#include "as_scriptfunction.h"
#include "as_variablescope.h"
#include "as_bytecode.h"
#include "as_array.h"
#include "as_datatype.h"

BEGIN_AS_NAMESPACE

// This class represents the value of an expression as evaluated by the compiler.
// It holds information such as the type of the value, stack offset for a local 
// variable, value of constants, whether the value can be modified (i.e. lvalue), etc.
struct asCExprValue
{
	asCExprValue();
	void Set(const asCDataType &dataType);

	void SetVariable(const asCDataType &dataType, int stackOffset, bool isTemporary);
	void SetConstantB(const asCDataType &dataType, asBYTE value);
	void SetConstantQW(const asCDataType &dataType, asQWORD value);
	void SetConstantDW(const asCDataType &dataType, asDWORD value);
	void SetConstantW(const asCDataType &dataType, asWORD value);
	void SetConstantF(const asCDataType &dataType, float value);
	void SetConstantD(const asCDataType &dataType, double value);
	void SetConstantB(asBYTE value);
	void SetConstantW(asWORD value);
	void SetConstantQW(asQWORD value);
	void SetConstantDW(asDWORD value);
	void SetConstantF(float value);
	void SetConstantD(double value);
	asBYTE  GetConstantB();
	asWORD  GetConstantW();
	asQWORD GetConstantQW();
	asDWORD GetConstantDW();
	float   GetConstantF();
	double  GetConstantD();

	void SetConstantData(const asCDataType &dataType, asQWORD value);
	asQWORD GetConstantData();

	void SetNullConstant();
	void SetUndefinedFuncHandle(asCScriptEngine *engine);
	void SetVoid();
	void SetDummy();

	bool IsUndefinedFuncHandle() const;
	bool IsNullConstant() const;
	bool IsVoid() const;

	asCDataType dataType;
	bool  isLValue : 1; // Can this value be updated in assignment, or increment operators, etc
	bool  isTemporary : 1;
	bool  isConstant : 1;
	bool  isVariable : 1;
	bool  isExplicitHandle : 1;
	bool  isRefToLocal : 1; // The reference may be to a local variable
	bool  isAppBindProperty : 1; // whether this is a property bound from a C++ bind that we're accessing
	bool  constantHasBackingDouble : 1;
	short dummy : 7;
	short stackOffset;
	double backingDouble;

private:
	// These values must not be accessed directly in order to avoid problems with endianess. 
	// Use the appropriate accessor methods instead
	union
	{
		asQWORD qwordValue;
		double  doubleValue;
		asDWORD dwordValue;
		float   floatValue;
		asWORD  wordValue;
		asBYTE  byteValue;
	};
};

struct asCExprContext;

// This class holds information for arguments that needs to be 
// cleaned up after the result of a function has been evaluated.
struct asSDeferredParam
{
	asSDeferredParam() {argNode = 0; origExpr = 0;}

	asCScriptNode  *argNode;
	asCExprValue    argType;
	int             argInOutFlags;
	asCExprContext *origExpr;
};

// TODO: refactor: asCExprContext should have indicators to inform where the value is, 
//                 i.e. if the reference to an object is pushed on the stack or not, etc

// This class holds information about an expression that is being evaluated, e.g.
// the current bytecode, ambiguous symbol names, property accessors, etc.
struct asCExprContext
{
	asCExprContext(asCBuilder* builder);
	~asCExprContext();
	void Clear();
	bool IsClassMethod() const;
	bool IsGlobalFunc() const;
	void SetLambda(asCScriptNode *funcDecl);
	bool IsLambda() const;
	void SetVoidExpression();
	bool IsVoidExpression() const;
	void Merge(asCExprContext *after);
	void SetAnonymousInitList(asCScriptNode *initList);
	bool IsAnonymousInitList() const;

	void ClearNoDiscard()
	{
		noDiscardExpression = false;
		noDiscardFunction.Clear();
	}

	asCByteCode bc;
	asCExprValue type;
	int  property_get;
	int  property_set;
	bool property_const;   // If the object that is being accessed through property accessor is read-only
	bool property_handle;  // If the property accessor is called on an object stored in a handle
	bool property_ref;     // If the property accessor is called on a reference
	bool isVoidExpression; // Set to true if the expression is an explicit 'void', e.g. used to ignore out parameters in func calls
	bool isCleanArg;       // Set to true if the expression has only been initialized with default constructor
	int  shadowed_property_get;
	int  shadowed_property_set;
	asCExprContext *property_arg;
	asCArray<asSDeferredParam> deferredParams;
	asCScriptNode  *exprNode;
	asCExprContext *origExpr;
	// TODO: cleanup: use ambiguousName and an enum to say if it is a method, global func, or enum value
	asCString methodName;
	asCString enumValue;
	bool isAnonymousInitList; // Set to true if the expression is an init list for which the type has not yet been determined
	bool noDiscardExpression;
	asCString noDiscardFunction;
};

struct asSOverloadCandidate
{
	asSOverloadCandidate() : funcId(0), cost(0) {}
	asSOverloadCandidate(int _id, asUINT _cost) : funcId(_id), cost(_cost) {}
	int funcId;
	asUINT cost;
};

struct asSNamedArgument
{
	asCString name;
	asCExprContext *ctx;
	asUINT match;
};

enum EImplicitConv
{
	asIC_IMPLICIT_CONV,
	asIC_EXPLICIT_REF_CAST,
	asIC_EXPLICIT_VAL_CAST,
	asIC_IMPLICIT_CONV_ALLOW_NARROWING,
};

enum EConvCost
{
	asCC_NO_CONV,
	asCC_CONST_CONV,
	asCC_FLOAT_CONSTANT_WIDEN_CONV,
	asCC_FLOAT_CONSTANT_NARROW_CONV,
	asCC_INT_TO_DOUBLE_CONV,
	asCC_FLOAT_WIDEN_TO_DOUBLE_CONV,
	asCC_INT_TO_FLOAT_CONV,
	asCC_PRIMITIVE_SIZE_CONV,
	asCC_SIGNED_CONV,
	asCC_SHADOW_CONV,
	asCC_DOUBLE_NARROW_TO_FLOAT_CONV,
	asCC_REF_CONV,
	asCC_FLOAT_TO_INT_CONV,
	asCC_DOUBLE_TO_INT_CONV,
	asCC_OBJ_TO_PRIMITIVE_CONV,
	asCC_TO_OBJECT_CONV,
	asCC_VARIABLE_CONV,
	asCC_ENUM_SAME_SIZE_CONV,
	asCC_ENUM_DIFF_SIZE_CONV,
};

class asCCompiler
{
public:
	asCCompiler(asCBuilder* builder);
	~asCCompiler();

	int CompileFunction(asCBuilder *builder, asCScriptCode *script, asCArray<asCString> &parameterNames, asCScriptNode *func, asCScriptFunction *outFunc, sClassDeclaration *classDecl);
	int CompileDefaultConstructor(asCBuilder *builder, asCScriptCode *script, asCScriptNode *node, asCScriptFunction *outFunc, sClassDeclaration *classDecl);
	int CompileDefaultDestructor(asCBuilder *builder, asCScriptCode *script, asCScriptNode *node, asCScriptFunction *outFunc, sClassDeclaration *classDecl);
	int CompileFactory(asCBuilder *builder, asCScriptCode *script, asCScriptFunction *outFunc);
	int CompileGlobalVariable(asCBuilder *builder, asCScriptCode *script, asCScriptNode *expr, sGlobalVariableDescription *gvar, asCScriptFunction *outFunc);

protected:
	friend class asCBuilder;

	void Reset(asCBuilder *builder, asCScriptCode *script, asCScriptFunction *outFunc);

	struct FStopCondition
	{
		bool hasReturn = false;
		bool hasBreak = false;
		bool hasContinue = false;
		bool hasMultiple = false;

		void Reset()
		{
			hasReturn = false;
			hasBreak = false;
			hasContinue = false;
			hasMultiple = false;
		}

		bool HasAny() const
		{
			return hasReturn || hasBreak || hasContinue || hasMultiple;
		}

		FStopCondition And(const FStopCondition& Other) const
		{
			FStopCondition Out;
			Out.hasReturn = hasReturn && Other.hasReturn;
			Out.hasBreak = hasBreak && Other.hasBreak;
			Out.hasContinue = hasContinue && Other.hasContinue;
			Out.hasMultiple = HasAny() && Other.HasAny();
			return Out;
		}
	};

	// Statements
	void CompileStatementBlock(asCScriptNode *block, bool ownVariableScope, FStopCondition& stopCondition, asCByteCode *bc);
	void CompileDeclaration(asCScriptNode *decl, asCByteCode *bc);
	void CompileStatement(asCScriptNode *statement, FStopCondition& stopCondition, asCByteCode *bc);
	void CompileIfStatement(asCScriptNode *node, FStopCondition& stopCondition, asCByteCode *bc);
	void CompileSwitchStatement(asCScriptNode *node, FStopCondition& stopCondition, asCByteCode *bc);
	void CompileCase(asCScriptNode *node, FStopCondition& stopCondition, bool *hasFallthrough, asCByteCode *bc);
	void CompileForStatement(asCScriptNode *node, asCByteCode *bc);
	void CompileWhileStatement(asCScriptNode *node, asCByteCode *bc);
	void CompileDoWhileStatement(asCScriptNode *node, asCByteCode *bc);
	void CompileBreakStatement(asCScriptNode *node, asCByteCode *bc);
	void CompileContinueStatement(asCScriptNode *node, asCByteCode *bc);
	void CompileReturnStatement(asCScriptNode *node, asCByteCode *bc);
	void CompileExpressionStatement(asCScriptNode *node, asCByteCode *bc);

	// Expressions
	int  CompileAssignment(asCScriptNode *expr, asCExprContext *out, bool bIsStatementRoot = false);
	int  CompileCondition(asCScriptNode *expr, asCExprContext *out, bool bIsStatementRoot = false);
	int  CompileExpression(asCScriptNode *expr, asCExprContext *out, bool bIsStatementRoot = false);
	int  CompilePostFixExpression(asCArray<asCScriptNode *> *postfix, asCExprContext *out, bool bIsStatementRoot);
	int  CompileExpressionTerm(asCScriptNode *node, asCExprContext *out, bool bIsStatementRoot = false);
	int  CompileExpressionPreOp(asCScriptNode *node, asCExprContext *out, bool bIsStatementRoot = false);
	int  CompileExpressionPostOp(asCScriptNode *node, asCExprContext *out, bool bIsStatementRoot = false);
	int  CompileExpressionValue(asCScriptNode *node, asCExprContext *out);
	int  CompileFunctionCall(asCScriptNode *node, asCExprContext *out, asCObjectType *objectType, bool objIsConst, const asCString &scope = "");
	int  CompileConstructCall(asCScriptNode *node, asCExprContext *out);
	int  CompileConversion(asCScriptNode *node, asCExprContext *out);
	int  CompileOperator(asCScriptNode *node, asCExprContext *l, asCExprContext *r, asCExprContext *out, eTokenType opToken = ttUnrecognizedToken, bool leftToRight = true);
	void CompileOperatorOnHandles(asCScriptNode *node, asCExprContext *l, asCExprContext *r, asCExprContext *out, eTokenType opToken = ttUnrecognizedToken);
	void CompileMathOperator(asCScriptNode *node, asCExprContext *l, asCExprContext *r, asCExprContext *out, eTokenType opToken = ttUnrecognizedToken);
	void CompileBitwiseOperator(asCScriptNode *node, asCExprContext *l, asCExprContext *r, asCExprContext *out, eTokenType opToken = ttUnrecognizedToken);
	void CompileComparisonOperator(asCScriptNode *node, asCExprContext *l, asCExprContext *r, asCExprContext *out, eTokenType opToken = ttUnrecognizedToken);
	void CompileBooleanOperator(asCScriptNode *node, asCExprContext *l, asCExprContext *r, asCExprContext *out, eTokenType opToken = ttUnrecognizedToken);
	bool CompileOverloadedDualOperator(asCScriptNode *node, asCExprContext *l, asCExprContext *r, bool leftToRight, asCExprContext *out, bool isHandle = false, eTokenType opToken = ttUnrecognizedToken);
	int  CompileOverloadedDualOperator2(asCScriptNode *node, const char *methodName, asCExprContext *l, asCExprContext *r, bool leftToRight, asCExprContext *out, bool specificReturn = false, const asCDataType &returnType = asCDataType::CreatePrimitive(ttVoid, false));

	void CompileInitList(asCExprValue *var, asCScriptNode *node, asCByteCode *bc, int isVarGlobOrMem);
	int  CompileInitListElement(asSListPatternNode *&patternNode, asCScriptNode *&valueNode, int bufferTypeId, short bufferVar, asUINT &bufferSize, asCByteCode &byteCode, int &elementsInSubList);
	int  CompileAnonymousInitList(asCScriptNode *listNode, asCExprContext *ctx, const asCDataType &dt);

	int  CallDefaultConstructor(const asCDataType &type, int offset, bool isObjectOnHeap, asCByteCode *bc, asCScriptNode *node, int isVarGlobOrMem = 0, bool derefDest = false);
	int  CallCopyConstructor(asCDataType &type, int offset, bool isObjectOnHeap, asCByteCode *bc, asCExprContext *arg, asCScriptNode *node, bool isGlobalVar = false, bool derefDestination = false);
	void CallDestructor(asCDataType &type, int offset, bool isObjectOnHeap, asCByteCode *bc, bool alwaysClearHandles = true);
	int  CompileArgumentList(asCScriptNode *node, asCArray<asCExprContext *> &args, asCArray<asSNamedArgument> &namedArgs);
	int  CompileDefaultAndNamedArgs(asCScriptNode *node, asCArray<asCExprContext*> &args, int funcId, asCObjectType *type, asCArray<asSNamedArgument> *namedArgs = 0);
	asUINT MatchFunctions(asCArray<int> &funcs, asCArray<asCExprContext*> &args, asCScriptNode *node, const char *name, asCArray<asSNamedArgument> *namedArgs = NULL, asCObjectType *objectType = NULL, bool isConstMethod = false, bool silent = false, bool allowObjectConstruct = true, const asCString &scope = "", bool bConstDueToTemporary = false);
	int  CompileVariableAccess(const asCString &name, const asCString &scope, asCExprContext *ctx, asCScriptNode *errNode, bool isOptional = false, bool noFunction = false, bool noGlobal = false, asCObjectType *objType = 0);
	void CompileMemberInitialization(asCByteCode *bc, bool onlyDefaults);
	void CompileDefaultStatements(asCByteCode *bc);
	bool CompileAutoType(asCDataType &autoType, asCExprContext &compiledCtx, asCScriptNode *exprNode, asCScriptNode *errNode, bool bAllowRef = false);
	bool CompileInitialization(asCScriptNode *node, asCByteCode *bc, const asCDataType &type, asCScriptNode *errNode, int offset, asQWORD *constantValue, int isVarGlobOrMem, asCExprContext *preCompiled = 0, bool bCanAssumeZeroed = true);
	void CompileInitAsCopy(asCDataType &type, int offset, asCByteCode *bc, asCExprContext *arg, asCScriptNode *node, bool derefDestination);

	// Helper functions
	void ConvertToPostFix(asCScriptNode *expr, asCArray<asCScriptNode *> &postfix);
	void ProcessPropertyGetAccessor(asCExprContext *ctx, asCScriptNode *node);
	int  ProcessPropertySetAccessor(asCExprContext *ctx, asCExprContext *arg, asCScriptNode *node);
	int  ProcessPropertyGetSetAccessor(asCExprContext *ctx, asCExprContext *lctx, asCExprContext *rctx, eTokenType op, asCScriptNode *errNode);
	int  FindPropertyAccessor(const asCString &name, asCExprContext *ctx, asCScriptNode *node, asSNameSpace *ns, bool isThisAccess = false);
	int  FindPropertyAccessor(const asCString &name, asCExprContext *ctx, asCExprContext *arg, asCScriptNode *node, asSNameSpace *ns, bool isThisAccess = false);
	void PrepareTemporaryVariable(asCScriptNode *node, asCExprContext *ctx, bool forceOnHeap = false);
	void PrepareOperand(asCExprContext *ctx, asCScriptNode *node);
	void PrepareForAssignment(asCDataType *lvalue, asCExprContext *rvalue, asCScriptNode *node, bool toTemporary, asCExprContext *lvalueExpr = 0, bool allowNarrowing = false);
	int  PerformAssignment(asCExprValue *lvalue, asCExprValue *rvalue, asCByteCode *bc, asCScriptNode *node);
	bool IsVariableInitialized(asCExprValue *type, asCScriptNode *node);
	void Dereference(asCExprContext *ctx, bool generateCode);
	bool CompileRefCast(asCExprContext *ctx, const asCDataType &to, bool isExplicit, asCScriptNode *node, bool generateCode = true);
	asUINT MatchArgument(asCArray<int> &funcs, asCArray<asSOverloadCandidate> &matches, const asCExprContext *argExpr, int paramNum, bool allowObjectConstruct = true, bool resolvedArgument = false);
	int  MatchArgument(asCScriptFunction *desc, const asCExprContext *argExpr, int paramNum, bool allowObjectConstruct = true);
	void PerformFunctionCall(int funcId, asCExprContext *out, bool isConstructor = false, asCArray<asCExprContext*> *args = 0, asCObjectType *objTypeForConstruct = 0, bool useVariable = false, int varOffset = 0, int funcPtrVar = 0, bool virtualCall = true, asCDataType* overrideReturnType = nullptr);
	void MoveArgsToStack(int funcId, asCByteCode *bc, asCArray<asCExprContext *> &args, bool addOneToOffset);
	void MakeFunctionCall(asCExprContext *ctx, int funcId, asCObjectType *objectType, asCArray<asCExprContext*> &args, asCScriptNode *node, bool useVariable = false, int stackOffset = 0, int funcPtrVar = 0, bool virtualCall = true);
	void PrepareFunctionCall(int funcId, asCByteCode *bc, asCArray<asCExprContext *> &args);
	void AfterFunctionCall(int funcId, asCArray<asCExprContext*> &args, asCExprContext *ctx, bool deferAll);
	void ProcessDeferredParams(asCExprContext *ctx);
	int  PrepareArgument(asCDataType *paramType, asCExprContext *ctx, asCScriptNode *node, bool isFunction = false, int refType = 0, bool isMakingCopy = false, bool allowNarrowing = false);
	void PrepareArgument2(asCExprContext *ctx, asCExprContext *arg, asCDataType *paramType, bool isFunction = false, int refType = 0, bool isMakingCopy = false, bool allowNarrowing = false);
	bool IsLValue(asCExprValue &type);
	int  DoAssignment(asCExprContext *out, asCExprContext *lctx, asCExprContext *rctx, asCScriptNode *lexpr, asCScriptNode *rexpr, eTokenType op, asCScriptNode *opNode);
	void MergeExprBytecode(asCExprContext *before, asCExprContext *after);
	void MergeExprBytecodeAndType(asCExprContext *before, asCExprContext *after);
	void FilterConst(asCArray<int> &funcs, bool removeConst = true);
	void ConvertToVariable(asCExprContext *ctx);
	void ConvertToVariableNotIn(asCExprContext *ctx, asCExprContext *exclude);
	void ConvertToTempVariable(asCExprContext *ctx);
	void ConvertToTempVariableNotIn(asCExprContext *ctx, asCExprContext *exclude);
	void ConvertToReference(asCExprContext *ctx);
	void PushVariableOnStack(asCExprContext *ctx, bool asReference);
	void DestroyVariables(asCByteCode *bc);
	asSNameSpace *DetermineNameSpace(const asCString &scope);
	int  SetupParametersAndReturnVariable(asCArray<asCString> &parameterNames, asCScriptNode *func);
	bool AreAllTemplateSubtypesCovariant(const asCDataType &to, const asCDataType &from) const;
	bool AreTemplateTypesWithFailedCovariance(const asCDataType &to, const asCDataType &from) const;
	asCDataType ApplyDeterminesOutputType(const asCDataType& original, const asCDataType& determinedBy, bool bDoTypeCheck = true) const;
	void ShowTypoSuggestionForVariableAccess(asCScriptNode* node, const asCString& name, asCObjectType* suggestObjectType = nullptr, asSNameSpace* suggestNameSpace = nullptr, bool bSuggestLocals = false);

	void DetermineSingleFunc(asCExprContext *ctx, asCScriptNode *node);

	// Returns the cost of the conversion (the sum of the EConvCost performed)
	asUINT ImplicitConversion(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode = true, bool allowObjectConstruct = true);
	asUINT ImplicitConvPrimitiveToPrimitive(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode = true);
	asUINT ImplicitConvObjectToPrimitive(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode = true);
	asUINT ImplicitConvPrimitiveToObject(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode = true, bool allowObjectConstruct = true);
	asUINT ImplicitConvObjectToObject(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode = true, bool allowObjectConstruct = true);
	asUINT ImplicitConvObjectRef(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode);
	asUINT ImplicitConvObjectValue(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode);
	void   ImplicitConversionConstant(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool showWarnings = true);
	void   ImplicitConvObjectToBestMathType(asCExprContext *ctx, asCScriptNode *node);
	asUINT ImplicitConvLambdaToFunc(asCExprContext *ctx, const asCDataType &to, asCScriptNode *node, EImplicitConv convType, bool generateCode = true);

	void LineInstr(asCByteCode *bc, size_t pos);

	asUINT ProcessStringConstant(asCString &str, asCScriptNode *node, bool processEscapeSequences = true);
	void ProcessHeredocStringConstant(asCString &str, asCScriptNode *node);
	int  GetPrecedence(asCScriptNode *op);
	void Error(const asCString &msg, asCScriptNode *node);
	void Warning(const asCString &msg, asCScriptNode *node);
	void Information(const asCString &msg, asCScriptNode *node);
	void PrintMatchingFuncs(asCArray<int> &funcs, asCScriptNode *node, asCObjectType *inType = 0);
	void AddVariableScope(bool isBreakScope = false, bool isContinueScope = false);
	void RemoveVariableScope();
	void FinalizeFunction();

	asCByteCode byteCode;

	bool hasCompileErrors;

	int nextLabel;
	int numLambdas;

	asCVariableScope  *variables;
	asCBuilder        *builder;
	asCScriptEngine   *engine;
	asCScriptCode     *script;
	asCScriptFunction *outFunc;

	bool                        m_isConstructor;
	bool                        m_isInitDefaults;
	bool                        m_isDestructor;
	bool                        m_isConstructorCalled;
	sClassDeclaration          *m_classDecl;
	sGlobalVariableDescription *m_globalVar;

	asCArray<int> breakLabels;
	asCArray<int> continueLabels;

	int AllocateVariable(const asCDataType &type, bool isTemporary, bool forceOnHeap = false, bool bAllowReference = false);
	int AllocateVariableNotIn(const asCDataType &type, bool isTemporary, bool forceOnHeap, asCExprContext *ctx);
	int GetVariableOffset(int varIndex);
	int GetVariableSlot(int varOffset);
	void DeallocateVariable(int pos);
	void ReleaseTemporaryVariable(asCExprValue &t, asCByteCode *bc);
	void ReleaseTemporaryVariable(int offset, asCByteCode *bc);
	bool IsVariableOnHeap(int offset);
	bool IsVariableOnHeapOrReference(int offset);

	void MarkVariableInitialized(int stackOffset);

	// This ordered array indicates the type of each variable
	asCArray<asCDataType> variableAllocations;

	// This ordered array indicates which variables are temporaries or not
	asCArray<bool>        variableIsTemporary;

	// This unordered array gives the offsets of all temporary variables, whether currently allocated or not
	asCArray<int>         tempVariableOffsets;

	// This ordered array indicated if the variable is on the heap or not
	asCArray<bool>        variableIsOnHeap;

	// This unordered array gives the indexes of the currently unused variables
	asCArray<int>         freeVariables;

	// This array holds the offsets of the currently allocated temporary variables
	asCArray<int>         tempVariables; 

	// This array holds the indices of variables that must not be used in an allocation
	asCArray<int>         reservedVariables;

	// This array holds the string constants that were allocated during the compilation, 
	// so they can be released upon completion, whether the compilation was successful or not.
	asCArray<void*>       usedStringConstants;

	typedef TArray<sVariable*, TInlineAllocator<16>> FBlockInitializedVariables;
	bool bRecordInitializedVariables = false;
	FBlockInitializedVariables BlockInitializedVariables;

	void UndoInitializedVariables(const FBlockInitializedVariables& Vars);
	void ApplyInitializedVariables(const FBlockInitializedVariables& Vars);
	void MergeInitializedVariablesBlocks(const FBlockInitializedVariables& BranchOne, const FBlockInitializedVariables& BranchTwo);

	bool isCompilingDefaultArg;
	bool isProcessingDeferredParams;
	int  noCodeOutput;
	bool allowEditPropertyAccess;
	bool warnOnWorldContextUsage = false;
	int compilingMemberInitializationOffset = -1;
	int counterHoldDeferredParams = 0;

	bool bDelayTempVariableDeallocs = false;
	asCArray<int> DelayedTempVariableDeallocs;

	bool bForceAllCodeIsConst = false;
	asCString ForceConstFunction;

	asCObjectType* ExternalThisType = nullptr;
	int32 ExternalThisOffset = 0;
};

END_AS_NAMESPACE

#endif // AS_NO_COMPILER

#endif
