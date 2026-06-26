import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../game_lang.dart';
import '../primary_set.dart';
import '../domain/loc_catalog_provider.dart';
import '../domain/loc_edits_notifier.dart';

/// One editable text field per game language for a single loc id, shared by the Dialoge tab
/// and the Items name editor. Each field is prefilled from the staged edit (if any) or the
/// catalog value; editing writes through [LocEditsNotifier] to that language's primary set.
/// Setting the field back to the catalog value clears the edit.
class LangFieldsEditor extends ConsumerWidget {
  const LangFieldsEditor({super.key, required this.locId});
  final String locId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final catalogAsync = ref.watch(locCatalogProvider);
    final catalog = catalogAsync.value;
    // Don't expose editable fields until the catalog is actually loaded with data: the target
    // loc set is derived from the catalog, so editing against a still-loading or empty catalog
    // could stage text on a set that differs once the catalog resolves.
    if (catalog == null || catalog.isEmpty) {
      return Padding(
        padding: const EdgeInsets.all(8),
        child: Text(
          catalogAsync.isLoading
              ? 'Loading localization…'
              : 'Extract the localization catalog to edit names.',
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final lang in kGameLangs)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: _LangField(locId: locId, lang: lang, catalog: catalog),
          ),
      ],
    );
  }
}

class _LangField extends ConsumerStatefulWidget {
  const _LangField({required this.locId, required this.lang, required this.catalog});
  final String locId;
  final GameLang lang;
  final Map<String, Map<String, String>> catalog;

  @override
  ConsumerState<_LangField> createState() => _LangFieldState();
}

class _LangFieldState extends ConsumerState<_LangField> {
  late final TextEditingController _controller;
  late final FocusNode _focusNode;

  String get _set => primarySetFor(widget.catalog, widget.locId, widget.lang);
  // The value stored in THIS language's target set (not the English fallback that
  // resolveGameText would return), so prefill, the edit-vs-original comparison, and what gets
  // written all agree — otherwise a field shows English yet writes to an empty target set, and
  // typing the fallback text would clear the edit.
  String get _catalogValue =>
      widget.catalog[widget.locId.toLowerCase()]?[_set] ?? '';

  String _currentValue() {
    final staged = ref.read(locEditsProvider).editFor(widget.locId, _set);
    return staged ?? _catalogValue;
  }

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: _currentValue());
    _focusNode = FocusNode();
  }

  @override
  void didUpdateWidget(covariant _LangField old) {
    super.didUpdateWidget(old);
    if (old.locId != widget.locId) {
      _controller.text = _currentValue();
    } else if (!identical(old.catalog, widget.catalog) && !_focusNode.hasFocus) {
      // The catalog was re-extracted/invalidated (a new map instance) while the same line stays
      // selected: _set and _catalogValue derive from widget.catalog, so resync to the refreshed
      // value rather than keep stale text (which a later edit would stage over the new catalog).
      // Skipped while focused so we don't clobber the user's in-progress typing.
      _controller.text = _currentValue();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _onChanged(String text) {
    final notifier = ref.read(locEditsProvider.notifier);
    if (text == _catalogValue) {
      notifier.removeEdit(widget.locId, _set);
    } else {
      notifier.setEdit(widget.locId, _set, text);
    }
  }

  @override
  Widget build(BuildContext context) {
    // Sync the field to EXTERNAL edit changes (project load, clear-all, remove row, clear
    // line) without clobbering the user's in-progress typing — skipped while the field is
    // focused, where the user's own onChanged already keeps the staged value current.
    ref.listen(locEditsProvider, (_, _) {
      final desired = _currentValue();
      if (!_focusNode.hasFocus && _controller.text != desired) {
        _controller.text = desired;
      }
    });
    // react to external edit changes (e.g. revert / project load) for the marker
    final modified =
        ref.watch(locEditsProvider).editFor(widget.locId, _set) != null;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 64,
          child: Padding(
            padding: const EdgeInsets.only(top: 14),
            child: Text(widget.lang.code,
                style: Theme.of(context).textTheme.labelSmall),
          ),
        ),
        Expanded(
          child: TextField(
            controller: _controller,
            focusNode: _focusNode,
            onChanged: _onChanged,
            minLines: 1,
            maxLines: 4,
            decoration: InputDecoration(
              isDense: true,
              border: const OutlineInputBorder(),
              labelText: widget.lang.endonym,
              suffixIcon: modified
                  ? IconButton(
                      tooltip: 'Revert',
                      icon: const Icon(Icons.undo, size: 18),
                      onPressed: () {
                        ref
                            .read(locEditsProvider.notifier)
                            .removeEdit(widget.locId, _set);
                        _controller.text = _catalogValue;
                      },
                    )
                  : null,
            ),
          ),
        ),
      ],
    );
  }
}
