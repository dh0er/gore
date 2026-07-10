import 'core_service.dart';

/// Typed wrappers over the gore-ffi commands for audio + unified mod build/deploy.
class ModFfi {
  ModFfi(this._core);
  final GoreCoreFfiService _core;

  Future<Map<String, Object?>> _call(String cmd, Map<String, Object?> payload) async {
    final r = await _core.execute(cmd, payload: payload);
    if (r['ok'] != true) {
      final e = r['error'];
      final msg = e is Map ? (e['message'] ?? e.toString()) : 'unknown error';
      throw ModFfiException('$cmd: $msg');
    }
    return r;
  }

  Future<List<AudioSampleInfo>> audioList(String bank, {String? key}) async {
    final payload = <String, Object?>{'bank': bank};
    if (key != null) payload['key'] = key;
    final r = await _call('audio_list', payload);
    final list = (r['samples'] as List?) ?? const [];
    return list
        .whereType<Map>()
        .map((m) => AudioSampleInfo.fromJson(m.cast<String, Object?>()))
        .toList();
  }

  /// Extract one sample to a temp .ogg; returns its path.
  Future<String> audioExtract(String bank, String sample, {String? key}) async {
    final payload = <String, Object?>{'bank': bank, 'sample': sample};
    if (key != null) payload['key'] = key;
    final r = await _call('audio_extract', payload);
    return r['ogg_path'] as String;
  }

  /// Build the unified bundle into `outDir`; returns the bundle dir.
  Future<String> modBuild(Map<String, Object?> spec, String outDir) async {
    final r = await _call('mod_build', {'out_dir': outDir, 'spec': spec});
    return r['bundle_dir'] as String;
  }

  Future<void> modDeploy(String bundleDir, String gameRoot) =>
      _call('mod_deploy', {'bundle_dir': bundleDir, 'game_root': gameRoot});

  /// Undeploy the active mod. Returns true if a deployment was actually undone, false if nothing
  /// was deployed (the FFI returns a null record in that case).
  Future<bool> modUndeploy(String gameRoot) async {
    final r = await _call('mod_undeploy', {'game_root': gameRoot});
    return r['record'] != null;
  }

  /// Load (or build, if absent/`rebuild`) the texture index. Returns {assetPath: packageIdString}.
  Future<Map<String, String>> textureIndex(String game, {bool rebuild = false}) async {
    final r = await _call('texture_index', {'game': game, 'rebuild': rebuild});
    final entries = (r['entries'] as Map).cast<String, Object?>();
    return entries.map((k, v) => MapEntry(k, v as String));
  }

  /// Extract a texture to a temp PNG; returns the FFI result map (png_path, width, height, format).
  Future<Map<String, Object?>> textureExtract(String game, {String? asset, String? packageId}) async {
    final payload = <String, Object?>{'game': game};
    if (asset != null) payload['asset'] = asset;
    if (packageId != null) payload['package_id'] = packageId;
    return _call('texture_extract', payload);
  }

  /// Auto-detect the game install via Steam; returns the exe path hint, or null.
  Future<String?> findGameExe() async {
    final r = await _call('find_game', const {});
    if (r['found'] != true) return null;
    return r['exe'] as String?;
  }

  /// List modules in a precompiled script cache: [{name, file}].
  Future<List<ScriptModuleInfo>> scriptListModules(String cache) async {
    final r = await _call('script_list_modules', {'cache': cache});
    final list = (r['modules'] as List?) ?? const [];
    return list
        .whereType<Map>()
        .map((m) => ScriptModuleInfo.fromJson(m.cast<String, Object?>()))
        .toList();
  }

  /// Emit recompilable .as source for one module.
  Future<String> scriptEmitModule(String cache, String module) async {
    final r = await _call('script_emit_module', {'cache': cache, 'module': module});
    return r['source'] as String;
  }

  /// Compile a staged .as into a 1-module mini-cache via the game; returns {mini_path, module}.
  Future<Map<String, Object?>> scriptCompile({
    required String gameDir,
    required String op,
    required String moduleName,
    required String relPath,
    required String asPath,
    required String workDir,
  }) =>
      _call('script_compile', {
        'game_dir': gameDir,
        'op': op,
        'module_name': moduleName,
        'rel_path': relPath,
        'as_path': asPath,
        'work_dir': workDir,
      });
}

class ModFfiException implements Exception {
  ModFfiException(this.message);
  final String message;
  @override
  String toString() => message;
}

class AudioSampleInfo {
  AudioSampleInfo({
    required this.index,
    required this.name,
    required this.freq,
    required this.channels,
    required this.seconds,
  });
  final int index;
  final String name;
  final int freq;
  final int channels;
  final double seconds;

  factory AudioSampleInfo.fromJson(Map<String, Object?> j) => AudioSampleInfo(
        index: (j['index'] as num).toInt(),
        name: j['name'] as String,
        freq: (j['freq'] as num).toInt(),
        channels: (j['channels'] as num).toInt(),
        seconds: (j['seconds'] as num).toDouble(),
      );
}

class ScriptModuleInfo {
  ScriptModuleInfo({required this.name, required this.file});
  final String name;
  final String file;
  factory ScriptModuleInfo.fromJson(Map<String, Object?> j) =>
      ScriptModuleInfo(name: j['name'] as String, file: (j['file'] as String?) ?? '');
}
