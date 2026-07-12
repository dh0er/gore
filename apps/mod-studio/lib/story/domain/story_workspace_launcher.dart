import 'dart:io';

import 'package:path/path.dart' as p;

import '../../core/mod_ffi.dart';
import 'story_workspace_bootstrap.dart';

const _executableName = 'G1R-Win64-Shipping.exe';

enum StoryWorkspaceLaunchError {
  invalidConfiguredGame,
  ambiguousGameRoot,
  missingExecutable,
  unsafeFileType,
  invalidWorkspace,
  pathInspectionFailed,
  catalogBuildFailed,
  workspaceBootstrapFailed,
}

/// Stable public launch failure. Messages deliberately contain neither local
/// paths nor nested I/O/native parser details.
final class StoryWorkspaceLaunchException implements Exception {
  const StoryWorkspaceLaunchException(this.code, this.message);

  final StoryWorkspaceLaunchError code;
  final String message;

  @override
  String toString() => 'StoryWorkspaceLaunchException: $message';
}

/// Safe configured install root and exact Shipping executable layout.
///
/// Shipping/Binds paths and pristine-backup provenance deliberately never cross this Dart DTO;
/// native gore-mod owns those decisions from the root onward.
final class StoryWorkspaceGameInputs {
  const StoryWorkspaceGameInputs._({
    required this.gameRoot,
    required this.executable,
  });

  final String gameRoot;
  final String executable;
}

final class StoryWorkspaceLaunch {
  const StoryWorkspaceLaunch._({required this.inputs, required this.workspace});

  final StoryWorkspaceGameInputs inputs;
  final StoryWorkspaceHandle workspace;

  bool get isClosed => workspace.isClosed;

  Future<void> close() => workspace.close();
}

/// Resolves only a safe configured root/executable layout, then delegates pristine-cache
/// selection and generation sealing to native gore-mod before creating/opening a production
/// managed Story workspace. It performs no deployment, game launch, or runtime qualification.
final class StoryWorkspaceLauncher {
  const StoryWorkspaceLauncher(this._ffi);

  final ModFfi _ffi;

  Future<StoryWorkspaceLaunch> create({
    required String configuredGamePath,
    required Directory workspaceRoot,
    required StoryProjectMetadata metadata,
    StoryProjectIdSource? projectIdSource,
  }) => _launch(
    configuredGamePath: configuredGamePath,
    workspaceRoot: workspaceRoot,
    bootstrap: (catalog) => StoryWorkspaceBootstrap.create(
      root: workspaceRoot,
      ffi: _ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.production,
      metadata: metadata,
      projectIdSource: projectIdSource,
    ),
  );

  Future<StoryWorkspaceLaunch> open({
    required String configuredGamePath,
    required Directory workspaceRoot,
  }) => _launch(
    configuredGamePath: configuredGamePath,
    workspaceRoot: workspaceRoot,
    bootstrap: (catalog) => StoryWorkspaceBootstrap.open(
      root: workspaceRoot,
      ffi: _ffi,
      catalogSelections: catalog,
      profile: AuthoringValidationProfile.production,
    ),
  );

  Future<StoryWorkspaceGameInputs> resolveGameInputs(
    String configuredGamePath,
  ) async {
    try {
      return await _resolveGameInputs(configuredGamePath);
    } on StoryWorkspaceLaunchException {
      rethrow;
    } catch (_) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.pathInspectionFailed,
        'The configured game files could not be inspected safely.',
      );
    }
  }

  Future<StoryWorkspaceGameInputs> _resolveGameInputs(
    String configuredGamePath,
  ) async {
    final configured = _absolute(
      configuredGamePath,
      error: StoryWorkspaceLaunchError.invalidConfiguredGame,
    );
    final configuredType = await FileSystemEntity.type(
      configured,
      followLinks: false,
    );
    late final String root;
    switch (configuredType) {
      case FileSystemEntityType.file:
        await _requireRegularFile(
          configured,
          missing: StoryWorkspaceLaunchError.invalidConfiguredGame,
        );
        root = _rootFromExactExecutable(configured);
      case FileSystemEntityType.directory:
        await _requireDirectoryChain(
          configured,
          error: StoryWorkspaceLaunchError.invalidConfiguredGame,
        );
        root = await _rootFromDirectory(configured);
      case FileSystemEntityType.notFound:
        throw const StoryWorkspaceLaunchException(
          StoryWorkspaceLaunchError.invalidConfiguredGame,
          'The configured game path does not exist.',
        );
      case FileSystemEntityType.link:
      case FileSystemEntityType.pipe:
      case FileSystemEntityType.unixDomainSock:
        throw const StoryWorkspaceLaunchException(
          StoryWorkspaceLaunchError.unsafeFileType,
          'The configured game path is not a safe regular file or directory.',
        );
    }

    final g1r = p.join(root, 'G1R');
    await _requireDirectoryChain(
      g1r,
      error: StoryWorkspaceLaunchError.invalidConfiguredGame,
    );
    final executable = p.join(g1r, 'Binaries', 'Win64', _executableName);
    await _requireRegularFile(
      executable,
      missing: StoryWorkspaceLaunchError.missingExecutable,
    );
    return StoryWorkspaceGameInputs._(gameRoot: root, executable: executable);
  }

  Future<StoryWorkspaceLaunch> _launch({
    required String configuredGamePath,
    required Directory workspaceRoot,
    required Future<StoryWorkspaceHandle> Function(
      AuthoringStoryCatalogSelections catalog,
    )
    bootstrap,
  }) async {
    try {
      final workspacePath = _absolute(
        workspaceRoot.path,
        error: StoryWorkspaceLaunchError.invalidWorkspace,
      );
      await _requireDirectoryChain(
        workspacePath,
        error: StoryWorkspaceLaunchError.invalidWorkspace,
      );
    } on StoryWorkspaceLaunchException {
      rethrow;
    } catch (_) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.pathInspectionFailed,
        'The managed Story workspace could not be inspected safely.',
      );
    }

    final inputs = await resolveGameInputs(configuredGamePath);
    final AuthoringStoryCatalogSelections catalog;
    try {
      catalog = await _ffi.authoringStoryCatalogV1BuildAndReadForGameRoot(
        gameRoot: inputs.gameRoot,
      );
    } catch (_) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.catalogBuildFailed,
        'The trusted Story catalog could not be built for this game generation.',
      );
    }

    StoryWorkspaceHandle? handle;
    try {
      handle = await bootstrap(catalog);
      return StoryWorkspaceLaunch._(inputs: inputs, workspace: handle);
    } catch (error, stackTrace) {
      if (handle != null) {
        try {
          await handle.close();
        } catch (_) {}
      }
      Error.throwWithStackTrace(
        const StoryWorkspaceLaunchException(
          StoryWorkspaceLaunchError.workspaceBootstrapFailed,
          'The managed Story workspace could not be created or opened.',
        ),
        stackTrace,
      );
    }
  }

  Future<String> _rootFromDirectory(String configured) async {
    final candidates = <String>[];
    final childG1r = p.join(configured, 'G1R');
    final childType = await FileSystemEntity.type(childG1r, followLinks: false);
    if (childType == FileSystemEntityType.directory) {
      candidates.add(configured);
    } else if (childType != FileSystemEntityType.notFound) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.unsafeFileType,
        'The G1R install directory is not a safe real directory.',
      );
    }
    if (_sameSegment(p.basename(configured), 'G1R')) {
      candidates.add(p.dirname(configured));
    }
    final unique = <String>[];
    for (final candidate in candidates) {
      if (!unique.any((existing) => p.equals(existing, candidate))) {
        unique.add(candidate);
      }
    }
    if (unique.isEmpty) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.invalidConfiguredGame,
        'The configured directory is not a G1R install root.',
      );
    }
    if (unique.length != 1) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.ambiguousGameRoot,
        'The configured directory resolves to multiple G1R install roots.',
      );
    }
    return unique.single;
  }

  String _rootFromExactExecutable(String configured) {
    final win64 = p.dirname(configured);
    final binaries = p.dirname(win64);
    final g1r = p.dirname(binaries);
    final root = p.dirname(g1r);
    final expected = p.join(root, 'G1R', 'Binaries', 'Win64', _executableName);
    if (!p.equals(configured, expected)) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.invalidConfiguredGame,
        'The configured executable is not the exact G1R Shipping executable.',
      );
    }
    return root;
  }
}

Future<void> _requireRegularFile(
  String path, {
  required StoryWorkspaceLaunchError missing,
}) async {
  await _requireDirectoryAncestors(path);
  final type = await FileSystemEntity.type(path, followLinks: false);
  if (type == FileSystemEntityType.notFound) {
    throw StoryWorkspaceLaunchException(missing, switch (missing) {
      StoryWorkspaceLaunchError.missingExecutable =>
        'The exact G1R Shipping executable is missing.',
      _ => 'A required regular file is missing.',
    });
  }
  if (type != FileSystemEntityType.file) {
    throw const StoryWorkspaceLaunchException(
      StoryWorkspaceLaunchError.unsafeFileType,
      'A required game file is not a safe regular file.',
    );
  }
}

Future<void> _requireDirectoryChain(
  String path, {
  required StoryWorkspaceLaunchError error,
}) async {
  await _requireDirectoryAncestors(path);
  if (await FileSystemEntity.type(path, followLinks: false) !=
      FileSystemEntityType.directory) {
    throw StoryWorkspaceLaunchException(
      error,
      error == StoryWorkspaceLaunchError.invalidWorkspace
          ? 'The managed Story workspace must be an existing safe directory.'
          : 'The configured G1R directory is missing or unsafe.',
    );
  }
}

Future<void> _requireDirectoryAncestors(String path) async {
  var current = p.dirname(path);
  while (true) {
    if (await FileSystemEntity.type(current, followLinks: false) !=
        FileSystemEntityType.directory) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.unsafeFileType,
        'A path ancestor is missing, linked, or otherwise unsafe.',
      );
    }
    final parent = p.dirname(current);
    if (p.equals(parent, current)) return;
    current = parent;
  }
}

String _absolute(String value, {required StoryWorkspaceLaunchError error}) {
  if (value.isEmpty || value.contains('\u0000')) {
    throw StoryWorkspaceLaunchException(
      error,
      error == StoryWorkspaceLaunchError.invalidWorkspace
          ? 'The managed Story workspace path is invalid.'
          : 'The configured game path is invalid.',
    );
  }
  return p.normalize(p.absolute(value));
}

bool _sameSegment(String left, String right) => Platform.isWindows
    ? left.toLowerCase() == right.toLowerCase()
    : left == right;
