Map<String, Object?> scriptUpdateDetails({
  String token = 'exact-update-token',
}) => {
  'token': token,
  'warnings': [
    <String, Object?>{
      'mod_id': 'dialog-mod',
      'mod_name': 'Diego Dialog',
      'module': 'NPC/Diego',
      'reason': 'vanilla_module_changed',
      'original_sha256': List.filled(64, 'a').join(),
      'current_sha256': List.filled(64, 'b').join(),
    },
    <String, Object?>{
      'mod_id': 'quest-mod',
      'mod_name': 'Old Quest',
      'module': 'Quests/OldQuest',
      'reason': 'vanilla_module_removed',
      'original_sha256': List.filled(64, 'c').join(),
      'current_sha256': null,
    },
  ],
};

Map<String, Object?> scriptUpdateRefusal({
  String token = 'exact-update-token',
}) => {
  'ok': false,
  'error': {
    'code': 'SCRIPT_REBUILD_CONFIRMATION_REQUIRED',
    'message': 'Native warning requiring explicit approval',
    'details': scriptUpdateDetails(token: token),
  },
};
