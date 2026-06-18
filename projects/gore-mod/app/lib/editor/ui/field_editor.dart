import 'package:flutter/material.dart';
import '../../catalog/domain/field_schema.dart';
import '../../catalog/domain/item_entry.dart';
import '../domain/field_validator.dart';
import '../domain/override_entry.dart';

/// Per-item field editor: renders each [FieldSchema] as a typed input.
/// Calls [onOverrideChanged] with a valid [OverrideEntry] on every committed
/// valid change — including values that equal the placeholder default (e.g.
/// `0`), since the model carries no real CDO default to compare against.
/// Removing an override is done explicitly from the OverridesPanel.
class FieldEditor extends StatefulWidget {
  const FieldEditor({
    super.key,
    required this.item,
    required this.pendingOverrides,
    required this.onOverrideChanged,
  });

  final CatalogItem item;

  /// Existing pending overrides for this item (keyed by field name).
  final Map<String, OverrideEntry> pendingOverrides;
  final void Function(OverrideEntry) onOverrideChanged;

  @override
  State<FieldEditor> createState() => _FieldEditorState();
}

class _FieldEditorState extends State<FieldEditor> {
  // Text controllers keyed by field name.
  final Map<String, TextEditingController> _controllers = {};
  final Map<String, String?> _errors = {};

  @override
  void initState() {
    super.initState();
    _rebuildControllers();
  }

  @override
  void didUpdateWidget(covariant FieldEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.item.id != widget.item.id) {
      for (final c in _controllers.values) {
        c.dispose();
      }
      _controllers.clear();
      _errors.clear();
      _rebuildControllers();
      return;
    }
    // Same item, but the pending overrides may have changed externally — the
    // right-hand OverridesPanel can remove a single override or Clear all. The
    // controllers would otherwise keep showing the old overridden value that is
    // no longer pending (and won't be exported). Resync each field's text to
    // its current pending value, or back to the default when it was cleared.
    // Skip fields the user is mid-editing with a validation error so we don't
    // clobber in-progress input — and skip fields whose text already represents
    // the desired value, so we never reformat live typing (e.g. "3" -> "3.0",
    // which would drop a half-typed ".5" and re-select the field).
    for (final field in widget.item.fields) {
      final controller = _controllers[field.name];
      if (controller == null || _errors[field.name] != null) continue;
      final pending = widget.pendingOverrides[field.name];
      if (_controllerRepresents(field, controller.text, pending)) continue;
      controller.text =
          pending != null ? _pendingText(field, pending) : _defaultText(field);
    }
  }

  /// Whether [text] already encodes the value the field should show — the
  /// pending override's value, or the placeholder default when none is pending.
  /// Compared by parsed value, not string, so equivalent spellings ("3" vs
  /// "3.0") and in-progress input are left untouched.
  bool _controllerRepresents(
    FieldSchema field,
    String text,
    OverrideEntry? pending,
  ) {
    if (validateField(field, text) != null) return false;
    final current = parsedValue(field, text);
    final expected =
        pending?.newValue ?? parsedValue(field, _defaultText(field));
    return current == expected;
  }

  @override
  void dispose() {
    for (final c in _controllers.values) {
      c.dispose();
    }
    super.dispose();
  }

  void _rebuildControllers() {
    for (final field in widget.item.fields) {
      final pending = widget.pendingOverrides[field.name];
      final text = pending != null
          ? _pendingText(field, pending)
          : _defaultText(field);
      _controllers[field.name] = TextEditingController(text: text);
    }
  }

  String _defaultText(FieldSchema field) => switch (field.type) {
    FieldType.bool_  => 'false',
    FieldType.enum_  => field.enumValues.firstOrNull ?? '',
    _                => '0',
  };

  /// Display text for a pending override. Enum overrides store the backing
  /// integer (the member index), so map it back to the member name for the
  /// dropdown / text field; everything else displays its value directly.
  String _pendingText(FieldSchema field, OverrideEntry override) {
    final v = override.newValue;
    if (field.type == FieldType.enum_ && v is int) {
      if (v >= 0 && v < field.enumValues.length) return field.enumValues[v];
    }
    return v.toString();
  }

  void _onChanged(FieldSchema field, String raw) {
    final error = validateField(field, raw);
    setState(() => _errors[field.name] = error);
    if (error != null) return;

    // Emit an override for every valid value, including the placeholder
    // default (0 / false / first enum member). The model carries no real CDO
    // default to compare against — the placeholder is only a display guess
    // (the true default of m_Value may be 4, not 0) — so treating "value ==
    // placeholder" as a revert both mis-fires and makes it impossible to set a
    // field to 0. Override removal is done from the OverridesPanel.
    widget.onOverrideChanged(OverrideEntry(
      classId:  widget.item.id,
      field:    field.name,
      oldValue: parsedValue(field, _defaultText(field)),
      newValue: parsedValue(field, raw),
    ));
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Row(
            children: [
              const Icon(Icons.tune, size: 20),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  widget.item.displayName,
                  style: theme.textTheme.titleMedium,
                ),
              ),
              Text(
                widget.item.id,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
        for (final field in widget.item.fields)
          Padding(
            padding: const EdgeInsets.only(bottom: 12),
            child: _FieldRow(
              schema:     field,
              controller: _controllers[field.name]!,
              error:      _errors[field.name],
              hasPending: widget.pendingOverrides.containsKey(field.name),
              onChanged:  (raw) => _onChanged(field, raw),
            ),
          ),
      ],
    );
  }
}

class _FieldRow extends StatelessWidget {
  const _FieldRow({
    required this.schema,
    required this.controller,
    required this.error,
    required this.hasPending,
    required this.onChanged,
  });

  final FieldSchema schema;
  final TextEditingController controller;
  final String? error;
  final bool hasPending;
  final void Function(String) onChanged;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    Widget input;

    if (schema.type == FieldType.bool_) {
      final boolVal = controller.text.trim() == 'true';
      input = Row(
        children: [
          Switch(
            value: boolVal,
            onChanged: (v) {
              controller.text = v.toString();
              onChanged(v.toString());
            },
          ),
          const SizedBox(width: 8),
          Text(boolVal ? 'true' : 'false'),
        ],
      );
    } else if (schema.type == FieldType.enum_) {
      input = DropdownButton<String>(
        value: schema.enumValues.contains(controller.text.trim())
            ? controller.text.trim()
            : schema.enumValues.firstOrNull,
        items: schema.enumValues
            .map((v) => DropdownMenuItem(value: v, child: Text(v)))
            .toList(),
        onChanged: (v) {
          if (v != null) {
            controller.text = v;
            onChanged(v);
          }
        },
      );
    } else {
      input = TextField(
        controller: controller,
        decoration: InputDecoration(
          // The field name is already shown in the left label column.
          errorText: error,
          isDense: true,
          suffixIcon: hasPending
              ? Icon(Icons.edit, size: 16, color: scheme.primary)
              : null,
        ),
        keyboardType: schema.type == FieldType.int_
            ? TextInputType.number
            : const TextInputType.numberWithOptions(decimal: true),
        onChanged: onChanged,
      );
    }

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 200,
          child: Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(
              schema.name,
              style: TextStyle(color: scheme.onSurfaceVariant),
            ),
          ),
        ),
        Expanded(child: input),
      ],
    );
  }
}
