import 'dart:convert';

import 'package:flutter_riverpod/legacy.dart';

/// The operation eventually lowered to `gore_mod::VoicePatchOp`.
enum VoicePatchOperation { add, replace }

String voicePatchOperationToString(VoicePatchOperation operation) =>
    switch (operation) {
      VoicePatchOperation.add => 'add',
      VoicePatchOperation.replace => 'replace',
    };

VoicePatchOperation _voicePatchOperationFromJson(Object? value) =>
    switch (value) {
      'add' => VoicePatchOperation.add,
      'replace' => VoicePatchOperation.replace,
      _ => throw FormatException('unsupported voice patch operation: $value'),
    };

enum VoiceMemberProofState { present, absent }

/// A read-only observation of the target member in one exact archive snapshot.
///
/// This is authoring evidence, not a user-controlled build qualification flag.
class VoiceMemberProof {
  const VoiceMemberProof.absent()
    : state = VoiceMemberProofState.absent,
      uncompressedSize = null,
      crc32 = null;

  const VoiceMemberProof.present({
    required int this.uncompressedSize,
    required int this.crc32,
  }) : state = VoiceMemberProofState.present;

  final VoiceMemberProofState state;
  final int? uncompressedSize;
  final int? crc32;

  Map<String, Object?> toJson() => switch (state) {
    VoiceMemberProofState.absent => const {'state': 'absent'},
    VoiceMemberProofState.present => {
      'state': 'present',
      'uncompressed_size': uncompressedSize,
      'crc32': crc32,
    },
  };

  factory VoiceMemberProof.fromJson(Map<String, Object?> json) {
    switch (json['state']) {
      case 'absent':
        if (json.containsKey('uncompressed_size') ||
            json.containsKey('crc32')) {
          throw const FormatException(
            'absent voice member proof cannot contain member metadata',
          );
        }
        return const VoiceMemberProof.absent();
      case 'present':
        final size = json['uncompressed_size'];
        final crc32 = json['crc32'];
        if (size is! int || size <= 0) {
          throw const FormatException(
            'voice member uncompressed_size must be a positive integer',
          );
        }
        if (crc32 is! int || crc32 < 0 || crc32 > 0xffffffff) {
          throw const FormatException(
            'voice member crc32 must be an unsigned 32-bit integer',
          );
        }
        return VoiceMemberProof.present(uncompressedSize: size, crc32: crc32);
      default:
        throw FormatException(
          'unsupported voice member proof state: ${json['state']}',
        );
    }
  }
}

/// Seal and member-presence proof captured by read-only archive inspection.
class VoiceArchiveObservation {
  const VoiceArchiveObservation({
    required this.archiveSize,
    required this.archiveSha256,
    required this.memberProof,
  });

  final int archiveSize;
  final String archiveSha256;
  final VoiceMemberProof memberProof;

  Map<String, Object?> toJson() => {
    'archive_size': archiveSize,
    'archive_sha256': archiveSha256,
    'member_proof': memberProof.toJson(),
  };

  factory VoiceArchiveObservation.fromJson(Map<String, Object?> json) {
    final archiveSize = json['archive_size'];
    final archiveSha256 = json['archive_sha256'];
    final memberProof = json['member_proof'];
    if (archiveSize is! int || archiveSize <= 0) {
      throw const FormatException(
        'voice archive_size must be a positive integer',
      );
    }
    if (archiveSha256 is! String ||
        !RegExp(r'^[0-9a-f]{64}$').hasMatch(archiveSha256)) {
      throw const FormatException(
        'voice archive_sha256 must be a lowercase SHA-256 digest',
      );
    }
    if (memberProof is! Map) {
      throw const FormatException('voice member_proof must be an object');
    }
    return VoiceArchiveObservation(
      archiveSize: archiveSize,
      archiveSha256: archiveSha256,
      memberProof: VoiceMemberProof.fromJson(
        memberProof.cast<String, Object?>(),
      ),
    );
  }
}

/// One selected Ogg edit for a semantic localization-line/locale slot.
///
/// [locId] and canonical [locale] remain in the project so future UI code never
/// has to reconstruct semantic identity from an archive member basename.
class VoiceArchiveEdit {
  const VoiceArchiveEdit({
    required this.locId,
    required this.locale,
    required this.archive,
    required this.operation,
    required this.archivePath,
    required this.oggPath,
    required this.observation,
  });

  final String locId;
  final String locale;
  final String archive;
  final VoicePatchOperation operation;
  final String archivePath;
  final String oggPath;
  final VoiceArchiveObservation observation;

  (String, String) get semanticKey => (locId.toLowerCase(), locale);
  (String, String) get deploymentKey =>
      (archive.toLowerCase(), archivePath.toLowerCase());

  Map<String, Object?> toJson() {
    validateVoiceArchiveEdit(this);
    return {
      'loc_id': locId,
      'locale': locale,
      'archive': archive,
      'op': voicePatchOperationToString(operation),
      'archive_path': archivePath,
      'ogg_path': oggPath,
      'observation': observation.toJson(),
    };
  }

  /// The exact shape consumed by `gore_mod::VoiceArchiveEdit`.
  ///
  /// Authoring-only semantic identity does not cross this boundary. The sealed
  /// archive observation does: production must verify that the archive and
  /// target member still match the read-only inspection which authorized this
  /// edit.
  Map<String, Object?> toBuildJson() {
    validateVoiceArchiveEdit(this);
    return {
      'archive': archive,
      'op': voicePatchOperationToString(operation),
      'archive_path': archivePath,
      'ogg_path': oggPath,
      'observation': observation.toJson(),
    };
  }

  factory VoiceArchiveEdit.fromJson(Map<String, Object?> json) {
    final observation = json['observation'];
    if (observation is! Map) {
      throw const FormatException(
        'voice edit requires a sealed archive observation',
      );
    }
    final edit = VoiceArchiveEdit(
      locId: json['loc_id'] as String,
      locale: json['locale'] as String,
      archive: json['archive'] as String,
      operation: _voicePatchOperationFromJson(json['op']),
      archivePath: json['archive_path'] as String,
      oggPath: json['ogg_path'] as String,
      observation: VoiceArchiveObservation.fromJson(
        observation.cast<String, Object?>(),
      ),
    );
    validateVoiceArchiveEdit(edit);
    return edit;
  }

  VoiceArchiveEdit withOggPath(String path) => VoiceArchiveEdit(
    locId: locId,
    locale: locale,
    archive: archive,
    operation: operation,
    archivePath: archivePath,
    oggPath: path,
    observation: observation,
  );
}

void validateVoiceArchiveEdit(VoiceArchiveEdit edit) {
  if (edit.locId.isEmpty ||
      edit.locId.length > 512 ||
      edit.locId.trim() != edit.locId ||
      edit.locId == '.' ||
      edit.locId == '..' ||
      edit.locId.contains('/') ||
      edit.locId.contains(r'\') ||
      !_isAscii(edit.locId) ||
      edit.locId.runes.any(_isControlRune)) {
    throw const FormatException(
      'voice loc_id must be one trimmed ASCII basename stem',
    );
  }
  _validateCanonicalLocale(edit.locale);
  if (!_isSafeArchiveName(edit.archive)) {
    throw const FormatException('voice archive must be one safe .zip filename');
  }
  if (!_isSafeArchiveMember(edit.archivePath)) {
    throw const FormatException(
      'voice archive_path must be a safe forward-slash .ogg path',
    );
  }
  if (edit.oggPath.isEmpty ||
      edit.oggPath.runes.any(_isControlRune) ||
      !edit.oggPath.toLowerCase().endsWith('.ogg')) {
    throw const FormatException(
      'voice ogg_path must name a non-empty Ogg file',
    );
  }
  // Round-trip through the strict parser so programmatically authored values
  // receive the same structural checks as project JSON.
  VoiceArchiveObservation.fromJson(edit.observation.toJson());
  final proofState = edit.observation.memberProof.state;
  final proofMatchesOperation = switch (edit.operation) {
    VoicePatchOperation.add => proofState == VoiceMemberProofState.absent,
    VoicePatchOperation.replace => proofState == VoiceMemberProofState.present,
  };
  if (!proofMatchesOperation) {
    throw const FormatException(
      'voice operation does not match the observed member presence',
    );
  }
  final expectedBasename = '${edit.locId}.ogg';
  final memberBasename = edit.archivePath.split('/').last;
  if (!_asciiEqualsIgnoreCase(memberBasename, expectedBasename)) {
    throw const FormatException(
      'voice archive_path basename must equal loc_id plus .ogg',
    );
  }
}

bool _isAscii(String value) => value.codeUnits.every((unit) => unit <= 0x7f);

bool _asciiEqualsIgnoreCase(String left, String right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    final leftUnit = left.codeUnitAt(index);
    final rightUnit = right.codeUnitAt(index);
    if (leftUnit > 0x7f || rightUnit > 0x7f) return false;
    final foldedLeft = leftUnit >= 0x41 && leftUnit <= 0x5a
        ? leftUnit + 0x20
        : leftUnit;
    final foldedRight = rightUnit >= 0x41 && rightUnit <= 0x5a
        ? rightUnit + 0x20
        : rightUnit;
    if (foldedLeft != foldedRight) return false;
  }
  return true;
}

bool _isControlRune(int rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);

bool _isSafeArchiveName(String value) =>
    !value.contains('/') &&
    _hasSafePortableComponents(value) &&
    value.toLowerCase().endsWith('.zip');

bool _isSafeArchiveMember(String value) {
  if (!_hasSafePortableComponents(value) ||
      !value.toLowerCase().endsWith('.ogg')) {
    return false;
  }
  return true;
}

bool _hasSafePortableComponents(String value) {
  if (value.isEmpty ||
      utf8.encode(value).length > 1024 ||
      value.startsWith('/') ||
      value.startsWith(r'\') ||
      value.contains(r'\') ||
      value.runes.any(_isControlRune)) {
    return false;
  }
  for (final segment in value.split('/')) {
    if (segment.isEmpty ||
        segment == '.' ||
        segment == '..' ||
        segment.contains(':') ||
        RegExp(r'[<>"|?*]').hasMatch(segment) ||
        segment.endsWith(' ') ||
        segment.endsWith('.') ||
        _isWindowsReservedName(segment)) {
      return false;
    }
  }
  return true;
}

bool _isWindowsReservedName(String segment) {
  final stem = segment.split('.').first.replaceFirst(RegExp(r'[ .]+$'), '');
  final folded = stem.toUpperCase();
  if (const {
    'CON',
    'PRN',
    'AUX',
    'NUL',
    r'CLOCK$',
    r'CONIN$',
    r'CONOUT$',
  }.contains(folded)) {
    return true;
  }
  return RegExp(r'^(?:COM|LPT)(?:[1-9¹²³])$').hasMatch(folded);
}

void _validateCanonicalLocale(String locale) {
  if (locale.isEmpty || locale.length > 35 || !_isAscii(locale)) {
    throw const FormatException('voice locale is not canonical');
  }
  final segments = locale.split('-');
  final language = segments.first;
  if (language.length < 2 ||
      language.length > 8 ||
      !RegExp(r'^[a-z]+$').hasMatch(language)) {
    throw const FormatException('voice locale language is not canonical');
  }

  // Format 1 deliberately supports the stable language[-Script][-REGION]
  // subset used by Studio language slots. Accepting arbitrary alphanumeric
  // segments here would make malformed tags such as `de-a` or `de-DE-DE`
  // appear canonical and create semantic keys that no language catalog owns.
  var index = 1;
  if (index < segments.length &&
      RegExp(r'^[A-Z][a-z]{3}$').hasMatch(segments[index])) {
    index++;
  }
  if (index < segments.length &&
      (RegExp(r'^[A-Z]{2}$').hasMatch(segments[index]) ||
          RegExp(r'^\d{3}$').hasMatch(segments[index]))) {
    index++;
  }
  if (index != segments.length) {
    throw const FormatException(
      'voice locale must use canonical language[-Script][-REGION] form',
    );
  }
}

class VoiceEditsState {
  VoiceEditsState({Map<(String, String), VoiceArchiveEdit> items = const {}})
    : items = Map.unmodifiable(items);

  const VoiceEditsState.empty() : items = const {};

  /// Case-folded semantic loc ID + canonical locale, in authored order.
  final Map<(String, String), VoiceArchiveEdit> items;

  int get count => items.length;
  List<VoiceArchiveEdit> get entries => items.values.toList(growable: false);

  VoiceEditsState copyWith({Map<(String, String), VoiceArchiveEdit>? items}) =>
      VoiceEditsState(items: items ?? this.items);
}

class VoiceEditsNotifier extends StateNotifier<VoiceEditsState> {
  VoiceEditsNotifier() : super(const VoiceEditsState.empty());

  static void validateAll(List<VoiceArchiveEdit> edits) {
    _indexEdits(edits);
  }

  void setEdit(VoiceArchiveEdit edit) {
    validateVoiceArchiveEdit(edit);
    final items = Map<(String, String), VoiceArchiveEdit>.from(state.items);
    for (final entry in items.entries) {
      if (entry.key != edit.semanticKey &&
          entry.value.deploymentKey == edit.deploymentKey) {
        throw FormatException(
          'duplicate voice deployment target: '
          '${edit.archive}/${edit.archivePath}',
        );
      }
    }
    items[edit.semanticKey] = edit;
    state = state.copyWith(items: items);
  }

  void remove(String locId, String locale) {
    final key = (locId.toLowerCase(), locale);
    if (!state.items.containsKey(key)) return;
    final items = Map<(String, String), VoiceArchiveEdit>.from(state.items)
      ..remove(key);
    state = state.copyWith(items: items);
  }

  void clearAll() {
    if (state.items.isEmpty) return;
    state = const VoiceEditsState.empty();
  }

  void loadAll(List<VoiceArchiveEdit> edits) {
    state = VoiceEditsState(items: _indexEdits(edits));
  }
}

Map<(String, String), VoiceArchiveEdit> _indexEdits(
  List<VoiceArchiveEdit> edits,
) {
  final items = <(String, String), VoiceArchiveEdit>{};
  final targets = <(String, String), (String, String)>{};
  for (final edit in edits) {
    validateVoiceArchiveEdit(edit);
    if (items.containsKey(edit.semanticKey)) {
      throw FormatException(
        'duplicate voice edit for ${edit.locId}/${edit.locale}',
      );
    }
    final previousSemanticKey = targets[edit.deploymentKey];
    if (previousSemanticKey != null &&
        previousSemanticKey != edit.semanticKey) {
      throw FormatException(
        'duplicate voice deployment target: '
        '${edit.archive}/${edit.archivePath}',
      );
    }
    targets[edit.deploymentKey] = edit.semanticKey;
    items[edit.semanticKey] = edit;
  }
  return items;
}

final voiceEditsProvider =
    StateNotifierProvider<VoiceEditsNotifier, VoiceEditsState>(
      (ref) => VoiceEditsNotifier(),
    );
