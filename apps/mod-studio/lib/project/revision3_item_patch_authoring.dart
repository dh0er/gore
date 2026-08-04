import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';

typedef Revision3ItemPatchContentLoader =
    Future<Revision3ContentIndex> Function();
typedef Revision3ItemPatchNativeCatalogLoader =
    Future<AuthoringRevision3ItemCatalogReadResult> Function();
typedef Revision3ItemPatchTechnicalPublisher =
    Future<Revision3ItemPatchPublication> Function(
      Revision3ItemPatchTechnicalPlan plan,
    );

final class Revision3ItemPatchRequiresReopenException implements Exception {
  const Revision3ItemPatchRequiresReopenException();
}

final class Revision3ItemPatchStaleCheckpointException implements Exception {
  const Revision3ItemPatchStaleCheckpointException();
}

final class Revision3ItemPatchNoChangesException implements Exception {
  const Revision3ItemPatchNoChangesException();
}

final class Revision3ItemPatchUnsupportedSchemaException implements Exception {
  const Revision3ItemPatchUnsupportedSchemaException();
}

/// One field in the exact native schema plus its optional project override.
final class Revision3ItemPatchFieldChoice {
  const Revision3ItemPatchFieldChoice._({
    required this.name,
    required this.scalarType,
    required this.numericDomain,
    required this.minimumValue,
    required this.maximumValue,
    required this.defaultValue,
    required this.currentValue,
  });

  final String name;
  final AuthoringRevision3ItemScalarType scalarType;
  final AuthoringRevision3ItemNumericDomain? numericDomain;
  final AuthoringRevision3ItemScalarValue? minimumValue;
  final AuthoringRevision3ItemScalarValue? maximumValue;
  final AuthoringRevision3ItemScalarValue? defaultValue;
  final Revision3ContentItemScalarValue? currentValue;

  bool accepts(AuthoringRevision3ItemScalarValue value) =>
      switch ((numericDomain, value.type)) {
        (
          AuthoringRevision3ItemNumericDomain.signedInteger32,
          AuthoringRevision3ItemScalarType.integer,
        ) =>
          value.integerValue! >= minimumValue!.integerValue! &&
              value.integerValue! <= maximumValue!.integerValue!,
        (
          AuthoringRevision3ItemNumericDomain.finiteFloat32,
          AuthoringRevision3ItemScalarType.float_,
        ) =>
          value.floatValue! >= minimumValue!.floatValue! &&
              value.floatValue! <= maximumValue!.floatValue!,
        (null, AuthoringRevision3ItemScalarType.boolean) =>
          scalarType == AuthoringRevision3ItemScalarType.boolean,
        _ => false,
      };
}

/// Author-facing item choice. Managed entity identity and provenance remain
/// hidden orchestration state; the vanilla class is a useful game identifier.
final class Revision3ItemPatchChoice {
  Revision3ItemPatchChoice._({
    required this.stableKey,
    required this.displayName,
    required this.vanillaClass,
    required this.category,
    required List<Revision3ItemPatchFieldChoice> fields,
    required Map<String, AuthoringRevision3ItemScalarValue> currentOverrides,
    required this.hasPatch,
    required this.canEdit,
    required this._projectScopeIdentity,
    required this._projectId,
    required this._projectRevision,
    required this._expectedHead,
    required this._catalogRead,
    required this._catalogEntry,
    required this._entityId,
    required this._entityRevision,
    required this._fingerprint,
  }) : fields = List<Revision3ItemPatchFieldChoice>.unmodifiable(fields),
       currentOverrides =
           Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(
             currentOverrides,
           );

  final String stableKey;
  final String displayName;
  final String vanillaClass;
  final AuthoringRevision3ItemCatalogCategory category;
  final List<Revision3ItemPatchFieldChoice> fields;
  final Map<String, AuthoringRevision3ItemScalarValue> currentOverrides;
  final bool hasPatch;
  final bool canEdit;

  final String _projectScopeIdentity;
  final String _projectId;
  final int _projectRevision;
  final AuthoringWorkingHead _expectedHead;
  final AuthoringRevision3ItemCatalogReadResult _catalogRead;
  final AuthoringRevision3ItemCatalogEntry _catalogEntry;
  final String? _entityId;
  final int? _entityRevision;
  final String _fingerprint;

  Revision3ItemPatchFieldChoice? field(String name) {
    for (final field in fields) {
      if (field.name == name) return field;
    }
    return null;
  }

  bool matches(String query) {
    final folded = query.trim().toLowerCase();
    if (folded.isEmpty) return true;
    return <String>[
      displayName,
      vanillaClass,
      category.wireName,
      ...fields.map((field) => field.name),
    ].any((value) => value.toLowerCase().contains(folded));
  }
}

final class Revision3ItemPatchCatalog {
  Revision3ItemPatchCatalog._({
    required this.projectId,
    required this.projectRevision,
    required this.head,
    required this.catalogSeal,
    required List<Revision3ItemPatchChoice> choices,
  }) : choices = List<Revision3ItemPatchChoice>.unmodifiable(choices);

  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead head;
  final AuthoringDraftContentSeal catalogSeal;
  final List<Revision3ItemPatchChoice> choices;

  Revision3ItemPatchChoice? choiceByStableKey(String stableKey) {
    for (final choice in choices) {
      if (choice.stableKey == stableKey) return choice;
    }
    return null;
  }
}

/// Hidden exact-current plan. The managed session re-reads the native catalog
/// inside its serialized publication lane before constructing the native
/// prepare request.
final class Revision3ItemPatchTechnicalPlan {
  Revision3ItemPatchTechnicalPlan._({
    required this.expectedProjectId,
    required this.expectedProjectRevision,
    required this.expectedHead,
    required this.action,
    required this.entityId,
    required this.expectedEntityRevision,
    required this.displayName,
    required this.vanillaClass,
    required this.expectedCatalogSeal,
    required this.expectedCatalogLayer,
    required this.expectedSourceSeal,
    required Map<String, AuthoringRevision3ItemScalarValue> fields,
  }) : fields = Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(
         fields,
       );

  final String expectedProjectId;
  final int expectedProjectRevision;
  final AuthoringWorkingHead expectedHead;
  final AuthoringRevision3ItemPatchAction action;
  final String entityId;
  final int? expectedEntityRevision;
  final String? displayName;
  final String vanillaClass;
  final AuthoringDraftContentSeal expectedCatalogSeal;
  final String expectedCatalogLayer;
  final AuthoringDraftContentSeal expectedSourceSeal;
  final Map<String, AuthoringRevision3ItemScalarValue> fields;
}

final class Revision3ItemPatchPublication {
  Revision3ItemPatchPublication({
    required this.projectId,
    required this.projectRevision,
    required this.entityId,
    required this.entityRevision,
    required this.change,
    required this.vanillaClass,
  }) {
    _itemAuthoringId(projectId, 'publication project ID');
    _itemAuthoringId(entityId, 'publication entity ID');
    _itemAuthoringIdentifier(vanillaClass, 'publication vanilla class');
    if (projectRevision < 1 ||
        (change == AuthoringRevision3ItemPatchChange.removed
            ? entityRevision != null
            : entityRevision == null || entityRevision! < 0)) {
      throw const FormatException(
        'ItemPatch publication has invalid revisions.',
      );
    }
  }

  final String projectId;
  final int projectRevision;
  final String entityId;
  final int? entityRevision;
  final AuthoringRevision3ItemPatchChange change;
  final String vanillaClass;
}

/// Safe authoring facade for discovering, editing, and reverting managed item
/// overrides without exposing build, deployment, game, or save authority.
final class Revision3ItemPatchAuthoringService {
  const Revision3ItemPatchAuthoringService({
    required this.projectScopeIdentity,
    required this.projectId,
    required this.projectRevision,
    required this.expectedHead,
    required this.loadContentIndex,
    required this.loadNativeCatalog,
    required this.publishTechnicalPlan,
  }) : assert(projectScopeIdentity != '');

  /// Stable managed-project scope, normally the canonical project-root path.
  ///
  /// Project ID, revision, and head can be byte-identical across copied roots;
  /// drafts and cached native catalogs must still never cross that boundary.
  final String projectScopeIdentity;
  final String projectId;
  final int projectRevision;
  final AuthoringWorkingHead expectedHead;
  final Revision3ItemPatchContentLoader loadContentIndex;
  final Revision3ItemPatchNativeCatalogLoader loadNativeCatalog;
  final Revision3ItemPatchTechnicalPublisher publishTechnicalPlan;

  Future<Revision3ItemPatchCatalog> loadCatalog() async {
    final content = await loadContentIndex();
    final native = await loadNativeCatalog();
    return _buildCatalog(content: content, native: native);
  }

  Future<Revision3ItemPatchPublication> save({
    required Revision3ItemPatchChoice choice,
    required Map<String, AuthoringRevision3ItemScalarValue> desiredOverrides,
  }) async {
    // Freeze the button-click intent before the first asynchronous checkpoint
    // refresh. The editor owns a mutable draft map; without this copy, typing
    // while the refresh is in flight could silently change the plan that is
    // eventually published.
    final submittedOverrides =
        Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(
          Map<String, AuthoringRevision3ItemScalarValue>.from(desiredOverrides),
        );
    final fresh = await loadCatalog();
    final current = fresh.choiceByStableKey(choice.stableKey);
    if (current == null || current._fingerprint != choice._fingerprint) {
      throw const Revision3ItemPatchStaleCheckpointException();
    }
    final desired = _itemAuthoringValidateDesired(current, submittedOverrides);
    if (!current.canEdit && desired.isNotEmpty) {
      throw const Revision3ItemPatchUnsupportedSchemaException();
    }
    if (current.canEdit &&
        _itemAuthoringSameValues(desired, current.currentOverrides)) {
      throw const Revision3ItemPatchNoChangesException();
    }
    final plan = _plan(current, desired);
    final publication = await publishTechnicalPlan(plan);
    final expectedChange = switch ((current.hasPatch, desired.isEmpty)) {
      (true, true) => AuthoringRevision3ItemPatchChange.removed,
      (true, false) => AuthoringRevision3ItemPatchChange.updated,
      (false, false) => AuthoringRevision3ItemPatchChange.created,
      (false, true) => throw const Revision3ItemPatchNoChangesException(),
    };
    final expectedEntityRevision = switch (expectedChange) {
      AuthoringRevision3ItemPatchChange.created => 0,
      AuthoringRevision3ItemPatchChange.updated => current._entityRevision! + 1,
      AuthoringRevision3ItemPatchChange.removed => null,
    };
    if (publication.projectId != projectId ||
        publication.projectRevision != projectRevision + 1 ||
        publication.entityId != plan.entityId ||
        publication.entityRevision != expectedEntityRevision ||
        publication.change != expectedChange ||
        publication.vanillaClass != current.vanillaClass) {
      throw const Revision3ItemPatchStaleCheckpointException();
    }
    return publication;
  }

  Revision3ItemPatchCatalog _buildCatalog({
    required Revision3ContentIndex content,
    required AuthoringRevision3ItemCatalogReadResult native,
  }) {
    if (content.projectId != projectId ||
        content.projectRevision != projectRevision ||
        native.projectId != projectId ||
        native.projectRevision != projectRevision ||
        native.head.canonicalJson != expectedHead.canonicalJson ||
        native.catalog.targetExecutable.sha256 !=
            content.targetExecutableSha256 ||
        native.catalog.targetExecutable.byteLength !=
            content.targetExecutableByteLength) {
      throw const Revision3ItemPatchStaleCheckpointException();
    }
    final patches = <String, Revision3ContentEntity>{};
    for (final entity in content.entities) {
      if (entity.kind != Revision3ContentEntityKind.itemPatch) continue;
      final facts = entity.summary.itemPatch;
      if (facts == null || patches.containsKey(facts.vanillaClass)) {
        throw const Revision3ItemPatchUnsupportedSchemaException();
      }
      patches[facts.vanillaClass] = entity;
    }
    final choices = <Revision3ItemPatchChoice>[];
    for (final entry in native.catalog.entries) {
      final patch = patches.remove(entry.vanillaClass);
      choices.add(
        _itemAuthoringChoice(
          projectScopeIdentity: projectScopeIdentity,
          projectId: projectId,
          projectRevision: projectRevision,
          expectedHead: expectedHead,
          native: native,
          entry: entry,
          patch: patch,
        ),
      );
    }
    if (patches.isNotEmpty) {
      throw const Revision3ItemPatchUnsupportedSchemaException();
    }
    choices.sort((left, right) {
      final byName = left.displayName.toLowerCase().compareTo(
        right.displayName.toLowerCase(),
      );
      return byName != 0
          ? byName
          : left.vanillaClass.compareTo(right.vanillaClass);
    });
    return Revision3ItemPatchCatalog._(
      projectId: projectId,
      projectRevision: projectRevision,
      head: expectedHead,
      catalogSeal: native.catalog.catalogSeal,
      choices: choices,
    );
  }

  Revision3ItemPatchTechnicalPlan _plan(
    Revision3ItemPatchChoice choice,
    Map<String, AuthoringRevision3ItemScalarValue> desired,
  ) {
    if (choice._projectScopeIdentity != projectScopeIdentity ||
        choice._projectId != projectId ||
        choice._projectRevision != projectRevision ||
        choice._expectedHead.canonicalJson != expectedHead.canonicalJson ||
        choice._catalogRead.head.canonicalJson != expectedHead.canonicalJson) {
      throw const Revision3ItemPatchStaleCheckpointException();
    }
    if (desired.isEmpty) {
      if (!choice.hasPatch ||
          choice._entityId == null ||
          choice._entityRevision == null) {
        throw const Revision3ItemPatchNoChangesException();
      }
      return Revision3ItemPatchTechnicalPlan._(
        expectedProjectId: projectId,
        expectedProjectRevision: projectRevision,
        expectedHead: expectedHead,
        action: AuthoringRevision3ItemPatchAction.remove,
        entityId: choice._entityId,
        expectedEntityRevision: choice._entityRevision,
        displayName: null,
        vanillaClass: choice.vanillaClass,
        expectedCatalogSeal: choice._catalogRead.catalog.catalogSeal,
        expectedCatalogLayer: choice._catalogRead.catalog.catalogLayer,
        expectedSourceSeal: choice._catalogEntry.sourceSeal,
        fields: const <String, AuthoringRevision3ItemScalarValue>{},
      );
    }
    if (!choice.canEdit) {
      throw const Revision3ItemPatchUnsupportedSchemaException();
    }
    return Revision3ItemPatchTechnicalPlan._(
      expectedProjectId: projectId,
      expectedProjectRevision: projectRevision,
      expectedHead: expectedHead,
      action: AuthoringRevision3ItemPatchAction.upsert,
      entityId:
          choice._entityId ??
          _itemAuthoringDeterministicEntityId(projectId, choice.vanillaClass),
      expectedEntityRevision: choice._entityRevision,
      displayName: choice.displayName,
      vanillaClass: choice.vanillaClass,
      expectedCatalogSeal: choice._catalogRead.catalog.catalogSeal,
      expectedCatalogLayer: choice._catalogRead.catalog.catalogLayer,
      expectedSourceSeal: choice._catalogEntry.sourceSeal,
      fields: desired,
    );
  }
}

Revision3ItemPatchChoice _itemAuthoringChoice({
  required String projectScopeIdentity,
  required String projectId,
  required int projectRevision,
  required AuthoringWorkingHead expectedHead,
  required AuthoringRevision3ItemCatalogReadResult native,
  required AuthoringRevision3ItemCatalogEntry entry,
  required Revision3ContentEntity? patch,
}) {
  final patchFacts = patch?.summary.itemPatch;
  final current = <String, AuthoringRevision3ItemScalarValue>{};
  final fieldChoices = <Revision3ItemPatchFieldChoice>[];
  final nativeFields = entry.fieldsByName;
  if (patchFacts != null) {
    for (final override in patchFacts.fields.entries) {
      final schema = nativeFields[override.key];
      final converted = _itemAuthoringValue(override.value);
      if (schema == null || converted == null || !schema.accepts(converted)) {
        throw const Revision3ItemPatchUnsupportedSchemaException();
      }
      current[override.key] = converted;
    }
  }
  for (final schema in entry.fields) {
    fieldChoices.add(
      Revision3ItemPatchFieldChoice._(
        name: schema.name,
        scalarType: schema.scalarType,
        numericDomain: schema.numericDomain,
        minimumValue: schema.minimumValue,
        maximumValue: schema.maximumValue,
        defaultValue: schema.defaultValue,
        currentValue: patchFacts?.fields[schema.name],
      ),
    );
  }
  final origin = patch?.origin;
  final source = origin?.sourceSeal;
  final storedSourceSeal = source == null
      ? null
      : AuthoringDraftContentSeal.fromJson(<String, Object?>{
          'byte_len': source.byteLength,
          'sha256': source.sha256,
        });
  if (patch != null &&
      (patchFacts == null ||
          origin?.catalogLayer == null ||
          storedSourceSeal == null)) {
    throw const Revision3ItemPatchUnsupportedSchemaException();
  }
  if (patch != null &&
      (origin?.catalogLayer != native.catalog.catalogLayer ||
          storedSourceSeal == null ||
          storedSourceSeal.byteLength != entry.sourceSeal.byteLength ||
          storedSourceSeal.sha256 != entry.sourceSeal.sha256)) {
    throw const Revision3ItemPatchUnsupportedSchemaException();
  }
  final fingerprint = crypto.sha256
      .convert(
        utf8.encode(
          jsonEncode(<Object?>[
            projectScopeIdentity,
            projectId,
            projectRevision,
            expectedHead.canonicalJson,
            native.catalog.catalogSeal.sha256,
            entry.vanillaClass,
            entry.sourceSeal.sha256,
            patch?.id,
            patch?.revision,
            origin?.catalogLayer,
            source?.sha256,
            for (final value in current.entries)
              <Object?>[
                value.key,
                value.value.type.wireName,
                value.value.value,
              ],
          ]),
        ),
      )
      .toString();
  return Revision3ItemPatchChoice._(
    stableKey: crypto.sha256
        .convert(
          utf8.encode(
            'gore-studio.revision3-item-choice\u0000$projectScopeIdentity\u0000$projectId\u0000${entry.vanillaClass}',
          ),
        )
        .toString(),
    displayName:
        patch?.displayName ?? revision3ItemFriendlyName(entry.vanillaClass),
    vanillaClass: entry.vanillaClass,
    category: entry.category,
    fields: fieldChoices,
    currentOverrides: current,
    hasPatch: patch != null,
    canEdit: true,
    projectScopeIdentity: projectScopeIdentity,
    projectId: projectId,
    projectRevision: projectRevision,
    expectedHead: expectedHead,
    catalogRead: native,
    catalogEntry: entry,
    entityId: patch?.id,
    entityRevision: patch?.revision,
    fingerprint: fingerprint,
  );
}

AuthoringRevision3ItemScalarValue? _itemAuthoringValue(
  Revision3ContentItemScalarValue value,
) => switch (value.type) {
  Revision3ContentItemScalarType.integer =>
    AuthoringRevision3ItemScalarValue.integer(value.integerValue!),
  Revision3ContentItemScalarType.float_ =>
    AuthoringRevision3ItemScalarValue.float(value.floatValue!),
  Revision3ContentItemScalarType.boolean =>
    AuthoringRevision3ItemScalarValue.boolean(value.booleanValue!),
  Revision3ContentItemScalarType.string ||
  Revision3ContentItemScalarType.enum_ => null,
};

Map<String, AuthoringRevision3ItemScalarValue> _itemAuthoringValidateDesired(
  Revision3ItemPatchChoice choice,
  Map<String, AuthoringRevision3ItemScalarValue> input,
) {
  if (input.length > 256) {
    throw const FormatException('ItemPatch contains too many fields.');
  }
  final names = input.keys.toList(growable: false)..sort();
  final result = <String, AuthoringRevision3ItemScalarValue>{};
  for (final name in names) {
    final field = choice.field(name);
    final value = input[name]!;
    if (field == null || !field.accepts(value)) {
      throw FormatException(
        'ItemPatch field $name is outside its native numeric domain.',
      );
    }
    result[name] = value;
  }
  return Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(result);
}

bool _itemAuthoringSameValues(
  Map<String, AuthoringRevision3ItemScalarValue> left,
  Map<String, AuthoringRevision3ItemScalarValue> right,
) {
  if (left.length != right.length) return false;
  for (final entry in left.entries) {
    final other = right[entry.key];
    if (other == null ||
        other.type != entry.value.type ||
        other.value != entry.value.value) {
      return false;
    }
  }
  return true;
}

String _itemAuthoringDeterministicEntityId(
  String projectId,
  String vanillaClass,
) => crypto.sha256
    .convert(
      utf8.encode(
        'gore-authoring.revision3-item-patch.entity-id\u0000$projectId\u0000$vanillaClass',
      ),
    )
    .toString()
    .substring(0, 32);

final _itemAuthoringIdPattern = RegExp(r'^[0-9a-f]{32}$');
final _itemAuthoringIdentifierPattern = RegExp(r'^[A-Za-z_][A-Za-z0-9_]*$');

String _itemAuthoringId(String value, String context) {
  if (!_itemAuthoringIdPattern.hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw FormatException('$context is invalid.');
  }
  return value;
}

String _itemAuthoringIdentifier(String value, String context) {
  if (!_itemAuthoringIdentifierPattern.hasMatch(value) ||
      utf8.encode(value).length > 256) {
    throw FormatException('$context is invalid.');
  }
  return value;
}

String revision3ItemFriendlyName(String vanillaClass) {
  const prefixes = <String>[
    'ItMw_',
    'ItRw_',
    'ItAr_',
    'ItFo_',
    'ItMi_',
    'ItAt_',
    'ItWr_',
    'ItMs_',
    'ItKe_',
    'ItAm_',
  ];
  var name = vanillaClass;
  for (final prefix in prefixes) {
    if (name.startsWith(prefix)) {
      name = name.substring(prefix.length);
      break;
    }
  }
  final cleaned = name.replaceAll('_', ' ').trim();
  return cleaned.isEmpty ? vanillaClass : cleaned;
}
