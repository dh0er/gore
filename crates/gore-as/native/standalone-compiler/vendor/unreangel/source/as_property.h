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
// as_property.h
//
// A class for storing object property information
//



#ifndef AS_PROPERTY_H
#define AS_PROPERTY_H

#include "as_string.h"
#include "as_datatype.h"
#include "as_atomic.h"
#include "as_scriptfunction.h"
#include "as_symboltable.h"

BEGIN_AS_NAMESPACE

struct asSNameSpace;

class asCObjectProperty
{
public:
	asCObjectProperty()
		: byteOffset(-1), accessMask(0xFFFFFFFF), compositeOffset(0), isCompositeIndirect(false), isPrivate(false),
		isProtected(false), isAppBindProperty(false), exposedType(255)
	{}

	asCObjectProperty(const asCObjectProperty &o)
		: name(o.name), type(o.type), byteOffset(o.byteOffset), accessMask(o.accessMask), compositeOffset(o.compositeOffset),
		isCompositeIndirect(o.isCompositeIndirect), isPrivate(o.isPrivate), isProtected(o.isProtected), isAppBindProperty(o.isAppBindProperty),
		exposedType(o.exposedType)
#if WITH_EDITOR
		, DeprecationMessage(o.DeprecationMessage)
		, isDeprecated(o.isDeprecated)
		, isEditorOnly(o.isEditorOnly)
#endif
	{}

	asCString   name;
	asCDataType type;
	int         byteOffset = -1;
	asDWORD     accessMask;
	int         compositeOffset;
	bool        isCompositeIndirect;
	bool        isPrivate;
	bool        isProtected;
	bool        isAppBindProperty;
	asUINT		exposedType;
	asSAccessSpecifier* accessSpecifier = nullptr;

#if WITH_EDITOR
	asCString DeprecationMessage;
	bool isDeprecated = false;
	bool isEditorOnly = false;
#endif

	inline const char* GetName() const
	{
		return name.AddressOf();
	}

	inline bool Editable() const
	{	
		return exposedType & (1 << 2);
	}

	inline bool Readable() const
	{
		return exposedType & (1 << 0);
	}

	inline bool Writeable() const
	{
		return exposedType & (1 << 1);
	}

	static asUINT GenerateExposedType(bool editable, bool readable, bool writeable)
	{
		asUINT accessMask = 0; // No access to start with

		if (readable)
			accessMask |= 1 << 0;

		if (writeable)
			accessMask |= 1 << 1;

		if (editable)
			accessMask |= 1 << 2;

		return accessMask;
	}
};

class ANGELSCRIPTCODE_API asCGlobalProperty
{
public:
	asCGlobalProperty();
	~asCGlobalProperty();

	void AddRef();
	void Release();
	void DestroyInternal();

	void *GetAddressOfValue();
	void  AllocateMemory();
	void  SetRegisteredAddress(void *p);
	void *GetRegisteredAddress() const;

	asCString          name;
	asCDataType        type;
	asUINT             id;
	asSNameSpace      *nameSpace;
	asCModule         *module;

	void SetInitFunc(asCScriptFunction *initFunc);
	asCScriptFunction *GetInitFunc();

//protected:
	// This is only stored for registered properties, and keeps the pointer given by the application
	void       *realAddress;
	void       *memory;
	asQWORD     storage;

	asCScriptFunction *initFunc;

	// The global property structure is reference counted, so that the
	// engine can keep track of how many references to the property there are.
	asCAtomic refCount;
	bool        memoryAllocated;
	bool        isPureConstant = false;
	bool        isDefaultInit = false;
#if WITH_EDITOR
	bool isEditorOnly = false;
#endif
};

class asCCompGlobPropType : public asIFilter
{
public:
	const asCDataType &m_type;

	asCCompGlobPropType(const asCDataType &type) : m_type(type) {}

	bool operator()(const void *p) const
	{
		const asCGlobalProperty* prop = reinterpret_cast<const asCGlobalProperty*>(p);
		return prop->type == m_type;
	}

private:
	// The assignment operator is required for MSVC9, otherwise it will complain that it is not possible to auto generate the operator
	asCCompGlobPropType &operator=(const asCCompGlobPropType &) {return *this;}
};

END_AS_NAMESPACE

#endif
