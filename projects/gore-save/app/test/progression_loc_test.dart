import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/progression_loc.dart';

void main() {
  final lang = gameLangByCode('en');
  final catalog = <String, Map<String, String>>{
    'info_diego_warumgeholfen_15_00': {'english': 'Why did you help me?'},
    'info_diego_exit_gamestart_15_00': {'english': 'I will be going.'},
    'info_stt_311_fisk_exit': {'english': 'See you, Fisk.'},
    'quest-banditscamp_banditstrust-name': {'english': 'The Bandits Trust'},
    'quest-banditscamp_banditstrust_banditstrust_obj_back-name': {
      'english': 'Go back',
    },
  };

  group('localizedKnowledgeEntry', () {
    test('Voiceline wrapper → exact inner id', () {
      expect(
        localizedKnowledgeEntry(
          catalog,
          lang,
          'Voiceline_info_diego_warumgeholfen_15_00_AlkimiaLocalization',
        ),
        'Why did you help me?',
      );
    });

    test('Choice CamelCase → info_<snake> via variant prefix', () {
      expect(
        localizedKnowledgeEntry(catalog, lang, 'ChoiceDiegoExitGamestart'),
        'I will be going.',
      );
    });

    test('letter/digit split: Stt311 → stt_311', () {
      expect(
        localizedKnowledgeEntry(catalog, lang, 'ChoiceStt311FiskExit'),
        'See you, Fisk.',
      );
    });

    test('numeric node id → null', () {
      expect(localizedKnowledgeEntry(catalog, lang, 'Topic_Diego_209799'), isNull);
      expect(localizedKnowledgeEntry(catalog, lang, 'ChoiceDiego214558'), isNull);
    });

    test('empty catalog → null', () {
      expect(localizedKnowledgeEntry(const {}, lang, 'ChoiceDiegoExitGamestart'),
          isNull);
    });
  });

  group('localizedQuestName', () {
    test('quest class id → quest-<body>-name', () {
      expect(
        localizedQuestName(catalog, lang, 'Quest_BanditsCamp_BANDITSTRUST'),
        'The Bandits Trust',
      );
    });

    test('objective quest id', () {
      expect(
        localizedQuestName(
          catalog,
          lang,
          'Quest_BanditsCamp_BANDITSTRUST_BANDITSTRUST_OBJ_BACK',
        ),
        'Go back',
      );
    });

    test('unknown quest → null', () {
      expect(localizedQuestName(catalog, lang, 'Quest_Nope'), isNull);
    });
  });
}
