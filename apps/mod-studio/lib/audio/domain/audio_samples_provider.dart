import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/mod_ffi.dart';
import '../../core/providers.dart';

/// Lazily lists the samples inside one FMOD bank, keyed by the bank's full path.
///
/// Loading can be large (SFX has ~7000 samples) so this is loaded per selected
/// bank and cached by Riverpod for the lifetime of the provider scope.
final audioSamplesProvider =
    FutureProvider.family<List<AudioSampleInfo>, String>((ref, bankFullPath) {
  final ffi = ModFfi(ref.watch(coreServiceProvider));
  return ffi.audioList(bankFullPath);
});
