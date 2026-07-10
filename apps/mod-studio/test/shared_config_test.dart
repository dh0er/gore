import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:path/path.dart' as p;
import 'package:gore_mod/app/domain/shared_config.dart';

void main() {
  test('write then read round-trips game_path', () {
    final dir = Directory.systemTemp.createTempSync('gore_cfg');
    addTearDown(() => dir.deleteSync(recursive: true));
    final file = File(p.join(dir.path, 'config.json'));
    final cfg = SharedConfig(file);
    cfg.setGamePath('D:/Games/G1R');
    expect(cfg.gamePath(), 'D:/Games/G1R');
    final json = jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
    expect(json['game_path'], 'D:/Games/G1R');
  });

  test('missing file reads null', () {
    final dir = Directory.systemTemp.createTempSync('gore_cfg');
    addTearDown(() => dir.deleteSync(recursive: true));
    final cfg = SharedConfig(File(p.join(dir.path, 'config.json')));
    expect(cfg.gamePath(), isNull);
  });

  test('unknown keys are preserved on write', () {
    final dir = Directory.systemTemp.createTempSync('gore_cfg');
    addTearDown(() => dir.deleteSync(recursive: true));
    final file = File(p.join(dir.path, 'config.json'))
      ..writeAsStringSync('{"game_path":"x","future":1}');
    SharedConfig(file).setGamePath('y');
    final json = jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
    expect(json['game_path'], 'y');
    expect(json['future'], 1);
  });
}
