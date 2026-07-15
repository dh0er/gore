import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/progression_loc.dart';

void main() {
  final lang = gameLangByCode('en');
  final catalog = <String, Map<String, String>>{
    'info_diego_warumgeholfen_15_00': {'english': 'Why did you help me?'},
    'info_diego_exit_gamestart_15_00': {'english': 'I will be going.'},
    'info_stt_311_fisk_exit': {'english': 'See you, Fisk.'},
    'info_npcexit': {'english': 'The NPC leaves.'},
    'info_diego_othercamps_15_00': {
      'english': 'Tell me about the other camps.',
    },
    'info_vlk_2_dielage_15_00': {'english': "What's life like here?"},
    'quest-banditscamp_banditstrust-name': {'english': 'The Bandits Trust'},
    'quest-banditscamp_banditstrust-description': {
      'english': 'Earn the trust of the bandits.',
    },
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

    test('acronym run preserved: ChoiceNPCExit → info_npcexit', () {
      expect(
        localizedKnowledgeEntry(catalog, lang, 'ChoiceNPCExit'),
        'The NPC leaves.',
      );
    });

    test('numeric node id → null', () {
      expect(
        localizedKnowledgeEntry(catalog, lang, 'Topic_Diego_209799'),
        isNull,
      );
      expect(
        localizedKnowledgeEntry(catalog, lang, 'ChoiceDiego214558'),
        isNull,
      );
    });

    test('exact cache caption key resolves numeric node ids', () {
      expect(
        localizedKnowledgeEntry(
          catalog,
          lang,
          'Topic_Diego_209799',
          locKey: 'INFO_DIEGO_OTHERCAMPS_15_00',
        ),
        'Tell me about the other camps.',
      );
      expect(
        localizedKnowledgeEntry(
          catalog,
          lang,
          'ChoiceDiego214558',
          locKey: 'INFO_DIEGO_WARUMGEHOLFEN_15_00',
        ),
        'Why did you help me?',
      );
      expect(
        localizedKnowledgeEntry(
          catalog,
          lang,
          'Info_Whatslife',
          locKey: 'Info_Vlk_2_DieLage_15_00',
        ),
        "What's life like here?",
      );
    });

    test('missing exact caption falls back to safe name heuristics', () {
      expect(
        localizedKnowledgeEntry(
          catalog,
          lang,
          'ChoiceDiegoExitGamestart',
          locKey: 'MISSING_KEY',
        ),
        'I will be going.',
      );
    });

    test('cache literal works without an extracted localization catalog', () {
      expect(
        localizedKnowledgeEntry(
          const {},
          lang,
          'ChoiceAsghan144609',
          caption: '[Forced Conversation]',
        ),
        '[Forced Conversation]',
      );
    });

    test('empty catalog → null', () {
      expect(
        localizedKnowledgeEntry(const {}, lang, 'ChoiceDiegoExitGamestart'),
        isNull,
      );
    });
  });

  group('knowledgeEntryType', () {
    test('classifies every known dialog-knowledge prefix', () {
      expect(knowledgeEntryType('ChoiceDiegoHello'), KnowledgeEntryType.choice);
      expect(knowledgeEntryType('Info_Diego_Hello'), KnowledgeEntryType.info);
      expect(
        knowledgeEntryType('Voiceline_info_diego'),
        KnowledgeEntryType.voiceLine,
      );
      expect(
        knowledgeEntryType('Topic_Diego_209799'),
        KnowledgeEntryType.topic,
      );
      expect(
        knowledgeEntryType('UnclassifiedKnowledge'),
        KnowledgeEntryType.other,
      );
    });

    test('prefers catalog category metadata over an opaque entry id', () {
      expect(
        knowledgeEntryType('GeneratedNode209799', catalogCategory: 'choice'),
        KnowledgeEntryType.choice,
      );
      expect(
        knowledgeEntryType('GeneratedNode209799', catalogCategory: 'info'),
        KnowledgeEntryType.info,
      );
      expect(
        knowledgeEntryType('GeneratedNode209799', catalogCategory: 'topic'),
        KnowledgeEntryType.topic,
      );
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

  group('localizedQuestDescription', () {
    test('quest class id → quest-<body>-description', () {
      expect(
        localizedQuestDescription(
          catalog,
          lang,
          'Quest_BanditsCamp_BANDITSTRUST',
        ),
        'Earn the trust of the bandits.',
      );
    });

    test('unknown or empty quest → null', () {
      expect(localizedQuestDescription(catalog, lang, 'Quest_Nope'), isNull);
      expect(localizedQuestDescription(catalog, lang, ''), isNull);
    });
  });

  group('readableKnowledgeEntry', () {
    test('humanizes structured ids without exposing opaque hashes', () {
      expect(readableKnowledgeEntry('Info_Whatslife'), 'Info Whatslife');
      expect(readableKnowledgeEntry('ChoiceDiego214558'), 'Dialog choice');
      expect(readableKnowledgeEntry('Topic_Jan_148468'), 'Dialog topic');
    });

    test('removes voiceline wrapper and localization suffix', () {
      expect(
        readableKnowledgeEntry('Voiceline_HelloWorld_AlkimiaLocalization'),
        'Hello World',
      );
    });
  });

  group('readableQuestEntry', () {
    test('humanizes camp, chapter, objective and numeric boundaries', () {
      expect(
        readableQuestEntry('Quest_OldCamp_OCCHAPTER1_BRINGLIST_OBJ_GETLIST'),
        'Old Camp OC Chapter 1 Bringlist Objective Getlist',
      );
    });

    test('preserves already readable player-facing text', () {
      expect(readableQuestEntry('Das Alte Lager'), 'Das Alte Lager');
    });
  });
}
