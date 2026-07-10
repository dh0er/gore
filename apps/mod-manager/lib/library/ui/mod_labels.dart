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
List<ComponentChip> componentChips(List<ComponentView> components) {
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

  return [
    for (final bucket in order)
      ComponentChip(
        countable.contains(bucket) && counts[bucket]! > 0
            ? '$bucket ${counts[bucket]}'
            : bucket,
      ),
  ];
}
