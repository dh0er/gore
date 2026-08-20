import '../../l10n/app_localizations.dart';
import '../domain/models.dart';

/// Human label for a mod [kind] tag (see [ModEntryMetaView.kind]). Unknown /
/// future tags fall back to the raw tag so nothing is silently blanked.
String kindLabel(AppLocalizations l10n, String kind) {
  return switch (kind) {
    'goremod' => l10n.kindGoremod,
    'foreign_triplet' => l10n.kindTriplet,
    'foreign_pak' => l10n.kindPak,
    'foreign_ue4ss' => l10n.kindUe4ss,
    'foreign_rawfile' => l10n.kindRawfile,
    'foreign_mixed' => l10n.kindMixed,
    _ => kind,
  };
}

/// The one word a player sees for a component: what part of the game it
/// touches, not which container format carried it.
///
/// Four wire kinds — `loose_pak`, `triplet`, `file_patch`, `pak_file_patch` —
/// collapse to a single "game files". They differ only in packaging and in
/// whether the file is overwritten or shadowed from an additive pak; the core
/// even points `file_patch` and `pak_file_patch` at the same destinations. Use
/// [componentKindLabel] where that mechanism is the point.
String componentPlainLabel(AppLocalizations l10n, ComponentView component) {
  if (component.rawFileTarget case final target?) {
    if (rawFileTargetLabel(l10n, target) case final label?) return label;
  }
  return switch (component.kind) {
    'loc_patch' => l10n.componentLocalization,
    'audio_patch' => l10n.componentAudio,
    'angel_script_patch' => l10n.componentAngelScript,
    'texture_patch' => l10n.componentTexture,
    'voice_archive_patch' => l10n.componentVoice,
    'ue4ss_lua' => l10n.componentKindUe4ssLua,
    'loose_pak' ||
    'triplet' ||
    'file_patch' ||
    'pak_file_patch' => l10n.componentGameFiles,
    _ => componentKindLabel(l10n, component.kind),
  };
}

/// Precise label for a deployable component wire tag, naming the exact
/// mechanism. Shown in the advanced view only — the plain view uses
/// [componentPlainLabel]. Unknown future tags stay visible as their raw value
/// so a newer core never produces a blank row.
String componentKindLabel(AppLocalizations l10n, String kind) {
  return switch (kind) {
    'loc_patch' => l10n.componentKindLocalizationPatch,
    'audio_patch' => l10n.componentKindAudioPatch,
    'angel_script_patch' => l10n.componentKindAngelScriptPatch,
    'texture_patch' => l10n.componentKindTexturePatch,
    'loose_pak' => l10n.componentKindLoosePak,
    'triplet' => l10n.componentKindTriplet,
    'ue4ss_lua' => l10n.componentKindUe4ssLua,
    'raw_file' => l10n.componentKindRawFile,
    'file_patch' => l10n.componentKindFilePatch,
    'pak_file_patch' => l10n.componentKindPakFilePatch,
    'voice_archive_patch' => l10n.componentKindVoiceArchivePatch,
    _ => kind,
  };
}

/// What a `raw_file` component actually replaces. These are wholesale swaps of
/// one game-wide file, so naming the destination ("all game text") is the only
/// label that tells a player anything; the bare component kind does not.
/// Returns null for an unrecognized destination so the caller can fall back.
String? rawFileTargetLabel(AppLocalizations l10n, RawFileTargetView target) {
  return switch (target.kind) {
    'lcache' => l10n.rawTargetGameText,
    'script_cache' => l10n.rawTargetGameScripts,
    'bank' => switch (target.bankName) {
      final name? when name.isNotEmpty => l10n.rawTargetSoundBankNamed(name),
      _ => l10n.rawTargetSoundBank,
    },
    _ => null,
  };
}

/// Human label for a conflict-analyzer wire tag. Unknown future tags fall back
/// to the raw value for forward-compatible diagnostics.
String conflictKindLabel(AppLocalizations l10n, String kind) {
  return switch (kind) {
    'loc' => l10n.conflictKindLocalization,
    'audio' => l10n.conflictKindAudio,
    'asset' => l10n.conflictKindAsset,
    'cdo' => l10n.conflictKindCdo,
    'ue4ss_unknown' => l10n.conflictKindUe4ssUnknown,
    'script_module' => l10n.conflictKindScriptModule,
    'voice_archive' => l10n.conflictKindVoiceArchive,
    'raw_file' => l10n.conflictKindRawFile,
    'loose_file' => l10n.conflictKindLooseFile,
    _ => kind,
  };
}

/// Human label for a conflict severity tag. Unknown severities fall back to the
/// raw tag.
String severityLabel(AppLocalizations l10n, String severity) {
  return switch (severity) {
    'hard' => l10n.sevHard,
    'soft' => l10n.sevSoft,
    'info' => l10n.sevInfo,
    _ => severity,
  };
}

/// Heading for the list of entries a component affects, carrying its coverage
/// grade as part of the sentence.
///
/// A grade only means something next to the thing it grades: a lone "estimated"
/// on a row reads as a verdict on the mod. `opaque` has no list at all, so its
/// text is a statement rather than a heading.
String footprintTargetsLabel(
  AppLocalizations l10n,
  FootprintCoverage coverage,
) {
  return switch (coverage) {
    FootprintCoverage.exact => l10n.footprintTargetsExact,
    FootprintCoverage.partial => l10n.footprintTargetsPartial,
    FootprintCoverage.advisory => l10n.footprintTargetsAdvisory,
    FootprintCoverage.opaque => l10n.footprintTargetsOpaque,
  };
}

/// A short chip caption summarizing one class of component, e.g. `loc 12`,
/// `AS 1`, `pak`, `tex 4`.
class ComponentChip {
  const ComponentChip(this.label);
  final String label;
}

/// Collapse a mod's component list into a compact set of chips, one per
/// component class, with the number of footprint targets that class claims
/// (omitted when a class has no meaningful count, e.g. a bare pak).
///
/// Buckets, in stable display order: loc / audio / AS (AngelScript) / tex /
/// pak / triplet / ue4ss / raw. Unknown component kinds are grouped under their
/// raw tag so a newer DLL still surfaces something.
List<ComponentChip> componentChips(
  AppLocalizations l10n,
  List<ComponentView> components,
) {
  // Preserve first-seen order of buckets while summing target counts.
  final order = <String>[];
  final counts = <String, int>{};
  // Whether a bucket should show its target count at all.
  const countable = {'loc', 'audio', 'AS', 'tex', 'voice'};

  void add(String bucket, int targets) {
    if (!counts.containsKey(bucket)) {
      order.add(bucket);
      counts[bucket] = 0;
    }
    counts[bucket] = counts[bucket]! + targets;
  }

  for (final c in components) {
    switch (c.kind) {
      case 'loc_patch':
        add('loc', c.targets.length);
      case 'audio_patch':
        add('audio', c.targets.length);
      case 'angel_script_patch':
        add('AS', c.targets.length);
      case 'texture_patch':
        add('tex', c.targets.length);
      case 'voice_archive_patch':
        add('voice', c.targets.length);
      case 'ue4ss_lua':
        add('ue4ss', c.targets.length);
      case 'loose_pak' || 'triplet' || 'file_patch' || 'pak_file_patch':
        add('files', c.targets.length);
      case 'raw_file':
        // A wholesale swap belongs to the content class it replaces.
        add(switch (c.rawFileTarget?.kind) {
          'lcache' => 'loc',
          'script_cache' => 'AS',
          'bank' => 'audio',
          _ => 'files',
        }, 0);
      default:
        add(c.kind, c.targets.length);
    }
  }

  String labelFor(String bucket) => switch (bucket) {
    'loc' => l10n.componentLocalization,
    'audio' => l10n.componentAudio,
    'AS' => l10n.componentAngelScript,
    'tex' => l10n.componentTexture,
    'voice' => l10n.componentVoice,
    'ue4ss' => l10n.componentKindUe4ssLua,
    'files' => l10n.componentGameFiles,
    _ => componentKindLabel(l10n, bucket),
  };

  return [
    for (final bucket in order)
      ComponentChip(
        countable.contains(bucket) && counts[bucket]! > 0
            ? '${labelFor(bucket)} ${counts[bucket]}'
            : labelFor(bucket),
      ),
  ];
}
