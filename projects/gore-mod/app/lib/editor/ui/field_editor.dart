import 'package:flutter/material.dart';
import '../../catalog/domain/field_schema.dart';
import '../../catalog/domain/item_entry.dart';
import '../domain/field_validator.dart';
import '../domain/override_entry.dart';

/// Per-item field editor: renders each [FieldSchema] as a typed input.
/// Calls [onOverrideChanged] with a valid [OverrideEntry] on every
/// committed valid change; calls [onOverrideCleared] when the user
/// reverts a field to the catalog default.
class FieldEditor extends StatefulWidget {
  const FieldEditor({
    super.key,
    required this.item,
    required this.pendingOverrides,
    required this.onOverrideChanged,
    required this.onOverrideCleared,
  });

  final CatalogItem item;

  /// Existing pending overrides for this item (keyed by field name).
  final Map<String, OverrideEntry> pendingOverrides;
  final void Function(OverrideEntry) onOverrideChanged;
  final void Function(String fieldName) onOverrideCleared;

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
    // clobber in-progress input.
    for (final field in widget.item.fields) {
      final controller = _controllers[field.name];
      if (controller == null || _errors[field.name] != null) continue;
      final pending = widget.pendingOverrides[field.name];
      final expected =
          pending != null ? pending.newValue.toString() : _defaultText(field);
      if (controller.text != expected) {
        controller.text = expected;
      }
    }
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
          ? pending.newValue.toString()
          : _defaultText(field);
      _controllers[field.name] = TextEditingController(text: text);
    }
  }

  String _defaultText(FieldSchema field) => switch (field.type) {
    FieldType.bool_  => 'false',
    FieldType.enum_  => field.enumValues.firstOrNull ?? '',
    _                => '0',
  };

  void _onChanged(FieldSchema field, String raw) {
    final error = validateField(field, raw);
    setState(() => _errors[field.name] = error);
    if (error != null) return;

    final value = parsedValue(field, raw);
    // If reverted to the default, clear the override instead of emitting one —
    // including when a pending override already exists for this field (the
    // common edit→revert flow), otherwise the no-op default assignment would
    // stay in the export payload.
    final defaultStr = _defaultText(field);
    if (raw.trim() == defaultStr) {
      widget.onOverrideCleared(field.name);
      return;
    }
    widget.onOverrideChanged(OverrideEntry(
      classId:  widget.item.id,
      field:    field.name,
      oldValue: parsedValue(field, defaultStr),
      newValue: value,
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
          labelText: schema.name,
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
          width: 130,
          child: Padding(
            padding: const EdgeInsets.only(top: 12),
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
