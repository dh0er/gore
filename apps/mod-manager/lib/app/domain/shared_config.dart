import 'dart:convert';
import 'dart:io';

import 'package:path/path.dart' as p;

import 'ui_settings.dart' show sharedDataDir;

/// Read/write the shared per-user `config.json` — the SAME file the `gore` CLI
/// and the other apps use. The contract is a flat JSON object; the only key
/// this app uses is `game_path` (snake_case, matching the Rust `Config`).
/// Unknown keys are preserved on write for forward-compatibility.
class SharedConfig {
  SharedConfig(this.file);

  /// The real config at `<shared>/config.json`.
  factory SharedConfig.defaultForPlatform({Map<String, String>? environment}) {
    final env = environment ?? Platform.environment;
    return SharedConfig(File(p.join(sharedDataDir(env), 'config.json')));
  }

  final File file;

  Map<String, Object?> _read() {
    try {
      if (!file.existsSync()) return {};
      final decoded = jsonDecode(file.readAsStringSync());
      return decoded is Map ? decoded.cast<String, Object?>() : {};
    } catch (_) {
      return {};
    }
  }

  void _write(Map<String, Object?> json) {
    file.parent.createSync(recursive: true);
    const encoder = JsonEncoder.withIndent('  ');
    file.writeAsStringSync('${encoder.convert(json)}\n');
  }

  String? gamePath() {
    final v = _read()['game_path'];
    // Trim so a blank/whitespace-only value reads as unset, matching the Rust
    // side (gore_loc::config) — else CLI and GUI disagree on whether it's set.
    return (v is String && v.trim().isNotEmpty) ? v : null;
  }

  void setGamePath(String path) {
    final json = _read();
    json['game_path'] = path;
    _write(json);
  }

  void clearGamePath() {
    final json = _read();
    json.remove('game_path');
    _write(json);
  }
}
