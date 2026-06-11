import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/providers/data_providers.dart';

sealed class UpdateState {
  const UpdateState();
}

/// No update staged: none available, updater disabled, or a check/download
/// failed (updates are best-effort and never block the app).
class UpdateIdle extends UpdateState {
  const UpdateIdle();
}

/// An update is downloaded and staged; restarting applies [version].
class UpdateReady extends UpdateState {
  const UpdateReady(this.version);

  final String version;
}

class UpdateNotifier extends StateNotifier<UpdateState> {
  UpdateNotifier(this._core) : super(const UpdateIdle()) {
    _checkAndDownload();
  }

  final GoresaveCoreService _core;

  Future<void> _checkAndDownload() async {
    try {
      final check = await _core.execute('update_check');
      final data = check['data'];
      if (check['ok'] != true ||
          data is! Map ||
          data['status'] != 'updateAvailable') {
        return;
      }
      final version = data['version'];
      final download = await _core.execute('update_download');
      if (download['ok'] == true && version is String && mounted) {
        state = UpdateReady(version);
      }
    } catch (error) {
      debugPrint('goresave update check failed: $error');
    }
  }

  Future<void> applyAndRestart() async {
    try {
      await _core.execute('update_apply_restart');
    } catch (error) {
      debugPrint('goresave update apply failed: $error');
    }
  }
}

final updateProvider = StateNotifierProvider<UpdateNotifier, UpdateState>(
  (ref) => UpdateNotifier(ref.watch(coreServiceProvider)),
);
