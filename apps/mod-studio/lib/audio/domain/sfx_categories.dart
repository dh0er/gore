import '../../l10n/app_localizations.dart';

/// Categories for SFX.bank samples, derived from the sample name's second
/// `_` token (validated against the real bank, 7218 samples).
enum SfxCategory {
  creatures,
  objects,
  magic,
  movement,
  world,
  action,
  combat,
  physics,
  items,
  ui,
  foley,
  underwater,
  vision,
  dialog,
  other;

  /// Localized category name for display, resolved via [AppLocalizations].
  String localizedLabel(AppLocalizations l10n) => switch (this) {
        creatures => l10n.audioCatCreatures,
        objects => l10n.audioCatObjects,
        magic => l10n.audioCatMagic,
        movement => l10n.audioCatMovement,
        world => l10n.audioCatWorld,
        action => l10n.audioCatAction,
        combat => l10n.audioCatCombat,
        physics => l10n.audioCatPhysics,
        items => l10n.audioCatItems,
        ui => l10n.audioCatUi,
        foley => l10n.audioCatFoley,
        underwater => l10n.audioCatUnderwater,
        vision => l10n.audioCatVision,
        dialog => l10n.audioCatDialog,
        other => l10n.audioCatOther,
      };
}

/// Maps a flat sample name like `SFX_CREA_Golem_Ice_M_Creak_Loop_L1_01` to
/// its [SfxCategory] via the second `_` token, case-folded, with the
/// singular/plural spelling variants merged.
SfxCategory sfxCategoryForSample(String name) {
  final parts = name.split('_');
  if (parts.length < 2) return SfxCategory.other;
  return switch (parts[1].toUpperCase()) {
    'CREA' => SfxCategory.creatures,
    'OBJ' || 'OBJECTS' => SfxCategory.objects,
    'MAGIC' => SfxCategory.magic,
    'MOVE' => SfxCategory.movement,
    'WORLD' => SfxCategory.world,
    'ACTION' || 'ACTIONS' => SfxCategory.action,
    'COMBAT' => SfxCategory.combat,
    'PHYSICS' => SfxCategory.physics,
    'ITEMS' => SfxCategory.items,
    'UI' => SfxCategory.ui,
    'FOLEY' => SfxCategory.foley,
    'UNDERWATER' => SfxCategory.underwater,
    'VISION' => SfxCategory.vision,
    'DIALOG' => SfxCategory.dialog,
    _ => SfxCategory.other,
  };
}
