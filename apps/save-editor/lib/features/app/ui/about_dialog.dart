import 'package:flutter/material.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

const _githubUrl = 'https://github.com/dh0er/goresave';

const String _gitSha = String.fromEnvironment('GIT_SHA', defaultValue: 'dev');

const aboutCopyrightNotice = '© 2026 goresave contributors';
const aboutLicenseNotice = 'Licensed under the MIT License.';

String aboutVersionLabel(PackageInfo? info) =>
    info == null ? '' : 'Version ${info.version} ($_gitSha)';

class GoresaveAboutDialog extends StatefulWidget {
  const GoresaveAboutDialog({super.key});

  @override
  State<GoresaveAboutDialog> createState() => _GoresaveAboutDialogState();
}

class _GoresaveAboutDialogState extends State<GoresaveAboutDialog> {
  final Future<PackageInfo> _packageInfo = PackageInfo.fromPlatform();

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Dialog(
      child: SizedBox(
        width: 420,
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Image.asset('assets/goresave_icon.png', height: 120),
              const SizedBox(height: 12),
              Text(
                'GORE Save Editor',
                style: textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(height: 4),
              FutureBuilder<PackageInfo>(
                future: _packageInfo,
                builder: (context, snapshot) {
                  final info = snapshot.data;
                  final version =
                      info == null ? '' : l10n.aboutVersion(info.version, _gitSha);
                  return Text(version, style: textTheme.bodySmall);
                },
              ),
              const SizedBox(height: 16),
              TextButton.icon(
                icon: const Icon(Icons.open_in_new, size: 16),
                label: const Text(_githubUrl),
                onPressed: () => launchUrl(Uri.parse(_githubUrl)),
              ),
              const SizedBox(height: 12),
              Text(
                l10n.aboutCopyright,
                textAlign: TextAlign.center,
                style: textTheme.bodySmall,
              ),
              Text(
                l10n.aboutLicense,
                textAlign: TextAlign.center,
                style: textTheme.bodySmall,
              ),
              const SizedBox(height: 12),
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.close),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
