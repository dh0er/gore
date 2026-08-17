/// Fixed Mod Manager V1 preflight checks, in native wire order.
enum PreflightCheckId {
  gameRoot('game_root'),
  install('install'),
  loadout('loadout'),
  deployment('deployment'),
  installMutation('install_mutation'),
  ue4ss('ue4ss'),
  writeAccess('write_access');

  const PreflightCheckId(this.wire);
  final String wire;
}

enum PreflightStateKind {
  ok('ok'),
  problem('problem'),
  unknown('unknown'),
  notRequired('not_required'),
  unverified('unverified');

  const PreflightStateKind(this.wire);
  final String wire;

  static PreflightStateKind? fromWire(String wire) {
    for (final value in values) {
      if (value.wire == wire) return value;
    }
    return null;
  }
}

/// Known recovery hints from the native evidence snapshot.
///
/// Unknown future values deliberately parse as null and never execute an
/// action. The UI then offers only a fresh read.
enum PreflightActionKind {
  none('none'),
  selectGameRoot('select_game_root'),
  inspectPermissions('inspect_permissions'),
  removeObstruction('remove_obstruction'),
  verifyGameFiles('verify_game_files'),
  repairLibrary('repair_library'),
  repairLoadout('repair_loadout'),
  repairPreflightInputs('repair_preflight_inputs'),
  recoverDeployment('recover_deployment'),
  removeStudioDeployment('remove_studio_deployment'),
  resolveLoadoutCheck('resolve_loadout_check'),
  reviewApply('review_apply'),
  reviewReapply('review_reapply'),
  inspectDeployment('inspect_deployment'),
  closeGame('close_game'),
  waitForInstallMutation('wait_for_install_mutation'),
  recoverInstall('recover_install'),
  recoverManagerMutation('recover_manager_mutation'),
  installUe4ss('install_ue4ss'),
  verifyUe4ssProxy('verify_ue4ss_proxy'),
  verifyDuringApply('verify_during_apply'),
  runFullStatus('run_full_status');

  const PreflightActionKind(this.wire);
  final String wire;

  static PreflightActionKind? fromWire(String wire) {
    for (final value in values) {
      if (value.wire == wire) return value;
    }
    return null;
  }
}

class PreflightCheckView {
  const PreflightCheckView({
    required this.id,
    required this.rawState,
    required this.code,
    required this.rawAction,
    required this.actionToken,
    required this.detail,
    required this.items,
  });

  final PreflightCheckId id;
  final String rawState;
  final String code;
  final String rawAction;
  final String? actionToken;
  final String detail;
  final List<String> items;

  PreflightStateKind? get state => PreflightStateKind.fromWire(rawState);
  PreflightActionKind? get action {
    final parsed = PreflightActionKind.fromWire(rawAction);
    if (parsed == PreflightActionKind.recoverManagerMutation &&
        !_usableActionToken(actionToken)) {
      return null;
    }
    return parsed;
  }

  bool get needsAttention =>
      state == PreflightStateKind.problem ||
      state == PreflightStateKind.unknown ||
      state == null;

  static PreflightCheckView fromJson(Object? raw, PreflightCheckId expectedId) {
    if (raw is! Map) {
      throw const FormatException('preflight check is not an object');
    }
    final id = raw['id'];
    final state = raw['state'];
    final code = raw['code'];
    final action = raw['action'];
    final actionToken = raw['action_token'];
    final detail = raw['detail'];
    final items = raw['items'];
    if (id != expectedId.wire ||
        state is! String ||
        code is! String ||
        action is! String ||
        (actionToken != null && actionToken is! String) ||
        detail is! String ||
        items is! List ||
        items.any((item) => item is! String)) {
      throw const FormatException('preflight check has an invalid schema');
    }
    return PreflightCheckView(
      id: expectedId,
      rawState: state,
      code: code,
      rawAction: action,
      actionToken: actionToken as String?,
      detail: detail,
      items: List.unmodifiable(items.cast<String>()),
    );
  }
}

/// The token is deliberately opaque to Dart. This check only prevents an
/// absent, empty, or unexpectedly large value from enabling a mutation; Native
/// remains the sole authority that compares it with the current transaction.
bool _usableActionToken(String? token) =>
    token != null && token.isNotEmpty && token.runes.length <= 512;

class ManagerPreflightView {
  const ManagerPreflightView({required this.checks});

  final List<PreflightCheckView> checks;

  static ManagerPreflightView fromJson(Object? raw) {
    if (raw is! Map || raw['format'] != 1) {
      throw const FormatException('preflight response has an invalid format');
    }
    final rawChecks = raw['checks'];
    if (rawChecks is! List ||
        rawChecks.length != PreflightCheckId.values.length) {
      throw const FormatException('preflight response has invalid checks');
    }
    final checks = <PreflightCheckView>[
      for (var index = 0; index < PreflightCheckId.values.length; index++)
        PreflightCheckView.fromJson(
          rawChecks[index],
          PreflightCheckId.values[index],
        ),
    ];
    return ManagerPreflightView(checks: List.unmodifiable(checks));
  }

  PreflightCheckView check(PreflightCheckId id) => checks[id.index];

  /// One setup finding for the compact Home banner. Deployment is owned by the
  /// existing status dialog, while read-only write-access evidence is neutral.
  PreflightCheckView? get primarySetupFinding {
    for (final id in const [
      PreflightCheckId.gameRoot,
      PreflightCheckId.install,
      PreflightCheckId.loadout,
      PreflightCheckId.installMutation,
      PreflightCheckId.ue4ss,
    ]) {
      final candidate = check(id);
      if (candidate.needsAttention) return candidate;
    }
    return null;
  }
}
