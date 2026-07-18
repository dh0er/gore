import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;

String revision3QuestInputFingerprint(Map<String, Object?> input) {
  const domain = 'gore-authoring.revision3-quest.input-fingerprint\u0000';
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
  List<String> additionalObjectiveTitles = const <String>[],
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
  if (additionalObjectiveTitles.isNotEmpty) {
    final output = StringBuffer('''FText $textHelper(const FName Text)
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

''');
    final titles = <String>[objectiveTitle, ...additionalObjectiveTitles];
    for (var index = 0; index < titles.length; index++) {
      final ordinal = index + 1;
      final objectiveClass = ordinal == 1
          ? objective
          : 'UQuest_${technicalId}_OBJ_$ordinal';
      final getter = ordinal == 1
          ? objectiveGetter
          : '$objectiveGetter$ordinal';
      output.write('''class $objectiveClass : UG1RQuest
{
    default ParentQuestClass = $root::StaticClass();
    default QuestKind = EQuestKind::Subobjective;
    default NameText = $textHelper(n"${titles[index]}");
    default bExternalStartTrigger = true;
    default bExternalSuccessTrigger = true;
''');
      if (ordinal == titles.length) {
        output.write('    default bSucceedParent = true;\n');
      }
      output.write('''}

$objectiveClass $getter()
{
    UQuestSubsystem Subsystem = UQuestSubsystem::Get();
    if (Subsystem == nullptr)
        return nullptr;

    TSubclassOf<UQuest> QuestClass =
        TSubclassOf<UQuest>($objectiveClass::StaticClass());
    UQuest Quest = Subsystem.GetQuestByClass(QuestClass);
    if (Quest == nullptr)
        return nullptr;

    return Cast<$objectiveClass>(Quest);
}
''');
      if (ordinal != titles.length) output.writeln();
    }
    return output.toString();
  }
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
