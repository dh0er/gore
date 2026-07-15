import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Returns a profile's user-provided name, or a localized default name when
/// the save only stores its numeric id as the name.
String localizedProfileDisplayName(
  AppLocalizations l10n,
  ProfileSummary profile,
) {
  final name = profile.profileName?.trim();
  if (name == null || name.isEmpty || name == profile.profileId.toString()) {
    return l10n.defaultProfileName(profile.profileId);
  }
  return name;
}
