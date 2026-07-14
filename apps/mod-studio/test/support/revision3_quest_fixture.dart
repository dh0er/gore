import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;

String revision3QuestInputFingerprint(Map<String, Object?> input) {
  const domain = 'gore-story-build.revision3-quest-v2.input-fingerprint\u0000';
  final inputBytes = utf8.encode(jsonEncode(input));
  final length = ByteData(8)..setUint64(0, inputBytes.length, Endian.big);
  return crypto.sha256.convert(<int>[
    ...utf8.encode(domain),
    ...length.buffer.asUint8List(),
    ...inputBytes,
  ]).toString();
}

String revision3QuestGeneratedSource({
  required String technicalId,
  required String textHelper,
  required String parentRuntimeClass,
  required String giverRuntimeUniqueName,
  required String title,
  required String description,
  required String objectiveTitle,
}) {
  final pascal = technicalId
      .split('_')
      .map(
        (segment) =>
            '${segment.substring(0, 1)}${segment.substring(1).toLowerCase()}',
      )
      .join();
  final root = 'UQuest_$technicalId';
  final objective = 'UQuest_${technicalId}_OBJ_DONE';
  final rootGetter = 'Get$pascal';
  final objectiveGetter = 'Get${pascal}Objective';
  return '''FText $textHelper(const FName Text)
{
    FString Value = Text.ToString();
    return FText::FromString(Value);
}

class $root : UG1RQuest
{
    default ParentQuestClass = $parentRuntimeClass::StaticClass();
    default QuestKind = EQuestKind::Side;
    default InvolvedCharacters.Add(n"Hero");
    default InvolvedCharacters.Add(n"$giverRuntimeUniqueName");
    default QuestGiverCharacterUniqueName = n"$giverRuntimeUniqueName";
    default NameText = $textHelper(n"$title");
    default DescriptionText = $textHelper(
        n"$description"
    );
    default bExternalStartTrigger = true;
}

$root $rootGetter()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr)
        return nullptr;

    TSubclassOf<UQuest> QuestClass =
        TSubclassOf<UQuest>($root::StaticClass());
    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);
    if (Quest == nullptr)
        return nullptr;

    return Cast<$root>(Quest);
}

class $objective : UG1RQuest
{
    default ParentQuestClass = $root::StaticClass();
    default QuestKind = EQuestKind::Subobjective;
    default NameText = $textHelper(n"$objectiveTitle");
    default bExternalStartTrigger = true;
    default bExternalSuccessTrigger = true;
    default bSucceedParent = true;
}

$objective $objectiveGetter()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr)
        return nullptr;

    TSubclassOf<UQuest> QuestClass =
        TSubclassOf<UQuest>($objective::StaticClass());
    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);
    if (Quest == nullptr)
        return nullptr;

    return Cast<$objective>(Quest);
}
''';
}
