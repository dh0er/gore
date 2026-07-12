import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:path/path.dart' as p;

import '../../core/mod_ffi.dart';
import 'story_workspace_bootstrap.dart';

const _executableName = 'G1R-Win64-Shipping.exe';
const _shippingCacheName = 'PrecompiledScript_Shipping.Cache';
const _bindsCacheName = 'Binds.Cache';

enum StoryWorkspaceLaunchError {
  invalidConfiguredGame,
  ambiguousGameRoot,
  missingExecutable,
  missingShippingCache,
  missingBindsCache,
  unsafeFileType,
  ambiguousPristineCache,
  invalidWorkspace,
  pathInspectionFailed,
  catalogBuildFailed,
  generationChanged,
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

/// Exact installed-generation paths passed to the trusted native catalog
/// builder. The Shipping path remains the live cache unless a future native
/// boundary can prove backup provenance with gore-mod's complete semantics.
final class StoryWorkspaceGameInputs {
  const StoryWorkspaceGameInputs._({
    required this.gameRoot,
    required this.executable,
    required this.shippingCache,
    required this.bindsCache,
  });

  final String gameRoot;
  final String executable;
  final String shippingCache;
  final String bindsCache;
}

final class StoryWorkspaceLaunch {
  const StoryWorkspaceLaunch._({required this.inputs, required this.workspace});

  final StoryWorkspaceGameInputs inputs;
  final StoryWorkspaceHandle workspace;

  bool get isClosed => workspace.isClosed;

  Future<void> close() => workspace.close();
}

/// Resolves and independently seals one installed generation, invokes the
/// trusted native catalog builder, then re-resolves and re-seals every input
/// immediately before creating/opening a production managed Story workspace.
/// It performs no deployment, game launch, or runtime qualification.
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
    final scriptDirectory = p.join(g1r, 'Script');
    final liveCache = p.join(scriptDirectory, _shippingCacheName);
    final bindsCache = p.join(scriptDirectory, _bindsCacheName);
    await _requireRegularFile(
      executable,
      missing: StoryWorkspaceLaunchError.missingExecutable,
    );
    await _requireRegularFile(
      liveCache,
      missing: StoryWorkspaceLaunchError.missingShippingCache,
    );
    await _requireRegularFile(
      bindsCache,
      missing: StoryWorkspaceLaunchError.missingBindsCache,
    );
    final shippingCache = await _selectShippingCache(liveCache);
    return StoryWorkspaceGameInputs._(
      gameRoot: root,
      executable: executable,
      shippingCache: shippingCache,
      bindsCache: bindsCache,
    );
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
    final before = await _measureGenerationSafely(inputs);
    final AuthoringStoryCatalogSelections catalog;
    try {
      catalog = await _ffi.authoringStoryCatalogV1BuildAndRead(
        executable: inputs.executable,
        shippingCache: inputs.shippingCache,
        bindsCache: inputs.bindsCache,
      );
    } catch (_) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.catalogBuildFailed,
        'The trusted Story catalog could not be built for this game generation.',
      );
    }
    _requireCatalogGeneration(catalog.generation, before);

    // Re-run path/backup choice and content sealing after native catalog
    // construction. No game file is read after this point.
    final revalidated = await resolveGameInputs(configuredGamePath);
    if (!_sameInputs(inputs, revalidated)) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.generationChanged,
        'The installed game generation changed while the Story workspace was opening.',
      );
    }
    final after = await _measureGenerationSafely(revalidated);
    if (before != after) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.generationChanged,
        'The installed game generation changed while the Story workspace was opening.',
      );
    }
    _requireCatalogGeneration(catalog.generation, after);

    StoryWorkspaceHandle? handle;
    try {
      handle = await bootstrap(catalog);
      return StoryWorkspaceLaunch._(inputs: revalidated, workspace: handle);
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

  Future<_MeasuredGeneration> _measureGenerationSafely(
    StoryWorkspaceGameInputs inputs,
  ) async {
    try {
      return await _measureGeneration(inputs);
    } on StoryWorkspaceLaunchException {
      rethrow;
    } catch (_) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.pathInspectionFailed,
        'The game-generation files could not be verified safely.',
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

  Future<String> _selectShippingCache(String live) async {
    final backup = '$live.gore-bak';
    final backupType = await FileSystemEntity.type(backup, followLinks: false);
    if (backupType == FileSystemEntityType.notFound) return live;
    if (backupType != FileSystemEntityType.file) {
      throw const StoryWorkspaceLaunchException(
        StoryWorkspaceLaunchError.unsafeFileType,
        'The Shipping cache backup is not a safe regular file.',
      );
    }
    await _requireRegularFile(
      backup,
      missing: StoryWorkspaceLaunchError.ambiguousPristineCache,
    );
    final liveSeal = await _measureFile(live);
    final backupSeal = await _measureFile(backup);
    if (liveSeal == backupSeal) return live;

    // A JSON subset cannot reproduce gore-mod's complete authenticated record,
    // ownership, drift, recovery, and path validation. Until a narrow native
    // boundary exists, never guess between divergent live and backup bytes.
    throw const StoryWorkspaceLaunchException(
      StoryWorkspaceLaunchError.ambiguousPristineCache,
      'The live and backup Shipping caches differ, so pristine provenance cannot be proven.',
    );
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
      StoryWorkspaceLaunchError.missingShippingCache =>
        'The Shipping script cache is missing.',
      StoryWorkspaceLaunchError.missingBindsCache =>
        'The Binds cache is missing.',
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

Future<_MeasuredGeneration> _measureGeneration(
  StoryWorkspaceGameInputs inputs,
) async => _MeasuredGeneration(
  executable: await _measureFile(inputs.executable),
  shippingCache: await _measureFile(inputs.shippingCache),
  bindsCache: await _measureFile(inputs.bindsCache),
);

Future<_MeasuredSeal> _measureFile(String path) async {
  await _requireRegularFile(
    path,
    missing: StoryWorkspaceLaunchError.generationChanged,
  );
  final file = File(path);
  final before = await file.stat();
  final digest = (await crypto.sha256.bind(file.openRead()).single).toString();
  final after = await file.stat();
  await _requireRegularFile(
    path,
    missing: StoryWorkspaceLaunchError.generationChanged,
  );
  if (before.type != FileSystemEntityType.file ||
      after.type != FileSystemEntityType.file ||
      before.size <= 0 ||
      after.size != before.size ||
      after.modified != before.modified) {
    throw const StoryWorkspaceLaunchException(
      StoryWorkspaceLaunchError.generationChanged,
      'A game-generation file changed while it was being verified.',
    );
  }
  return _MeasuredSeal(byteLength: before.size, sha256: digest);
}

void _requireCatalogGeneration(
  AuthoringStoryCatalogGeneration catalog,
  _MeasuredGeneration measured,
) {
  if (!_sameSeal(catalog.executable, measured.executable) ||
      !_sameSeal(catalog.shippingCache, measured.shippingCache) ||
      !_sameSeal(catalog.bindsCache, measured.bindsCache)) {
    throw const StoryWorkspaceLaunchException(
      StoryWorkspaceLaunchError.generationChanged,
      'The trusted Story catalog does not match the verified game generation.',
    );
  }
}

bool _sameSeal(AuthoringDraftContentSeal catalog, _MeasuredSeal measured) =>
    catalog.byteLength == measured.byteLength &&
    catalog.sha256 == measured.sha256;

bool _sameInputs(
  StoryWorkspaceGameInputs left,
  StoryWorkspaceGameInputs right,
) =>
    p.equals(left.gameRoot, right.gameRoot) &&
    p.equals(left.executable, right.executable) &&
    p.equals(left.shippingCache, right.shippingCache) &&
    p.equals(left.bindsCache, right.bindsCache);

final class _MeasuredGeneration {
  const _MeasuredGeneration({
    required this.executable,
    required this.shippingCache,
    required this.bindsCache,
  });

  final _MeasuredSeal executable;
  final _MeasuredSeal shippingCache;
  final _MeasuredSeal bindsCache;

  @override
  bool operator ==(Object other) =>
      other is _MeasuredGeneration &&
      executable == other.executable &&
      shippingCache == other.shippingCache &&
      bindsCache == other.bindsCache;

  @override
  int get hashCode => Object.hash(executable, shippingCache, bindsCache);
}

final class _MeasuredSeal {
  const _MeasuredSeal({required this.byteLength, required this.sha256});

  final int byteLength;
  final String sha256;

  @override
  bool operator ==(Object other) =>
      other is _MeasuredSeal &&
      byteLength == other.byteLength &&
      sha256 == other.sha256;

  @override
  int get hashCode => Object.hash(byteLength, sha256);
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
