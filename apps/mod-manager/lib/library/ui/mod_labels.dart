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

/// Human label for a deployable component wire tag. Unknown future tags stay
/// visible as their raw value so a newer core never produces a blank row.
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

/// Human explanation for one derived conflict-footprint coverage grade.
String footprintCoverageLabel(
  AppLocalizations l10n,
  FootprintCoverage coverage,
) {
  return switch (coverage) {
    FootprintCoverage.exact => l10n.footprintCoverageExact,
    FootprintCoverage.partial => l10n.footprintCoveragePartial,
    FootprintCoverage.advisory => l10n.footprintCoverageAdvisory,
    FootprintCoverage.opaque => l10n.footprintCoverageOpaque,
  };
}

/// Compact localized category for the per-component badge. The adjacent
/// tooltip/semantic label uses [footprintCoverageLabel] for the full meaning.
String footprintCoverageShortLabel(
  AppLocalizations l10n,
  FootprintCoverage coverage,
) {
  return switch (coverage) {
    FootprintCoverage.exact => l10n.footprintCoverageExactLabel,
    FootprintCoverage.partial => l10n.footprintCoveragePartialLabel,
    FootprintCoverage.advisory => l10n.footprintCoverageAdvisoryLabel,
    FootprintCoverage.opaque => l10n.footprintCoverageOpaqueLabel,
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
  const countable = {'loc', 'audio', 'AS', 'tex', 'triplet'};

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
      case 'loose_pak':
        add('pak', c.targets.length);
      case 'triplet':
        add('triplet', c.targets.length);
      case 'ue4ss_lua':
        add('ue4ss', c.targets.length);
      case 'raw_file':
        add('raw', 0);
      default:
        add(c.kind, c.targets.length);
    }
  }

  String labelFor(String bucket) => switch (bucket) {
    'loc' => l10n.componentLocalization,
    'audio' => l10n.componentAudio,
    'AS' => l10n.componentAngelScript,
    'tex' => l10n.componentTexture,
    'pak' => l10n.componentKindLoosePak,
    'triplet' => l10n.componentKindTriplet,
    'ue4ss' => l10n.componentKindUe4ssLua,
    'raw' => l10n.componentKindRawFile,
    'file_patch' => l10n.componentKindFilePatch,
    'pak_file_patch' => l10n.componentKindPakFilePatch,
    'voice_archive_patch' => l10n.componentKindVoiceArchivePatch,
    _ => bucket,
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
