#include "as_callfunc.h"
#include "as_array.h"
#include "as_objecttype.h"
#include "as_property.h"
#include "as_scriptengine.h"
#include "as_scriptfunction.h"
#include "as_string.h"
#include "target_layout.hpp"

#include <cstddef>
#include <cstdint>
#include <iostream>

namespace {

template <typename Type, typename Member>
std::size_t member_offset(Member Type::*member) noexcept {
  const auto object = reinterpret_cast<const Type*>(0x1000);
  const auto field = &(object->*member);
  return reinterpret_cast<std::uintptr_t>(field) -
         reinterpret_cast<std::uintptr_t>(object);
}

namespace expected = gore_as_capture::v1::instrumentation::layout_v23300::donor;

bool expect_equal(
    const char* const name,
    const std::size_t actual,
    const std::size_t wanted) noexcept {
  if (actual == wanted) return true;
  std::cerr << name << ": expected " << wanted << ", got " << actual << '\n';
  return false;
}

#define GORE_AS_EXPECT_SIZE(type, expected_value) \
  ok = expect_equal("sizeof(" #type ")", sizeof(type), expected_value) && ok
#define GORE_AS_EXPECT_LAYOUT(type, member, expected_value)                       \
  ok = expect_equal(                                                             \
           #type "." #member, member_offset(&type::member), expected_value) &&  \
       ok

}  // namespace

int main() {
  bool ok = true;
  GORE_AS_EXPECT_LAYOUT(asCTypeInfo, alignment, expected::type_info_alignment);
  GORE_AS_EXPECT_SIZE(asCString, expected::string_bytes);
  GORE_AS_EXPECT_SIZE(asCArray<int>, expected::array_bytes);
  GORE_AS_EXPECT_SIZE(asSFuncPtr, expected::function_pointer_descriptor_bytes);
  GORE_AS_EXPECT_LAYOUT(
      asSFuncPtr, flag, expected::function_pointer_descriptor_flag);
  GORE_AS_EXPECT_SIZE(asFunctionCaller, expected::function_caller_descriptor_bytes);
  GORE_AS_EXPECT_LAYOUT(
      asFunctionCaller, type, expected::function_caller_descriptor_type);

  GORE_AS_EXPECT_SIZE(asCScriptFunction, expected::script_function_bytes);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptFunction, exposedType, expected::script_function_exposed_type);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptFunction,
      hiddenArgumentDefault,
      expected::script_function_hidden_argument_default);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptFunction, sysFuncIntf, expected::script_function_system_interface);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptFunction,
      hiddenArgumentIndex,
      expected::script_function_hidden_argument_index);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptFunction,
      determinesOutputTypeArgumentIndex,
      expected::script_function_output_type_argument_index);
  GORE_AS_EXPECT_LAYOUT(asCScriptFunction, id, expected::script_function_id);
  GORE_AS_EXPECT_LAYOUT(asCScriptFunction, funcType, expected::script_function_type);
  GORE_AS_EXPECT_LAYOUT(asCScriptFunction, traits, expected::script_function_traits);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptFunction, compileOutType, expected::script_function_compile_out_type);

  GORE_AS_EXPECT_SIZE(asSSystemFunctionInterface, expected::system_interface_bytes);
  GORE_AS_EXPECT_LAYOUT(
      asSSystemFunctionInterface, func, expected::system_interface_function);
  GORE_AS_EXPECT_LAYOUT(
      asSSystemFunctionInterface, method, expected::system_interface_method);
  GORE_AS_EXPECT_LAYOUT(
      asSSystemFunctionInterface, baseOffset, expected::system_interface_base_offset);
  GORE_AS_EXPECT_LAYOUT(
      asSSystemFunctionInterface,
      callConv,
      expected::system_interface_call_convention);
  GORE_AS_EXPECT_LAYOUT(
      asSSystemFunctionInterface,
      passFirstParamMetaData,
      expected::system_interface_first_param_metadata);
  GORE_AS_EXPECT_LAYOUT(
      asSSystemFunctionInterface, caller, expected::system_interface_caller);

  GORE_AS_EXPECT_SIZE(asCObjectProperty, expected::object_property_bytes);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectProperty, byteOffset, expected::object_property_byte_offset);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectProperty, accessMask, expected::object_property_access_mask);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectProperty, compositeOffset, expected::object_property_composite_offset);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectProperty,
      isCompositeIndirect,
      expected::object_property_composite_indirect);
  GORE_AS_EXPECT_LAYOUT(asCObjectProperty, isPrivate, expected::object_property_private);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectProperty, isProtected, expected::object_property_protected);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectProperty, isAppBindProperty, expected::object_property_app_bind);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectProperty, exposedType, expected::object_property_exposed_type);

  GORE_AS_EXPECT_SIZE(asCGlobalProperty, expected::global_property_bytes);
  GORE_AS_EXPECT_LAYOUT(asCGlobalProperty, id, expected::global_property_id);
  GORE_AS_EXPECT_LAYOUT(
      asCGlobalProperty, realAddress, expected::global_property_real_address);
  GORE_AS_EXPECT_LAYOUT(asCGlobalProperty, storage, expected::global_property_storage);
  GORE_AS_EXPECT_LAYOUT(
      asCGlobalProperty, isPureConstant, expected::global_property_pure_constant);

  GORE_AS_EXPECT_SIZE(asCObjectType, expected::object_type_bytes);
  GORE_AS_EXPECT_LAYOUT(asCObjectType, properties, expected::object_type_properties);
  GORE_AS_EXPECT_LAYOUT(asCObjectType, interfaces, expected::object_type_interfaces);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectType, interfaceVFTOffsets, expected::object_type_interface_vft_offsets);
  GORE_AS_EXPECT_LAYOUT(asCObjectType, derivedFrom, expected::object_type_base);
  GORE_AS_EXPECT_LAYOUT(asCObjectType, shadowType, expected::object_type_shadow);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectType, acceptValueSubType, expected::object_type_accept_value_subtype);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectType, acceptRefSubType, expected::object_type_accept_reference_subtype);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectType,
      hasImplicitConstructors,
      expected::object_type_implicit_constructors);
  GORE_AS_EXPECT_LAYOUT(
      asCObjectType, isInvalidGeneratedType, expected::object_type_invalid_generated);

  GORE_AS_EXPECT_SIZE(asCScriptEngine, expected::script_engine_bytes);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, registeredObjTypes, expected::engine_registered_object_types);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, registeredTypeDefs, expected::engine_registered_typedefs);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, registeredEnums, expected::engine_registered_enums);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine,
      registeredGlobalProps,
      expected::engine_registered_global_properties);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine,
      registeredGlobalFuncs,
      expected::engine_registered_global_functions);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, registeredFuncDefs, expected::engine_registered_funcdefs);
  GORE_AS_EXPECT_LAYOUT(asCScriptEngine, stringFactory, expected::engine_string_factory);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, globalProperties, expected::engine_global_properties);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, scriptFunctions, expected::engine_script_functions);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, defaultAccessMask, expected::engine_default_access_mask);
  GORE_AS_EXPECT_LAYOUT(
      asCScriptEngine, defaultNamespace, expected::engine_default_namespace);

  if (ok) std::cout << "pinned donor Shipping layout witness passed\n";
  return ok ? 0 : 1;
}
