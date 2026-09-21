import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Returns the game's one-based profile-slot label.
///
/// The numeric `m_ProfileName` identifies the game-facing slot and can differ
/// from the stable internal id used to read and write that profile.
String localizedProfileDisplayName(
  AppLocalizations l10n,
  ProfileSummary profile,
) {
  return l10n.defaultProfileName(profile.displayNumber);
}
