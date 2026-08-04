import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';

void main() {
  test('direct semantic edit failures have a path-free recovery message', () {
    final message = revision3DataAssetFriendlyError(
      const ModFfiException(
        command: 'authoring_store_prepare_revision3_dataasset_edit_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_EDIT_INVALID',
        message: r'C:\private\receipt.json did not match',
      ),
    );
    expect(message, contains('Inspect it again'));
    expect(message, isNot(contains('private')));
    expect(message, isNot(contains('receipt.json')));
  });
}
