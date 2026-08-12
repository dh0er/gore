import 'package:file_selector/file_selector.dart';

typedef ImportFileOpener =
    Future<XFile?> Function(List<XTypeGroup> acceptedTypeGroups);

Future<XFile?> _openImportFile(List<XTypeGroup> acceptedTypeGroups) =>
    openFile(acceptedTypeGroups: acceptedTypeGroups);

/// Injectable boundary around the two platform pickers used by Manager import.
/// A null path means the user cancelled and grants no mutation authority.
abstract interface class ImportSourcePicker {
  Future<String?> pickFolder();

  Future<String?> pickFile({required String dialogLabel});
}

class FileSelectorImportSourcePicker implements ImportSourcePicker {
  const FileSelectorImportSourcePicker({this.fileOpener = _openImportFile});

  final ImportFileOpener fileOpener;

  @override
  Future<String?> pickFolder() => getDirectoryPath();

  @override
  Future<String?> pickFile({required String dialogLabel}) async {
    // Native import is the bounded authority for supported source detection.
    // Allow every file here so it can give the same refusal path for unknown,
    // unsupported (.7z/.rar), corrupt, and incomplete sources.
    final group = XTypeGroup(label: dialogLabel);
    final file = await fileOpener([group]);
    return file?.path;
  }
}
