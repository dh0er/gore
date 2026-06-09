import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/utils/default_paths.dart';

void main() {
  test(
    'codec host candidates prefer installed app directory before dev paths',
    () {
      final candidates = codecHostPathCandidates(
        executablePath: r'C:\Program Files\goresave\goresave.exe',
        currentDirectory: r'C:\sbx\goresave\apps\goresave',
      );

      expect(
        candidates.first,
        r'C:\Program Files\goresave\goresave_g1r_codec_host.exe',
      );
      expect(
        candidates,
        contains(r'C:\sbx\goresave\target\debug\goresave_g1r_codec_host.exe'),
      );
      expect(
        candidates,
        contains(r'C:\sbx\goresave\target\release\goresave_g1r_codec_host.exe'),
      );
      // The bare current-directory candidate is omitted so an untrusted binary
      // in the working directory can't be executed as the codec host.
      expect(
        candidates,
        isNot(
          contains(
            r'C:\sbx\goresave\apps\goresave\goresave_g1r_codec_host.exe',
          ),
        ),
      );
    },
  );

  test('Steam libraryfolders vdf paths are parsed as library roots', () {
    final roots = steamLibraryRootsFromVdf(r'''
"libraryfolders"
{
  "0"
  {
    "path" "C:\\Program Files (x86)\\Steam"
  }
  "1"
  {
    "path" "D:\\SteamLibrary"
  }
}
''');

    expect(roots, [r'C:\Program Files (x86)\Steam', r'D:\SteamLibrary']);
  });

  test('game exe candidates scan normal Steam library layout', () {
    final candidates = gameExePathCandidates(
      steamLibraryRoots: [r'C:\Program Files (x86)\Steam', r'D:\SteamLibrary'],
    );

    expect(
      candidates.first,
      r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
    );
    expect(
      candidates,
      contains(
        r'D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
      ),
    );
  });

  test('Steam root defaults include common secondary library drives', () {
    final roots = steamLibraryRootsFromEnvironment(
      environment: const {},
      driveLetters: const ['D', 'E'],
    );

    expect(roots, contains(r'D:\SteamLibrary'));
    expect(roots, contains(r'E:\SteamLibrary'));
  });
}
