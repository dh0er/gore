import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;
import 'package:url_launcher/url_launcher.dart';

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../../loc/game_lang.dart';
import '../domain/voice_edits_notifier.dart';

typedef VoiceArchiveMatcher =
    Future<VoiceArchiveMatchLineResult> Function({
      required String archive,
      required String locId,
    });

typedef VoiceOggPicker = Future<String?> Function();

typedef VoiceOggInspector =
    Future<VoiceOggInspectionResult> Function({required String oggPath});

/// Line-first authoring for replacing one existing spoken localization line.
///
/// This deliberately does not offer voice additions. A replacement is staged
/// only after the installed archive resolves to one user-confirmed member and
/// its exact archive/member observation has been captured.
class VoiceLineEditor extends ConsumerStatefulWidget {
  const VoiceLineEditor({
    required this.locId,
    this.matcher,
    this.oggPicker,
    this.oggInspector,
    super.key,
  });

  final String locId;

  /// Test seam for hermetic archive inspection.
  final VoiceArchiveMatcher? matcher;

  /// Test seam for hermetic file selection.
  final VoiceOggPicker? oggPicker;

  /// Test seam for hermetic native Ogg validation.
  final VoiceOggInspector? oggInspector;

  @override
  ConsumerState<VoiceLineEditor> createState() => _VoiceLineEditorState();
}

class _VoiceLineEditorState extends ConsumerState<VoiceLineEditor> {
  late String _localeCode;
  int _generation = 0;
  bool _busy = false;
  String? _status;
  VoiceArchiveMatchLineResult? _ambiguousResult;
  VoiceArchiveEntryInfo? _selectedCandidate;
  VoiceOggInspectionResult? _oggInspection;

  @override
  void initState() {
    super.initState();
    _localeCode = gameLangByCode(ref.read(localeProvider)).code;
  }

  @override
  void didUpdateWidget(covariant VoiceLineEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.locId == widget.locId) return;
    _generation++;
    _localeCode = gameLangByCode(ref.read(localeProvider)).code;
    _busy = false;
    _status = null;
    _ambiguousResult = null;
    _selectedCandidate = null;
    _oggInspection = null;
  }

  @override
  void dispose() {
    _generation++;
    super.dispose();
  }

  VoiceArchiveMatcher get _matcher =>
      widget.matcher ??
      ({required String archive, required String locId}) => ModFfi(
        ref.read(coreServiceProvider),
      ).voiceArchiveMatchLine(archive: archive, locId: locId);

  VoiceOggPicker get _oggPicker => widget.oggPicker ?? _pickOgg;

  VoiceOggInspector get _oggInspector =>
      widget.oggInspector ??
      ({required String oggPath}) => ModFfi(
        ref.read(coreServiceProvider),
      ).voiceOggInspectV1(oggPath: oggPath);

  bool _isCurrent(int generation, String locId, String locale) =>
      mounted &&
      generation == _generation &&
      widget.locId == locId &&
      _localeCode == locale;

  void _selectLocale(String? code) {
    if (code == null || code == _localeCode) return;
    setState(() {
      _generation++;
      _localeCode = code;
      _busy = false;
      _status = null;
      _ambiguousResult = null;
      _selectedCandidate = null;
      _oggInspection = null;
    });
  }

  Future<void> _inspect(String archivePath) async {
    final generation = ++_generation;
    final locId = widget.locId;
    final locale = _localeCode;
    setState(() {
      _busy = true;
      _status = 'Checking the original spoken line…';
      _ambiguousResult = null;
      _selectedCandidate = null;
      _oggInspection = null;
    });

    try {
      final result = await _matcher(archive: archivePath, locId: locId);
      if (!_isCurrent(generation, locId, locale)) return;
      _validateInspectionIdentity(result, archivePath, locId);

      switch (result.resolution) {
        case VoiceArchiveLineResolution.unresolved:
          setState(() {
            _busy = false;
            _status =
                'This line has no existing spoken audio in '
                '${gameLangByCode(locale).endonym}. Creating new voiced '
                'lines is not qualified yet, so nothing was changed.';
          });
        case VoiceArchiveLineResolution.ambiguous:
          setState(() {
            _busy = false;
            _ambiguousResult = result;
            _status =
                'More than one original recording matches this line. '
                'Choose the exact entry below before importing your Ogg.';
          });
        case VoiceArchiveLineResolution.unique:
          await _chooseAndStage(
            archivePath: archivePath,
            result: result,
            entry: result.matches.single,
            generation: generation,
            locId: locId,
            locale: locale,
          );
      }
    } catch (error) {
      if (!_isCurrent(generation, locId, locale)) return;
      setState(() {
        _busy = false;
        _status = 'The original spoken line could not be checked: $error';
      });
    }
  }

  Future<void> _useSelectedCandidate(String archivePath) async {
    final result = _ambiguousResult;
    final entry = _selectedCandidate;
    if (result == null || entry == null) return;
    final generation = ++_generation;
    final locId = widget.locId;
    final locale = _localeCode;
    setState(() {
      _busy = true;
      _status = 'Choose your replacement Ogg recording…';
    });
    await _chooseAndStage(
      archivePath: archivePath,
      result: result,
      entry: entry,
      generation: generation,
      locId: locId,
      locale: locale,
    );
  }

  Future<void> _chooseAndStage({
    required String archivePath,
    required VoiceArchiveMatchLineResult result,
    required VoiceArchiveEntryInfo entry,
    required int generation,
    required String locId,
    required String locale,
  }) async {
    try {
      if (!_isCurrent(generation, locId, locale)) return;
      setState(() {
        _busy = true;
        _status = 'Choose your replacement Ogg recording…';
      });
      final oggPath = await _oggPicker();
      if (!_isCurrent(generation, locId, locale)) return;
      if (oggPath == null) {
        setState(() {
          _busy = false;
          _status = null;
        });
        return;
      }
      _validatePickedOgg(oggPath);
      if (!_isCurrent(generation, locId, locale)) return;
      setState(() {
        _status = 'Validating the selected Ogg recording…';
      });
      final inspection = await _oggInspector(oggPath: oggPath);
      if (!_isCurrent(generation, locId, locale)) return;

      final edit = VoiceArchiveEdit(
        locId: locId,
        locale: locale,
        archive: p.basename(archivePath),
        operation: VoicePatchOperation.replace,
        archivePath: entry.path,
        oggPath: oggPath,
        observation: VoiceArchiveObservation(
          archiveSize: result.archiveSize,
          archiveSha256: result.archiveSha256,
          memberProof: VoiceMemberProof.present(
            uncompressedSize: entry.uncompressedSize,
            crc32: entry.crc32,
          ),
        ),
      );
      ref.read(voiceEditsProvider.notifier).setEdit(edit);
      if (!_isCurrent(generation, locId, locale)) {
        // The edit was authored for the captured semantic slot, never for the
        // now-visible line. No UI update is allowed after a selection change.
        return;
      }
      setState(() {
        _busy = false;
        _oggInspection = inspection;
        _status =
            'Replacement staged for ${gameLangByCode(locale).endonym}. '
            '${_inspectionSummary(inspection)}';
        _ambiguousResult = null;
        _selectedCandidate = null;
      });
    } catch (error) {
      if (!_isCurrent(generation, locId, locale)) return;
      setState(() {
        _busy = false;
        _oggInspection = null;
        _status = 'The replacement could not be staged: $error';
      });
    }
  }

  Future<void> _preview(String path) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      _validatePickedOgg(path);
      final opened = await launchUrl(
        Uri.file(path, windows: Platform.isWindows),
        mode: LaunchMode.externalApplication,
      );
      if (!opened) {
        throw const FileSystemException(
          'No external application accepted the selected Ogg file.',
        );
      }
    } catch (error) {
      messenger.showSnackBar(
        SnackBar(content: Text('Preview could not be opened: $error')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final lang = gameLangByCode(_localeCode);
    final configuredPath = ref.watch(gameExePathProvider);
    final gameRoot = gameRootFromExe(configuredPath);
    final archivePath = gameRoot == null
        ? null
        : _firstExistingVoiceArchive(gameRoot, lang);
    final staged = ref.watch(
      voiceEditsProvider.select(
        (state) => state.items[(widget.locId.toLowerCase(), _localeCode)],
      ),
    );

    final blocker = gameRoot == null
        ? 'Choose your Gothic installation in Settings before replacing '
              'speech.'
        : archivePath == null
        ? 'No voice archive was found for ${lang.endonym}. Check the game '
              'installation or choose another language.'
        : null;

    return Card(
      margin: const EdgeInsets.only(top: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                const Icon(Icons.record_voice_over_outlined, size: 20),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Spoken line',
                    style: theme.textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              'Replace the recording for this dialog line without changing '
              'its text.',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            DropdownButtonFormField<String>(
              key: const ValueKey('voice-locale'),
              initialValue: _localeCode,
              decoration: const InputDecoration(labelText: 'Voice language'),
              items: [
                for (final gameLang in kGameLangs)
                  DropdownMenuItem(
                    value: gameLang.code,
                    child: Text(gameLang.endonym),
                  ),
              ],
              onChanged: _selectLocale,
            ),
            if (staged != null) ...[
              const SizedBox(height: 12),
              Container(
                key: const ValueKey('voice-staged'),
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: theme.colorScheme.secondaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.check_circle_outline),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          const Text('Replacement ready'),
                          Text(
                            p.basename(staged.oggPath),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                    TextButton.icon(
                      key: const ValueKey('voice-preview'),
                      onPressed: () => _preview(staged.oggPath),
                      icon: const Icon(Icons.play_arrow, size: 18),
                      label: const Text('Preview'),
                    ),
                    TextButton.icon(
                      key: const ValueKey('voice-replace'),
                      onPressed: _busy || archivePath == null
                          ? null
                          : () => _inspect(archivePath),
                      icon: const Icon(Icons.swap_horiz, size: 18),
                      label: const Text('Replace'),
                    ),
                    IconButton(
                      key: const ValueKey('voice-remove'),
                      tooltip: 'Remove spoken-line replacement',
                      onPressed: _busy
                          ? null
                          : () {
                              ref
                                  .read(voiceEditsProvider.notifier)
                                  .remove(widget.locId, _localeCode);
                              setState(() {
                                _status = null;
                                _oggInspection = null;
                              });
                            },
                      icon: const Icon(Icons.delete_outline),
                    ),
                  ],
                ),
              ),
            ],
            const SizedBox(height: 12),
            if (blocker != null)
              _VoiceNotice(
                key: const ValueKey('voice-blocked'),
                icon: Icons.info_outline,
                text: blocker,
              )
            else if (staged == null && _ambiguousResult == null)
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.icon(
                  key: const ValueKey('voice-choose-ogg'),
                  onPressed: _busy ? null : () => _inspect(archivePath!),
                  icon: const Icon(Icons.audio_file_outlined, size: 18),
                  label: Text(_busy ? 'Checking…' : 'Choose Ogg recording'),
                ),
              ),
            if (_status != null) ...[
              const SizedBox(height: 10),
              Text(
                _status!,
                key: const ValueKey('voice-status'),
                style: theme.textTheme.bodySmall,
              ),
            ],
            if (_ambiguousResult != null && archivePath != null) ...[
              const SizedBox(height: 8),
              RadioGroup<VoiceArchiveEntryInfo>(
                groupValue: _selectedCandidate,
                onChanged: _busy
                    ? (_) {}
                    : (value) => setState(() => _selectedCandidate = value),
                child: Column(
                  children: [
                    for (final candidate in _ambiguousResult!.matches)
                      RadioListTile<VoiceArchiveEntryInfo>(
                        key: ValueKey('voice-candidate-${candidate.index}'),
                        dense: true,
                        contentPadding: EdgeInsets.zero,
                        title: Text(candidate.path),
                        value: candidate,
                        enabled: !_busy,
                      ),
                  ],
                ),
              ),
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.icon(
                  key: const ValueKey('voice-use-selected'),
                  onPressed: _busy || _selectedCandidate == null
                      ? null
                      : () => _useSelectedCandidate(archivePath),
                  icon: const Icon(Icons.audio_file_outlined, size: 18),
                  label: const Text('Use selected recording'),
                ),
              ),
            ],
            if (archivePath != null)
              ExpansionTile(
                tilePadding: EdgeInsets.zero,
                childrenPadding: EdgeInsets.zero,
                title: const Text('Technical details'),
                children: [
                  Align(
                    alignment: Alignment.centerLeft,
                    child: SelectableText(
                      'Installed archive: ${p.basename(archivePath)}',
                      style: theme.textTheme.bodySmall,
                    ),
                  ),
                  if (_oggInspection != null)
                    Align(
                      alignment: Alignment.centerLeft,
                      child: SelectableText(
                        _inspectionTechnicalDetails(_oggInspection!),
                        key: const ValueKey('voice-ogg-details'),
                        style: theme.textTheme.bodySmall,
                      ),
                    ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}

class _VoiceNotice extends StatelessWidget {
  const _VoiceNotice({required this.icon, required this.text, super.key});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Icon(icon, size: 18),
      const SizedBox(width: 8),
      Expanded(child: Text(text)),
    ],
  );
}

String? _firstExistingVoiceArchive(String gameRoot, GameLang lang) {
  final voiceRoot = p.join(gameRoot, 'G1R', 'Story', 'VoiceOver');
  for (final locSet in lang.locSets) {
    final candidate = p.join(voiceRoot, '$locSet.zip');
    if (File(candidate).existsSync()) return candidate;
  }
  return null;
}

void _validateInspectionIdentity(
  VoiceArchiveMatchLineResult result,
  String archivePath,
  String locId,
) {
  if (result.locId != locId || !p.equals(result.archive, archivePath)) {
    throw const FormatException(
      'archive inspection returned a different line or archive',
    );
  }
}

Future<String?> _pickOgg() async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(label: 'Ogg audio', extensions: ['ogg']),
    ],
  );
  return file?.path;
}

void _validatePickedOgg(String path) {
  if (!path.toLowerCase().endsWith('.ogg') || path.runes.any(_isControlRune)) {
    throw const FormatException('Choose an Ogg audio file.');
  }
  final type = FileSystemEntity.typeSync(path, followLinks: false);
  if (type != FileSystemEntityType.file) {
    throw const FormatException('The selected Ogg must be a regular file.');
  }
}

bool _isControlRune(int rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);

String _inspectionSummary(VoiceOggInspectionResult inspection) {
  final codec = switch (inspection.codec) {
    VoiceOggCodec.vorbis => 'Vorbis',
    VoiceOggCodec.opus => 'Opus',
  };
  final pages = inspection.pages == 1 ? 'page' : 'pages';
  final streams = inspection.streams == 1 ? 'stream' : 'streams';
  return 'Validated $codec Ogg: ${inspection.pages} $pages, '
      '${inspection.streams} $streams, '
      '${_formatByteLength(inspection.contentSeal.byteLength)}.';
}

String _inspectionTechnicalDetails(VoiceOggInspectionResult inspection) =>
    '${_inspectionSummary(inspection)}\n'
    'Validated size: ${inspection.contentSeal.byteLength} bytes\n'
    'SHA-256: ${inspection.contentSeal.sha256}';

String _formatByteLength(int bytes) {
  if (bytes < 1024) return '$bytes bytes';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KiB';
  return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MiB';
}
