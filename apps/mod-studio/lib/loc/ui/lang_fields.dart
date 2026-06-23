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
    final catalog = ref.watch(locCatalogProvider).value ?? const {};
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

  String get _set => primarySetFor(widget.catalog, widget.locId, widget.lang);
  String get _catalogValue =>
      resolveGameText(widget.catalog, widget.locId, widget.lang) ?? '';

  String _currentValue() {
    final staged = ref.read(locEditsProvider).editFor(widget.locId, _set);
    return staged ?? _catalogValue;
  }

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: _currentValue());
  }

  @override
  void didUpdateWidget(covariant _LangField old) {
    super.didUpdateWidget(old);
    if (old.locId != widget.locId) {
      _controller.text = _currentValue();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
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
