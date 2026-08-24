#pragma once

#include <cstddef>

namespace gore_as_capture::v1::instrumentation::layout_v23300 {

// Shipping-layout witness compiled from the pinned UNREANGEL donor. These constants are not
// sufficient evidence for target reads on their own: only members explicitly listed in
// target_confirmed have matching instruction witnesses in BuildID 24539464.
namespace donor {
constexpr std::size_t type_info_alignment = 0x08;
constexpr std::size_t string_bytes = 24;
constexpr std::size_t array_bytes = 32;
constexpr std::size_t function_pointer_descriptor_bytes = 40;
constexpr std::size_t function_pointer_descriptor_flag = 32;
constexpr std::size_t function_caller_descriptor_bytes = 16;
constexpr std::size_t function_caller_descriptor_type = 8;

constexpr std::size_t script_function_bytes = 0x188;
constexpr std::size_t script_function_exposed_type = 0x0e8;
constexpr std::size_t script_function_hidden_argument_default = 0x118;
constexpr std::size_t script_function_system_interface = 0x158;
constexpr std::size_t script_function_hidden_argument_index = 0x16c;
constexpr std::size_t script_function_output_type_argument_index = 0x16d;
constexpr std::size_t script_function_id = 0x170;
constexpr std::size_t script_function_type = 0x174;
constexpr std::size_t script_function_traits = 0x178;
constexpr std::size_t script_function_compile_out_type = 0x17c;

constexpr std::size_t system_interface_bytes = 0x48;
constexpr std::size_t system_interface_function = 0x00;
constexpr std::size_t system_interface_method = 0x08;
constexpr std::size_t system_interface_base_offset = 0x20;
constexpr std::size_t system_interface_call_convention = 0x24;
constexpr std::size_t system_interface_first_param_metadata = 0x30;
constexpr std::size_t system_interface_caller = 0x38;

constexpr std::size_t object_property_bytes = 0x50;
constexpr std::size_t object_property_byte_offset = 0x30;
constexpr std::size_t object_property_access_mask = 0x34;
constexpr std::size_t object_property_composite_offset = 0x38;
constexpr std::size_t object_property_composite_indirect = 0x3c;
constexpr std::size_t object_property_private = 0x3d;
constexpr std::size_t object_property_protected = 0x3e;
constexpr std::size_t object_property_app_bind = 0x3f;
constexpr std::size_t object_property_exposed_type = 0x40;

constexpr std::size_t global_property_bytes = 0x70;
constexpr std::size_t global_property_id = 0x30;
constexpr std::size_t global_property_real_address = 0x48;
constexpr std::size_t global_property_storage = 0x58;
constexpr std::size_t global_property_pure_constant = 0x6d;

constexpr std::size_t object_type_bytes = 0x2a8;
constexpr std::size_t object_type_properties = 0x090;
constexpr std::size_t object_type_interfaces = 0x160;
constexpr std::size_t object_type_interface_vft_offsets = 0x180;
constexpr std::size_t object_type_base = 0x1a0;
constexpr std::size_t object_type_shadow = 0x1c8;
constexpr std::size_t object_type_accept_value_subtype = 0x2a0;
constexpr std::size_t object_type_accept_reference_subtype = 0x2a1;
constexpr std::size_t object_type_implicit_constructors = 0x2a2;
constexpr std::size_t object_type_invalid_generated = 0x2a3;

constexpr std::size_t script_engine_bytes = 0x1638;
constexpr std::size_t engine_registered_object_types = 0x568;
constexpr std::size_t engine_registered_typedefs = 0x588;
constexpr std::size_t engine_registered_enums = 0x5a8;
constexpr std::size_t engine_registered_global_properties = 0x5c8;
constexpr std::size_t engine_registered_global_functions = 0x620;
constexpr std::size_t engine_registered_funcdefs = 0x678;
constexpr std::size_t engine_string_factory = 0x6b8;
constexpr std::size_t engine_global_properties = 0x7e0;
constexpr std::size_t engine_script_functions = 0x858;
constexpr std::size_t engine_default_access_mask = 0x13e0;
constexpr std::size_t engine_default_namespace = 0x13e8;
}  // namespace donor

namespace target_confirmed {
// Exact BuildID 24539464 instruction witnesses:
//   RVA 0x479a318/0x479aa98/0x479abfa: allocate 0x2d8 bytes for the object type;
//                  each path immediately calls the same constructor at RVA 0x46bc920.
//   RVA 0x46bc920: the object-type constructor writes alignment 8 at +08h, installs
//                  vtable RVA 0x81f3d90, initializes the interface array at +190h,
//                  interface-VFT-offset array at +1b0h, base at +1d0h, shadow at
//                  +1f8h, and writes 0x00000101 at +2d0h (the four boolean bytes).
//   Object-type vtable RVA 0x81f3d90: slot 8/RVA 0x34c4bb0 returns [rcx+1d0h],
//                  slot 17/RVA 0x431deb0 returns [rcx+198h], and slot 18/
//                  RVA 0x4755fe0 indexes the pointer array at [rcx+190h].
//   RVA 0x4754ed0: mov eax,[rcx+174h]; ret
//   RVA 0x4755c40: mov eax,[rcx+170h]; ret
//   RVA 0x476b9e0: mov eax,[rcx+178h]; shr eax,9; and al,1; ret
//   RVA 0x4755c60: loads [rcx+158h].
//   RVA 0x4754fa1: lea rcx,[r14+0e8h].
//   RVAs 0x47530b1/0x47700a8/0x4770321: read signed byte [function+16ch].
//   RVA 0x4791835: lea rdx,[function+118h].
//   RVA 0x468a068: writes byte [rax+16dh].
//   RVAs 0x467dd8e/0x467dde4/0x5722fef: write compile-out values 1/2/3
//                  to [function+17ch].
//   RVAs 0x4688c44/0x4688c4b and 0x4688c84/0x4688c8b: write first-param
//                  metadata values 1/2 to [system-interface+30h].
//   RVA 0x468a140: loads target engine [engine+8b0h], then writes global-property
//                  storage [r8+58h] and pure-constant byte [r8+6dh].
//   RVA 0x47557d0 (public engine slot 16/GetGlobalPropertyByIndex): bounds-checks
//                  [engine+630h], indexes the pointer array at [engine+628h], and returns
//                  the registered raw storage address from [property+48h].
//   RVA 0x476b9f0: reads object-type property array [rcx+90h] and property byte
//                  offset [rdx+30h].
//   RVAs 0x4799452..0x4799465 and 0x47994fd..0x479951f: initialize/write the
//                  complete object-property tail [property+30h..40h].
//   RVA 0x47a4aa0: swaps the default access mask at [engine+1558h].
//   RVA 0x4754270: returns the asCString payload reached through [engine+1560h].
//   RVA 0x1002520: shared zero-return body for BeginConfigGroup, EndConfigGroup and
//                  RemoveConfigGroup (engine vtable slots 39..41).
constexpr std::size_t script_function_exposed_type = donor::script_function_exposed_type;
constexpr std::size_t script_function_hidden_argument_default =
    donor::script_function_hidden_argument_default;
constexpr std::size_t script_function_system_interface =
    donor::script_function_system_interface;
constexpr std::size_t script_function_hidden_argument_index =
    donor::script_function_hidden_argument_index;
constexpr std::size_t script_function_output_type_argument_index =
    donor::script_function_output_type_argument_index;
constexpr std::size_t script_function_id = donor::script_function_id;
constexpr std::size_t script_function_type = donor::script_function_type;
constexpr std::size_t script_function_traits = donor::script_function_traits;
constexpr std::size_t script_function_compile_out_type =
    donor::script_function_compile_out_type;
constexpr std::size_t system_interface_first_param_metadata =
    donor::system_interface_first_param_metadata;
constexpr std::size_t global_property_storage = donor::global_property_storage;
constexpr std::size_t global_property_pure_constant = donor::global_property_pure_constant;
constexpr std::size_t global_property_real_address = donor::global_property_real_address;
constexpr std::size_t object_type_properties = donor::object_type_properties;
constexpr std::size_t target_object_type_bytes = 0x2d8;
constexpr std::size_t object_type_alignment = 0x008;
constexpr std::size_t object_type_interfaces = 0x190;
constexpr std::size_t object_type_interface_vft_offsets = 0x1b0;
constexpr std::size_t object_type_base = 0x1d0;
constexpr std::size_t object_type_shadow = 0x1f8;
constexpr std::size_t object_type_accept_value_subtype = 0x2d0;
constexpr std::size_t object_type_accept_reference_subtype = 0x2d1;
constexpr std::size_t object_type_implicit_constructors = 0x2d2;
constexpr std::size_t object_type_invalid_generated = 0x2d3;
constexpr std::size_t object_property_byte_offset = donor::object_property_byte_offset;
constexpr std::size_t object_property_access_mask = donor::object_property_access_mask;
constexpr std::size_t object_property_composite_offset =
    donor::object_property_composite_offset;
constexpr std::size_t object_property_composite_indirect =
    donor::object_property_composite_indirect;
constexpr std::size_t object_property_private = donor::object_property_private;
constexpr std::size_t object_property_protected = donor::object_property_protected;
constexpr std::size_t object_property_app_bind = donor::object_property_app_bind;
constexpr std::size_t object_property_exposed_type = donor::object_property_exposed_type;
constexpr std::size_t target_engine_global_properties = 0x8b0;
constexpr std::size_t target_engine_registered_global_properties = 0x628;
constexpr std::size_t target_engine_default_access_mask = 0x1558;
constexpr std::size_t target_engine_default_namespace = 0x1560;
}  // namespace target_confirmed

}  // namespace gore_as_capture::v1::instrumentation::layout_v23300
