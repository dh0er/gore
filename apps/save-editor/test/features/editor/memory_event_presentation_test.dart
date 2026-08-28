import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';
import 'package:goresave/features/editor/domain/memory_event_presentation.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_de.dart';
import 'package:goresave/l10n/app_localizations_en.dart';

void main() {
  group('MemoryEvent parsing', () {
    test('normalizes Unreal unset sentinels before presentation', () {
      final event = MemoryEvent.fromJson({
        'index': 7,
        'tags': ['None', '  Memory.Quest.Started  ', ''],
        'magnitude': -1.7976931348623157e308,
        'timeSeconds': -1.7976931348623157e308,
        'durationSeconds': double.infinity,
        'instigator': 'None',
        'affected': '  ',
        'optionalClass1': 'NONE',
        'optionalClass2': null,
      });

      expect(event.tags, ['Memory.Quest.Started']);
      expect(event.magnitude, isNull);
      expect(event.timeSeconds, isNull);
      expect(event.durationSeconds, isNull);
      expect(event.instigator, isNull);
      expect(event.affected, isNull);
      expect(event.optionalClass1, isNull);
      expect(event.optionalClass2, isNull);
    });

    test('reads bounded position and dynamic payload details', () {
      final event = MemoryEvent.fromJson({
        'index': 11,
        'tags': ['Memory.StoryEvent'],
        'position': {'x': 1.5, 'y': 2, 'z': -3},
        'payload': {
          'type': '/Script/G1R.StoryEventPayload',
          'fieldCount': 2,
          'truncated': true,
          'fields': [
            {
              'name': 'EventName',
              'type': 'NameProperty',
              'value': 'MudVoiceline1',
            },
            {
              'name': 'Context',
              'type': 'MapProperty',
              'value': {
                'count': 1,
                'entries': [
                  {'key': 'Actor', 'value': 'Mud'},
                ],
              },
            },
          ],
        },
      });

      expect(event.position?.x, 1.5);
      expect(event.position?.z, -3);
      expect(event.payload?.fieldCount, 2);
      expect(event.payload?.truncated, isTrue);
      expect(event.payload?.valueFor('eventname'), 'MudVoiceline1');
    });
  });

  group('MemoryEventPresenter', () {
    test(
      'recognizes the supported event vocabulary including save variants',
      () {
        final presenter = _presenter(AppLocalizationsEn());
        const cases = <String, MemoryEventKind>{
          'Memory.Quest.Started': MemoryEventKind.questStarted,
          'Memory.Quest.Succeeded': MemoryEventKind.questSucceeded,
          'Memory.Quest.Failed': MemoryEventKind.questFailed,
          'Memory.Document.Read': MemoryEventKind.documentRead,
          'Memory.Document.SegmentUnlocked':
              MemoryEventKind.documentSegmentUnlocked,
          'Memory.Document.SegmentViewed':
              MemoryEventKind.documentSegmentViewed,
          'Memory.Chapter.Completed': MemoryEventKind.chapterCompleted,
          'Memory.Area.Enter': MemoryEventKind.areaEntered,
          'Memory.Area.Leave': MemoryEventKind.areaLeft,
          'Memory.Character.Defeated.Kill': MemoryEventKind.characterKilled,
          'Memory.Character.Defeated': MemoryEventKind.characterDefeated,
          'Memory.WasDefeated': MemoryEventKind.characterDefeated,
          'Memory.Combat.WasDefeated': MemoryEventKind.characterDefeated,
          'Memory.SaveAndLoad.Defeated': MemoryEventKind.characterDefeated,
          'Memory.Combat.Dodge': MemoryEventKind.combatDodge,
          'Memory.Character.Debuffed': MemoryEventKind.characterDebuffed,
          'Memory.Character.Can.Trade': MemoryEventKind.tradeAvailable,
          'Memory.Item.Obtained': MemoryEventKind.itemObtained,
          'Memory.Item.Crafted': MemoryEventKind.itemCrafted,
          'Memory.Skill.Learned': MemoryEventKind.skillStateRecorded,
          'Memory.Learned.Skill': MemoryEventKind.skillStateRecorded,
          'Memory.Recipe.Learned': MemoryEventKind.recipeLearned,
          'Memory.Learned.Recipe': MemoryEventKind.recipeLearned,
          'Memory.Guild.Joined': MemoryEventKind.guildJoined,
          'Memory.Crime.Lockpick': MemoryEventKind.crimeRecorded,
          'Memory.Sleep': MemoryEventKind.slept,
          'Memory.StoryEvent': MemoryEventKind.storyEvent,
        };

        for (final MapEntry(key: tag, value: expected) in cases.entries) {
          expect(
            presenter.present(MemoryEvent(index: 0, tags: [tag])).kind,
            expected,
            reason: tag,
          );
        }
      },
    );

    test('categorizes unknown detailed tags by their stable domain prefix', () {
      final presenter = _presenter(AppLocalizationsEn());
      const cases = <String, MemoryEventCategory>{
        'Memory.Combat.Headshot': MemoryEventCategory.combat,
        'Memory.Character.Debuff': MemoryEventCategory.combat,
        'Memory.Character.Can.Trade': MemoryEventCategory.social,
        'Memory.Character.StartInteracting.Sleeping': MemoryEventCategory.rest,
        'Memory.Character.StartInteracting.Crafting': MemoryEventCategory.item,
        'Memory.Document.Bookmarked': MemoryEventCategory.document,
        'Memory.Quest.ObjectiveUpdated': MemoryEventCategory.quest,
        'Memory.Area.FastTravel': MemoryEventCategory.exploration,
        'Memory.LTM.Items.Consumed': MemoryEventCategory.item,
        'Memory.Learned.Language': MemoryEventCategory.learning,
        'Memory.Crime.Reported': MemoryEventCategory.crime,
        'Memory.Guild.Expelled': MemoryEventCategory.guild,
        'Memory.Sleep.Interrupted': MemoryEventCategory.rest,
        'Memory.Story.Choice': MemoryEventCategory.story,
        'Memory.Chapter.Started': MemoryEventCategory.story,
      };

      for (final MapEntry(key: tag, value: expected) in cases.entries) {
        final result = presenter.present(MemoryEvent(index: 0, tags: [tag]));
        expect(result.category, expected, reason: tag);
      }
    });

    test('uses localized quest names and localized event text', () {
      final result = _presenter(AppLocalizationsDe()).present(
        const MemoryEvent(
          index: 1,
          tags: ['Memory.Quest.Started'],
          optionalClass1:
              '/Script/Angelscript.QuestObjective_NewCamp_FINDMUTTON_TALK',
          optionalClass2: '/Script/Angelscript.Quest_NewCamp_FINDMUTTON',
        ),
      );

      expect(result.kind, MemoryEventKind.questStarted);
      expect(result.category, MemoryEventCategory.quest);
      expect(result.categoryLabel, 'Quest');
      expect(result.title, 'Quest gestartet: Ein Schafsgesuch');
      expect(result.subjectId, contains('Quest_NewCamp_FINDMUTTON'));
    });

    test(
      'resolves item classes through item and game localization catalogs',
      () {
        final result = _presenter(AppLocalizationsEn()).present(
          const MemoryEvent(
            index: 2,
            tags: ['Memory.Item.Obtained'],
            optionalClass1: '/Script/Angelscript.ItFo_Mutton_01',
            magnitude: 3,
          ),
        );

        expect(result.title, 'Item obtained: Raw mutton');
        expect(
          result.facts
              .singleWhere((fact) => fact.kind == MemoryEventFactKind.amount)
              .value,
          '3',
        );
      },
    );

    test('combines localized NPC and glossary segment labels', () {
      final result = _presenter(AppLocalizationsDe()).present(
        const MemoryEvent(
          index: 3,
          tags: ['Memory.Document.SegmentUnlocked'],
          optionalClass1: '/Script/Angelscript.Document_Diego',
          optionalClass2: '/Script/Angelscript.Glossary_Diego_Introduction',
        ),
      );

      expect(result.title, 'Eintrag entdeckt: Diego — Begegnung / Porträt');
      expect(result.category, MemoryEventCategory.document);
    });

    test(
      'turns actor ids, skill tags, guilds and crimes into useful subjects',
      () {
        final presenter = _presenter(AppLocalizationsEn());
        final kill = presenter.present(
          const MemoryEvent(
            index: 4,
            tags: ['Memory.Character.Defeated.Kill'],
            affected: 'OC_STT_Diego-01234567-89ab-cdef-0123-456789abcdef',
            magnitude: 0,
          ),
        );
        final skill = presenter.present(
          const MemoryEvent(
            index: 5,
            tags: ['Memory.Learned.Skill', 'Skill.Hunting.Claw'],
          ),
        );
        final guild = presenter.present(
          const MemoryEvent(
            index: 6,
            tags: ['Memory.Guild.Joined', 'Guild.OldCamp'],
          ),
        );
        final crime = presenter.present(
          const MemoryEvent(index: 7, tags: ['Memory.Crime.Lockpick']),
        );

        expect(kill.title, 'Character killed: Diego');
        expect(
          kill.facts.where((fact) => fact.kind == MemoryEventFactKind.amount),
          isEmpty,
        );
        expect(skill.title, 'Skill state recorded: Extract Claw');
        expect(guild.title, 'Guild joined: Old Camp');
        expect(crime.title, 'Crime recorded: Lockpicking');
      },
    );

    test('treats learned-skill memories as snapshots, not invented deltas', () {
      final result = _presenter(AppLocalizationsEn()).present(
        const MemoryEvent(
          index: 8,
          tags: [
            'Memory.Learned.Skill',
            'Skill.Lockpicking.Untrained',
            'Skill.Pickpocketing.Untrained',
            'Skill.Orcish.Untrained',
            'Skill.Melee.OneHanded.Trained',
            'Skill.Mage.Circle.Amateur',
          ],
        ),
      );

      expect(result.kind, MemoryEventKind.skillStateRecorded);
      expect(result.title, 'Skill state recorded: One Handed, Magic Circle');
      expect(result.title, isNot(contains('Lockpicking')));
      expect(result.title, isNot(contains('Orcish')));
    });

    test('prefers localized species over spawned waypoint ids', () {
      final result = _presenter(AppLocalizationsEn()).present(
        const MemoryEvent(
          index: 9,
          tags: ['Memory.Character.Defeated.Kill', 'Species.Creature.Meatbug'],
          instigator: 'Hero',
          affected: 'Meatbug-WP_EZ_GATE_MEATBUG_SPAWN_01-1',
        ),
      );

      expect(result.title, 'Character killed: Meatbug');
      expect(result.title, isNot(contains('01 1')));
      expect(
        result.facts
            .singleWhere((fact) => fact.kind == MemoryEventFactKind.affected)
            .value,
        'Meatbug',
      );
    });

    test('uses zero-based chapter magnitude as a one-based chapter label', () {
      final first = _presenter(AppLocalizationsEn()).present(
        const MemoryEvent(
          index: 10,
          tags: ['Memory.Chapter.Completed'],
          magnitude: 0,
          optionalClass1: '/Script/Angelscript.StoryG1R',
        ),
      );
      final second = _presenter(AppLocalizationsEn()).present(
        const MemoryEvent(
          index: 11,
          tags: ['Memory.Chapter.Completed'],
          magnitude: 1,
          optionalClass1: '/Script/Angelscript.StoryG1R',
        ),
      );

      expect(first.title, 'Chapter completed: Chapter 1');
      expect(second.title, 'Chapter completed: Chapter 2');
      final chapterFact = first.facts.singleWhere(
        (fact) => fact.kind == MemoryEventFactKind.chapter,
      );
      expect(chapterFact.value, 'Chapter 1');
      expect(chapterFact.technicalValue, '0.0');
    });

    test('normalizes recipe and glossary classes to game localization ids', () {
      final presenter = _presenter(AppLocalizationsEn());
      final recipe = presenter.present(
        const MemoryEvent(
          index: 12,
          tags: ['Memory.Learned.Recipe'],
          optionalClass1: '/Script/Angelscript.ReFo_Meatbugragout',
        ),
      );
      final document = presenter.present(
        const MemoryEvent(
          index: 13,
          tags: ['Memory.Document.Read'],
          optionalClass1: '/Script/Angelscript.Document_Glossary_Meatbug',
        ),
      );

      expect(recipe.title, 'Recipe learned: Meatbug ragout');
      expect(document.title, 'Document read: Meatbug');
    });

    test('resolves clean segment labels and localized segment prose', () {
      final result = _presenter(AppLocalizationsEn()).present(
        const MemoryEvent(
          index: 14,
          tags: ['Memory.Document.SegmentUnlocked'],
          optionalClass1: '/Script/Angelscript.Document_Glossary_Meatbug',
          optionalClass2:
              '/Script/Angelscript.DocumentSegment_Glossary_Meatbug_Entry2',
        ),
      );

      expect(result.title, 'Entry discovered: Meatbug — Entry 2');
      expect(
        result.facts
            .singleWhere((fact) => fact.kind == MemoryEventFactKind.segmentText)
            .value,
        'A surprisingly nutritious little creature.',
      );
    });

    test(
      'formats useful facts and does not surface invalid or zero values',
      () {
        final event = MemoryEvent.fromJson({
          'index': 8,
          'tags': ['Memory.Sleep'],
          'timeSeconds': 90061.9,
          'durationSeconds': 65,
          'magnitude': 0,
          'instigator': 'Hero',
          'affected': 'None',
        });
        final result = _presenter(AppLocalizationsEn()).present(event);

        expect(
          result.facts
              .singleWhere((fact) => fact.kind == MemoryEventFactKind.time)
              .value,
          'Day 1, 01:01:01',
        );
        expect(
          result.facts
              .singleWhere((fact) => fact.kind == MemoryEventFactKind.duration)
              .value,
          '00:01:05',
        );
        expect(
          result.facts
              .singleWhere(
                (fact) => fact.kind == MemoryEventFactKind.instigator,
              )
              .value,
          'Hero',
        );
        expect(
          result.facts.where((fact) => fact.kind == MemoryEventFactKind.amount),
          isEmpty,
        );
        expect(result.facts.map((fact) => fact.value), isNot(contains('None')));
      },
    );

    test('unknown future events remain readable and preserve raw tags', () {
      final result = _presenter(
        AppLocalizationsEn(),
      ).present(const MemoryEvent(index: 9, tags: ['Memory.Future.Danced']));

      expect(result.kind, MemoryEventKind.other);
      expect(result.category, MemoryEventCategory.other);
      expect(result.title, 'Future Danced');
      expect(result.tags, ['Memory.Future.Danced']);
    });

    test('uses StoryEvent payload names before generic actor fallbacks', () {
      final event = MemoryEvent.fromJson({
        'index': 12,
        'tags': ['Memory.StoryEvent'],
        'instigator': 'Hero',
        'payload': {
          'type': '/Script/G1R.StoryEventPayload',
          'fieldCount': 1,
          'fields': [
            {
              'name': 'EventName',
              'type': 'NameProperty',
              'value': 'MudVoiceline1',
            },
          ],
        },
      });

      expect(
        _presenter(AppLocalizationsEn()).present(event).title,
        'Story event: Mud Voiceline1',
      );
    });

    test('known actions and categories are localized in every app locale', () {
      for (final locale in AppLocalizations.supportedLocales) {
        final l10n = lookupAppLocalizations(locale);
        final result = _presenter(l10n).present(
          const MemoryEvent(
            index: 10,
            tags: ['Memory.Quest.Started'],
            optionalClass1: 'Quest_NewCamp_FINDMUTTON',
          ),
        );

        expect(result.title.trim(), isNotEmpty, reason: locale.toLanguageTag());
        expect(
          result.title,
          isNot(contains('Memory.Quest.Started')),
          reason: locale.toLanguageTag(),
        );
        expect(
          result.categoryLabel.trim(),
          isNotEmpty,
          reason: locale.toLanguageTag(),
        );
      }
    });
  });
}

MemoryEventPresenter _presenter(AppLocalizations l10n) => MemoryEventPresenter(
  l10n: l10n,
  locCatalog: const {
    'quest-newcamp_findmutton-name': {
      'english_newer': 'A Sheepish Request',
      'german_new': 'Ein Schafsgesuch',
    },
    'itfo_mutton_01': {
      'english_newer': 'Raw mutton',
      'german_new': 'Rohes Hammelfleisch',
    },
    'itfo_meatbugragout': {
      'english_newer': 'Meatbug ragout',
      'german_new': 'Fleischwanzenragout',
    },
    'meatbug': {'english_newer': 'Meatbug', 'german_new': 'Fleischwanze'},
    'text_meatbug_entry_2': {
      'english_newer': 'A surprisingly nutritious little creature.',
      'german_new': 'Ein überraschend nahrhaftes kleines Tier.',
    },
    'oc_stt_diego': {'english_newer': 'Diego', 'german_new': 'Diego'},
  },
  itemCatalog: const ItemCatalog([
    ItemCatalogEntry(
      id: 'ItFo_Mutton_01',
      path: '/Script/Angelscript.ItFo_Mutton_01',
      category: 'food',
      icon: 'ItFo_Mutton_01',
    ),
  ]),
  npcGlossaryCatalog: const [
    NpcGlossaryCatalogEntry(
      id: 'OC_STT_Diego',
      uniqueName: 'OC_STT_Diego',
      documentClass: 'Document_Diego',
      camp: NpcGlossaryCamp.oldCamp,
      segments: [
        NpcGlossaryCatalogSegment(
          id: 'Introduction',
          segmentClass: 'Glossary_Diego_Introduction',
          label: 'Introduction',
        ),
      ],
    ),
  ],
  segmentTextCatalog: const {
    '/script/angelscript.documentsegment_glossary_meatbug_entry2': [
      'text_meatbug_entry_2',
    ],
  },
);
