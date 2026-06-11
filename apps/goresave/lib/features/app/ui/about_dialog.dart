import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

const _githubUrl = 'https://github.com/dh0er/goresave';

const String gitSha = String.fromEnvironment('GIT_SHA', defaultValue: 'dev');

String aboutVersionLabel(PackageInfo? info) =>
    info == null ? '' : 'Version ${info.version} ($gitSha)';

class GoresaveAboutDialog extends StatefulWidget {
  const GoresaveAboutDialog({super.key});

  @override
  State<GoresaveAboutDialog> createState() => _GoresaveAboutDialogState();
}

class _GoresaveAboutDialogState extends State<GoresaveAboutDialog> {
  final Future<PackageInfo> _packageInfo = PackageInfo.fromPlatform();

  @override
  Widget build(BuildContext context) {
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
                'goresave',
                style: textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(height: 4),
              FutureBuilder<PackageInfo>(
                future: _packageInfo,
                builder: (context, snapshot) {
                  final version = aboutVersionLabel(snapshot.data);
                  return Text(version, style: textTheme.bodySmall);
                },
              ),
              const SizedBox(height: 16),
              Text(
                'Gothic Remake Savegame Editor',
                textAlign: TextAlign.center,
                style: textTheme.bodyMedium,
              ),
              const SizedBox(height: 8),
              TextButton.icon(
                icon: const Icon(Icons.open_in_new, size: 16),
                label: const Text(_githubUrl),
                onPressed: () => launchUrl(Uri.parse(_githubUrl)),
              ),
              const SizedBox(height: 12),
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('Close'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
