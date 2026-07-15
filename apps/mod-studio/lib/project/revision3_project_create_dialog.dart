import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';

/// Safe starting point selected for a new managed revision-3 project.
///
/// A starter is product intent only. Selecting one performs no filesystem or
/// game operation; the caller remains responsible for creating the empty
/// project and, for Draft starters, opening the existing guided setup after
/// that project has been created.
enum Revision3ProjectStarter { empty, npcDraft, questDraft }

/// Canonical, immutable input collected by [Revision3ProjectCreateDialog].
///
/// This value contains metadata and one starter choice only. Producing it
/// performs no filesystem, game, save, build, deployment, or runtime operation.
final class Revision3ProjectCreateFormResult {
  Revision3ProjectCreateFormResult({
    required this.name,
    required this.version,
    required this.author,
    required List<String> authoringLocales,
    this.starter = Revision3ProjectStarter.empty,
  }) : authoringLocales = List<String>.unmodifiable(authoringLocales);

  final String name;
  final String version;
  final String author;
  final List<String> authoringLocales;
  final Revision3ProjectStarter starter;
}

/// Opens the friendly project form and returns metadata plus starter intent.
Future<Revision3ProjectCreateFormResult?> showRevision3ProjectCreateDialog(
  BuildContext context,
) => showDialog<Revision3ProjectCreateFormResult>(
  context: context,
  builder: (_) => const Revision3ProjectCreateDialog(),
);

/// Write-free form for creating one managed offline project.
class Revision3ProjectCreateDialog extends StatefulWidget {
  const Revision3ProjectCreateDialog({super.key});

  @override
  State<Revision3ProjectCreateDialog> createState() =>
      _Revision3ProjectCreateDialogState();
}

class _Revision3ProjectCreateDialogState
    extends State<Revision3ProjectCreateDialog> {
  static const _maxNameUtf8Bytes = 256;
  static const _maxVersionUtf8Bytes = 128;
  static const _maxAuthorUtf8Bytes = 256;
  static const _maxAuthoringLocales = 64;

  final _formKey = GlobalKey<FormState>();
  final _name = TextEditingController();
  final _version = TextEditingController(text: '0.1.0');
  final _author = TextEditingController();
  final _locales = TextEditingController(text: 'en');
  Revision3ProjectStarter _starter = Revision3ProjectStarter.empty;

  @override
  void dispose() {
    _name.dispose();
    _version.dispose();
    _author.dispose();
    _locales.dispose();
    super.dispose();
  }

  void _submit() {
    FocusScope.of(context).unfocus();
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final l10n = AppLocalizations.of(context);
    Navigator.of(context).pop(
      Revision3ProjectCreateFormResult(
        name: _name.text,
        version: _version.text,
        author: _author.text,
        authoringLocales: _parseCanonicalLocales(_locales.text, l10n),
        starter: _starter,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AlertDialog(
      key: const Key('revision3-project-create-dialog'),
      title: Row(
        children: [
          const Icon(Icons.create_new_folder_outlined),
          const SizedBox(width: 10),
          Expanded(child: Text(l10n.projectCreateDialogTitle)),
        ],
      ),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Form(
            key: _formKey,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                const _ProjectCreationBoundary(),
                const SizedBox(height: 18),
                _ProjectStarterSelector(
                  starter: _starter,
                  onChanged: (value) => setState(() => _starter = value),
                ),
                const SizedBox(height: 18),
                TextFormField(
                  key: const Key('revision3-project-create-name'),
                  controller: _name,
                  autofocus: true,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.projectCreateNameLabel,
                    helperText: l10n.projectCreateNameHelper,
                    border: const OutlineInputBorder(),
                  ),
                  autovalidateMode: AutovalidateMode.onUserInteraction,
                  validator: (value) => _metadataError(
                    value,
                    label: l10n.projectCreateNameLabel,
                    maxUtf8Bytes: _maxNameUtf8Bytes,
                    l10n: l10n,
                  ),
                ),
                const SizedBox(height: 14),
                TextFormField(
                  key: const Key('revision3-project-create-version'),
                  controller: _version,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.projectCreateVersionLabel,
                    helperText: l10n.projectCreateVersionHelper,
                    border: const OutlineInputBorder(),
                  ),
                  autovalidateMode: AutovalidateMode.onUserInteraction,
                  validator: (value) => _metadataError(
                    value,
                    label: l10n.projectCreateVersionLabel,
                    maxUtf8Bytes: _maxVersionUtf8Bytes,
                    l10n: l10n,
                  ),
                ),
                const SizedBox(height: 14),
                TextFormField(
                  key: const Key('revision3-project-create-author'),
                  controller: _author,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.projectCreateAuthorLabel,
                    helperText: l10n.projectCreateAuthorHelper,
                    border: const OutlineInputBorder(),
                  ),
                  autovalidateMode: AutovalidateMode.onUserInteraction,
                  validator: (value) => _metadataError(
                    value,
                    label: l10n.projectCreateAuthorLabel,
                    maxUtf8Bytes: _maxAuthorUtf8Bytes,
                    l10n: l10n,
                  ),
                ),
                const SizedBox(height: 14),
                TextFormField(
                  key: const Key('revision3-project-create-locales'),
                  controller: _locales,
                  textInputAction: TextInputAction.done,
                  onFieldSubmitted: (_) => _submit(),
                  decoration: InputDecoration(
                    labelText: l10n.projectCreateLocalesLabel,
                    helperText: l10n.projectCreateLocalesHelper,
                    border: const OutlineInputBorder(),
                  ),
                  autovalidateMode: AutovalidateMode.onUserInteraction,
                  validator: (value) => _localeError(value, l10n),
                ),
              ],
            ),
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const Key('revision3-project-create-cancel'),
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton.icon(
          key: const Key('revision3-project-create-submit'),
          onPressed: _submit,
          icon: const Icon(Icons.add),
          label: Text(l10n.projectCreateSubmit),
        ),
      ],
    );
  }

  String? _localeError(String? value, AppLocalizations l10n) {
    try {
      _parseCanonicalLocales(value ?? '', l10n);
      return null;
    } on FormatException catch (error) {
      return error.message.toString();
    }
  }

  List<String> _parseCanonicalLocales(String value, AppLocalizations l10n) {
    if (value.trim().isEmpty) {
      throw FormatException(l10n.projectCreateLocalesRequired);
    }
    final unique = <String>{};
    for (final token in value.split(',')) {
      final locale = token.trim();
      if (locale.isEmpty) {
        throw FormatException(l10n.projectCreateLocalesEmptyEntry);
      }
      _requireCanonicalLocale(locale, l10n);
      unique.add(locale);
      if (unique.length > _maxAuthoringLocales) {
        throw FormatException(
          l10n.projectCreateLocalesTooMany(_maxAuthoringLocales),
        );
      }
    }
    final result = unique.toList()..sort();
    return List<String>.unmodifiable(result);
  }
}

class _ProjectStarterSelector extends StatelessWidget {
  const _ProjectStarterSelector({
    required this.starter,
    required this.onChanged,
  });

  final Revision3ProjectStarter starter;
  final ValueChanged<Revision3ProjectStarter> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final options = <_ProjectStarterOptionData>[
      _ProjectStarterOptionData(
        starter: Revision3ProjectStarter.empty,
        keyName: 'empty',
        icon: Icons.note_add_outlined,
        title: l10n.projectStarterEmptyTitle,
        description: l10n.projectStarterEmptyDescription,
      ),
      _ProjectStarterOptionData(
        starter: Revision3ProjectStarter.npcDraft,
        keyName: 'npc-draft',
        icon: Icons.person_add_alt_1_outlined,
        title: l10n.projectStarterNpcDraftTitle,
        description: l10n.projectStarterNpcDraftDescription,
      ),
      _ProjectStarterOptionData(
        starter: Revision3ProjectStarter.questDraft,
        keyName: 'quest-draft',
        icon: Icons.assignment_add,
        title: l10n.projectStarterQuestDraftTitle,
        description: l10n.projectStarterQuestDraftDescription,
      ),
    ];
    return Semantics(
      container: true,
      label: l10n.projectStarterSemanticsLabel,
      child: Column(
        key: const Key('revision3-project-starter-selector'),
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            l10n.projectStarterPrompt,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 4),
          Text(
            l10n.projectStarterWriteBoundary,
            key: const Key('revision3-project-starter-write-boundary'),
          ),
          const SizedBox(height: 10),
          RadioGroup<Revision3ProjectStarter>(
            groupValue: starter,
            onChanged: (value) {
              if (value != null) onChanged(value);
            },
            child: LayoutBuilder(
              builder: (context, constraints) {
                final optionWidgets = <Widget>[
                  for (final option in options)
                    _ProjectStarterOption(option: option),
                ];
                if (constraints.maxWidth < 480) {
                  return Column(
                    key: const Key('revision3-project-starter-options-column'),
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      for (var index = 0; index < optionWidgets.length; index++)
                        Padding(
                          padding: EdgeInsets.only(
                            bottom: index + 1 == optionWidgets.length ? 0 : 8,
                          ),
                          child: optionWidgets[index],
                        ),
                    ],
                  );
                }
                return Row(
                  key: const Key('revision3-project-starter-options-row'),
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (
                      var index = 0;
                      index < optionWidgets.length;
                      index++
                    ) ...[
                      Expanded(child: optionWidgets[index]),
                      if (index + 1 != optionWidgets.length)
                        const SizedBox(width: 8),
                    ],
                  ],
                );
              },
            ),
          ),
          const SizedBox(height: 10),
          Text(
            l10n.projectStarterPartialOutcome,
            key: const Key('revision3-project-starter-partial-outcome'),
          ),
        ],
      ),
    );
  }
}

class _ProjectStarterOption extends StatelessWidget {
  const _ProjectStarterOption({required this.option});

  final _ProjectStarterOptionData option;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    clipBehavior: Clip.antiAlias,
    child: RadioListTile<Revision3ProjectStarter>(
      key: Key('revision3-project-starter-${option.keyName}'),
      value: option.starter,
      contentPadding: const EdgeInsets.fromLTRB(10, 4, 10, 8),
      title: Row(
        children: [
          Icon(option.icon, size: 20),
          const SizedBox(width: 8),
          Expanded(child: Text(option.title)),
        ],
      ),
      subtitle: Padding(
        padding: const EdgeInsets.only(top: 4),
        child: Text(option.description),
      ),
    ),
  );
}

final class _ProjectStarterOptionData {
  const _ProjectStarterOptionData({
    required this.starter,
    required this.keyName,
    required this.icon,
    required this.title,
    required this.description,
  });

  final Revision3ProjectStarter starter;
  final String keyName;
  final IconData icon;
  final String title;
  final String description;
}

class _ProjectCreationBoundary extends StatelessWidget {
  const _ProjectCreationBoundary();

  @override
  Widget build(BuildContext context) => Container(
    key: const Key('revision3-project-create-boundary'),
    padding: const EdgeInsets.all(14),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(12),
    ),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Icon(Icons.inventory_2_outlined),
        const SizedBox(width: 12),
        Expanded(
          child: Text(AppLocalizations.of(context).projectCreateBoundary),
        ),
      ],
    ),
  );
}

String? _metadataError(
  String? raw, {
  required String label,
  required int maxUtf8Bytes,
  required AppLocalizations l10n,
}) {
  final value = raw ?? '';
  if (value.isEmpty) return l10n.projectCreateMetadataRequired(label);
  if (value.trim() != value) {
    return l10n.projectCreateMetadataNoOuterWhitespace(label);
  }
  try {
    _boundedUtf8Length(value, maxUtf8Bytes, label, l10n);
  } on FormatException catch (error) {
    return error.message.toString();
  }
  for (final rune in value.runes) {
    if (rune < 0x20 || (rune >= 0x7f && rune <= 0x9f)) {
      return l10n.projectCreateMetadataControlCharacters(label);
    }
  }
  return null;
}

void _requireCanonicalLocale(String value, AppLocalizations l10n) {
  if (value.isEmpty || value.length > 35 || !value.codeUnits.every(_isAscii)) {
    throw FormatException(l10n.projectCreateLocaleBoundedAscii(value));
  }
  final segments = value.split('-');
  final language = segments.first;
  if (language.length < 2 || language.length > 8 || !_allAsciiLower(language)) {
    throw FormatException(l10n.projectCreateLocaleLanguage(value));
  }
  final canonical = StringBuffer(language);
  for (var index = 1; index < segments.length; index++) {
    final segment = segments[index];
    if (segment.isEmpty ||
        segment.length > 8 ||
        !_allAsciiAlphanumeric(segment)) {
      throw FormatException(l10n.projectCreateLocaleInvalidSegment(value));
    }
    canonical.write('-');
    if (segment.length == 4 && _allAsciiAlphabetic(segment)) {
      canonical.write(
        '${segment[0].toUpperCase()}${segment.substring(1).toLowerCase()}',
      );
    } else if (segment.length == 2 && _allAsciiAlphabetic(segment)) {
      canonical.write(segment.toUpperCase());
    } else {
      canonical.write(segment.toLowerCase());
    }
  }
  if (canonical.toString() != value) {
    throw FormatException(
      l10n.projectCreateLocaleNotCanonical(value, canonical.toString()),
    );
  }
}

bool _isAscii(int unit) => unit <= 0x7f;

bool _allAsciiLower(String value) =>
    value.codeUnits.every((unit) => unit >= 0x61 && unit <= 0x7a);

bool _allAsciiAlphabetic(String value) => value.codeUnits.every(
  (unit) => (unit >= 0x41 && unit <= 0x5a) || (unit >= 0x61 && unit <= 0x7a),
);

bool _allAsciiAlphanumeric(String value) => value.codeUnits.every(
  (unit) =>
      (unit >= 0x30 && unit <= 0x39) ||
      (unit >= 0x41 && unit <= 0x5a) ||
      (unit >= 0x61 && unit <= 0x7a),
);

int _boundedUtf8Length(
  String value,
  int maxBytes,
  String context,
  AppLocalizations l10n,
) {
  var length = 0;
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    final int width;
    if (unit <= 0x7f) {
      width = 1;
    } else if (unit <= 0x7ff) {
      width = 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw FormatException(l10n.projectCreateMetadataMalformed(context));
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw FormatException(l10n.projectCreateMetadataMalformed(context));
      }
      index++;
      width = 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw FormatException(l10n.projectCreateMetadataMalformed(context));
    } else {
      width = 3;
    }
    length += width;
    if (length > maxBytes) {
      throw FormatException(
        l10n.projectCreateMetadataTooLong(context, maxBytes),
      );
    }
  }
  return length;
}
