import 'package:flutter/material.dart';

import '../../core/mgr_ffi.dart';
import '../../l10n/app_localizations.dart';

class ScriptRebuildConfirmationDialog extends StatelessWidget {
  const ScriptRebuildConfirmationDialog({
    super.key,
    required this.confirmation,
  });

  final MgrScriptRebuildConfirmation confirmation;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AlertDialog(
      key: const ValueKey('script-rebuild-confirmation-dialog'),
      title: Text(l10n.scriptUpdateConfirmationTitle),
      scrollable: true,
      content: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(l10n.scriptUpdateConfirmationBody),
          const SizedBox(height: 16),
          for (final warning in confirmation.warnings) ...[
            Text(
              l10n.scriptUpdateConfirmationModule(
                warning.modName,
                warning.module,
              ),
              style: Theme.of(context).textTheme.titleSmall,
            ),
            Text(
              warning.reason == 'added_module_now_exists'
                  ? l10n.scriptUpdateModuleNowExists
                  : warning.currentSha256 == null
                  ? l10n.scriptUpdateModuleMissing
                  : l10n.scriptUpdateModuleChanged,
            ),
            const SizedBox(height: 12),
          ],
        ],
      ),
      actions: [
        TextButton(
          key: const ValueKey('script-rebuild-cancel'),
          autofocus: true,
          onPressed: () => Navigator.pop(context, false),
          child: Text(l10n.commonCancel),
        ),
        FilledButton(
          key: const ValueKey('script-rebuild-confirm'),
          onPressed: () => Navigator.pop(context, true),
          child: Text(l10n.scriptUpdateConfirmationAction),
        ),
      ],
    );
  }
}
