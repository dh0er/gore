part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3ItemPatchRequestBytes = 128 * 1024;
const _maxAuthoringRevision3ItemCatalogJsonBytes = 32 * 1024 * 1024;
const _maxAuthoringRevision3ItemCatalogEntries = 4096;
const _maxAuthoringRevision3ItemPatchFields = 256;
const _maxAuthoringRevision3ItemClassBytes = 256;
const _maxAuthoringRevision3ItemFieldBytes = 128;
const _maxAuthoringRevision3ItemCatalogLayerBytes = 128;
const _maxAuthoringRevision3ItemRuntimePathBytes = 1024;
const _minAuthoringSignedJsonInteger = -0x7fffffffffffffff - 1;
const _itemPatchSignedInteger32Minimum = -0x80000000;
const _itemPatchSignedInteger32Maximum = 0x7fffffff;
const _itemPatchFiniteFloat32Maximum = 3.4028234663852886e38;

final _authoringRevision3ItemIdentifierPattern = RegExp(
  r'^[A-Za-z_][A-Za-z0-9_]*$',
);

enum AuthoringRevision3ItemScalarType {
  integer('integer'),
  float_('float'),
  boolean('boolean');

  const AuthoringRevision3ItemScalarType(this.wireName);

  final String wireName;

  static AuthoringRevision3ItemScalarType _parse(
    Object? value,
    String context,
  ) => values.firstWhere(
    (type) => type.wireName == value,
    orElse: () => throw FormatException('$context has an invalid scalar type'),
  );
}

enum AuthoringRevision3ItemNumericDomain {
  signedInteger32('signed_integer32'),
  finiteFloat32('finite_float32');

  const AuthoringRevision3ItemNumericDomain(this.wireName);

  final String wireName;

  static AuthoringRevision3ItemNumericDomain _parse(
    Object? value,
    String context,
  ) => values.firstWhere(
    (domain) => domain.wireName == value,
    orElse: () => throw FormatException('$context has an invalid domain'),
  );
}

/// Closed scalar set currently proven by the native embedded Item schema.
final class AuthoringRevision3ItemScalarValue {
  const AuthoringRevision3ItemScalarValue._({
    required this.type,
    this.integerValue,
    this.floatValue,
    this.booleanValue,
  });

  factory AuthoringRevision3ItemScalarValue.integer(int value) {
    _itemPatchSignedInteger(value, 'item integer');
    return AuthoringRevision3ItemScalarValue._(
      type: AuthoringRevision3ItemScalarType.integer,
      integerValue: value,
    );
  }

  factory AuthoringRevision3ItemScalarValue.float(double value) =>
      AuthoringRevision3ItemScalarValue._(
        type: AuthoringRevision3ItemScalarType.float_,
        floatValue: _itemPatchFiniteFloat(value, 'item float'),
      );

  factory AuthoringRevision3ItemScalarValue.boolean(bool value) =>
      AuthoringRevision3ItemScalarValue._(
        type: AuthoringRevision3ItemScalarType.boolean,
        booleanValue: value,
      );

  final AuthoringRevision3ItemScalarType type;
  final int? integerValue;
  final double? floatValue;
  final bool? booleanValue;

  Object get value => switch (type) {
    AuthoringRevision3ItemScalarType.integer => integerValue!,
    AuthoringRevision3ItemScalarType.float_ => floatValue!,
    AuthoringRevision3ItemScalarType.boolean => booleanValue!,
  };

  Map<String, Object?> _toJson() => <String, Object?>{
    'type': type.wireName,
    'data': value,
  };

  factory AuthoringRevision3ItemScalarValue._fromJson(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const {'type', 'data'}, context);
    final type = AuthoringRevision3ItemScalarType._parse(
      json['type'],
      '$context type',
    );
    return switch (type) {
      AuthoringRevision3ItemScalarType.integer =>
        AuthoringRevision3ItemScalarValue.integer(
          _itemPatchSignedInteger(json['data'], '$context data'),
        ),
      AuthoringRevision3ItemScalarType.float_ =>
        AuthoringRevision3ItemScalarValue.float(
          _itemPatchFiniteFloat(json['data'], '$context data'),
        ),
      AuthoringRevision3ItemScalarType.boolean =>
        AuthoringRevision3ItemScalarValue.boolean(
          _itemPatchBoolean(json['data'], '$context data'),
        ),
    };
  }
}

enum AuthoringRevision3ItemCatalogCategory {
  ammunition('ammunition'),
  amulet('amulet'),
  armor('armor'),
  food('food'),
  key('key'),
  meleeWeapon('melee_weapon'),
  misc('misc'),
  mission('mission'),
  rangedWeapon('ranged_weapon'),
  ring('ring'),
  rune('rune'),
  scroll('scroll'),
  special('special'),
  trophy('trophy'),
  writing('writing');

  const AuthoringRevision3ItemCatalogCategory(this.wireName);

  final String wireName;

  static AuthoringRevision3ItemCatalogCategory _parse(
    Object? value,
    String context,
  ) => values.firstWhere(
    (category) => category.wireName == value,
    orElse: () => throw FormatException('$context has an invalid category'),
  );
}

final class AuthoringRevision3ItemCatalogField {
  const AuthoringRevision3ItemCatalogField._({
    required this.name,
    required this.scalarType,
    required this.numericDomain,
    required this.minimumValue,
    required this.maximumValue,
    required this.defaultValue,
  });

  final String name;
  final AuthoringRevision3ItemScalarType scalarType;
  final AuthoringRevision3ItemNumericDomain? numericDomain;
  final AuthoringRevision3ItemScalarValue? minimumValue;
  final AuthoringRevision3ItemScalarValue? maximumValue;
  final AuthoringRevision3ItemScalarValue? defaultValue;

  bool accepts(AuthoringRevision3ItemScalarValue value) {
    if (value.type != scalarType) return false;
    return switch ((numericDomain, value.type)) {
      (
        AuthoringRevision3ItemNumericDomain.signedInteger32,
        AuthoringRevision3ItemScalarType.integer,
      ) =>
        value.integerValue! >= _itemPatchSignedInteger32Minimum &&
            value.integerValue! <= _itemPatchSignedInteger32Maximum,
      (
        AuthoringRevision3ItemNumericDomain.finiteFloat32,
        AuthoringRevision3ItemScalarType.float_,
      ) =>
        value.floatValue!.abs() <= _itemPatchFiniteFloat32Maximum,
      (null, AuthoringRevision3ItemScalarType.boolean) => true,
      _ => false,
    };
  }

  factory AuthoringRevision3ItemCatalogField._fromJson(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    final scalarType = AuthoringRevision3ItemScalarType._parse(
      json['scalar_type'],
      '$context scalar_type',
    );
    final numeric = scalarType != AuthoringRevision3ItemScalarType.boolean;
    final hasDefault = json.containsKey('default_value');
    _authoringExactFields(json, <String>{
      'name',
      'scalar_type',
      if (numeric) ...{'numeric_domain', 'minimum_value', 'maximum_value'},
      if (hasDefault) 'default_value',
    }, context);
    final name = _itemPatchIdentifier(
      json['name'],
      '$context name',
      maxBytes: _maxAuthoringRevision3ItemFieldBytes,
    );
    final numericDomain = numeric
        ? AuthoringRevision3ItemNumericDomain._parse(
            json['numeric_domain'],
            '$context numeric_domain',
          )
        : null;
    final minimumValue = numeric
        ? AuthoringRevision3ItemScalarValue._fromJson(
            json['minimum_value'],
            '$context minimum_value',
          )
        : null;
    final maximumValue = numeric
        ? AuthoringRevision3ItemScalarValue._fromJson(
            json['maximum_value'],
            '$context maximum_value',
          )
        : null;
    final defaultValue = hasDefault
        ? AuthoringRevision3ItemScalarValue._fromJson(
            json['default_value'],
            '$context default_value',
          )
        : null;
    final field = AuthoringRevision3ItemCatalogField._(
      name: name,
      scalarType: scalarType,
      numericDomain: numericDomain,
      minimumValue: minimumValue,
      maximumValue: maximumValue,
      defaultValue: defaultValue,
    );
    final exactDomain = switch (scalarType) {
      AuthoringRevision3ItemScalarType.integer =>
        numericDomain == AuthoringRevision3ItemNumericDomain.signedInteger32 &&
            minimumValue?.type == AuthoringRevision3ItemScalarType.integer &&
            minimumValue?.integerValue == _itemPatchSignedInteger32Minimum &&
            maximumValue?.type == AuthoringRevision3ItemScalarType.integer &&
            maximumValue?.integerValue == _itemPatchSignedInteger32Maximum,
      AuthoringRevision3ItemScalarType.float_ =>
        numericDomain == AuthoringRevision3ItemNumericDomain.finiteFloat32 &&
            minimumValue?.type == AuthoringRevision3ItemScalarType.float_ &&
            minimumValue?.floatValue == -_itemPatchFiniteFloat32Maximum &&
            maximumValue?.type == AuthoringRevision3ItemScalarType.float_ &&
            maximumValue?.floatValue == _itemPatchFiniteFloat32Maximum,
      AuthoringRevision3ItemScalarType.boolean =>
        numericDomain == null && minimumValue == null && maximumValue == null,
    };
    if (!exactDomain) {
      throw FormatException('$context has a false native numeric domain');
    }
    if (defaultValue != null && !field.accepts(defaultValue)) {
      throw FormatException('$context default has a false scalar type');
    }
    return field;
  }
}

final class AuthoringRevision3ItemCatalogEntry {
  AuthoringRevision3ItemCatalogEntry._({
    required this.category,
    required List<AuthoringRevision3ItemCatalogField> fields,
    required this.runtimePath,
    required this.sourceSeal,
    required this.vanillaClass,
  }) : fields = List<AuthoringRevision3ItemCatalogField>.unmodifiable(fields);

  final AuthoringRevision3ItemCatalogCategory category;
  final List<AuthoringRevision3ItemCatalogField> fields;
  final String runtimePath;
  final AuthoringDraftContentSeal sourceSeal;
  final String vanillaClass;

  late final Map<String, AuthoringRevision3ItemCatalogField> fieldsByName =
      Map<String, AuthoringRevision3ItemCatalogField>.unmodifiable(
        <String, AuthoringRevision3ItemCatalogField>{
          for (final field in fields) field.name: field,
        },
      );

  factory AuthoringRevision3ItemCatalogEntry._fromJson(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const {
      'category',
      'fields',
      'runtime_path',
      'source_seal',
      'vanilla_class',
    }, context);
    final vanillaClass = _itemPatchIdentifier(
      json['vanilla_class'],
      '$context vanilla_class',
      maxBytes: _maxAuthoringRevision3ItemClassBytes,
    );
    final runtimePath = _itemPatchText(
      json['runtime_path'],
      '$context runtime_path',
      maxBytes: _maxAuthoringRevision3ItemRuntimePathBytes,
    );
    if (runtimePath != '/Script/Angelscript.$vanillaClass') {
      throw FormatException('$context runtime path disagrees with its class');
    }
    final rawFields = json['fields'];
    if (rawFields is! List<Object?> ||
        rawFields.isEmpty ||
        rawFields.length > _maxAuthoringRevision3ItemPatchFields) {
      throw FormatException('$context fields are not a bounded nonempty list');
    }
    final fieldNames = <String>{};
    final fields = <AuthoringRevision3ItemCatalogField>[];
    for (var index = 0; index < rawFields.length; index++) {
      final field = AuthoringRevision3ItemCatalogField._fromJson(
        rawFields[index],
        '$context field $index',
      );
      if (!fieldNames.add(field.name)) {
        throw FormatException('$context has duplicate field ${field.name}');
      }
      fields.add(field);
    }
    return AuthoringRevision3ItemCatalogEntry._(
      category: AuthoringRevision3ItemCatalogCategory._parse(
        json['category'],
        '$context category',
      ),
      fields: fields,
      runtimePath: runtimePath,
      sourceSeal: _itemPatchSeal(json['source_seal'], '$context source_seal'),
      vanillaClass: vanillaClass,
    );
  }
}

final class AuthoringRevision3ItemCatalog {
  AuthoringRevision3ItemCatalog._({
    required this.catalogLayer,
    required this.catalogSeal,
    required List<AuthoringRevision3ItemCatalogEntry> entries,
    required this.targetExecutable,
    required this.canonicalJson,
  }) : entries = List<AuthoringRevision3ItemCatalogEntry>.unmodifiable(entries);

  final String catalogLayer;
  final AuthoringDraftContentSeal catalogSeal;
  final List<AuthoringRevision3ItemCatalogEntry> entries;
  final AuthoringDraftContentSeal targetExecutable;
  final String canonicalJson;

  late final Map<String, AuthoringRevision3ItemCatalogEntry> entriesByClass =
      Map<String, AuthoringRevision3ItemCatalogEntry>.unmodifiable(
        <String, AuthoringRevision3ItemCatalogEntry>{
          for (final entry in entries) entry.vanillaClass: entry,
        },
      );

  AuthoringRevision3ItemCatalogEntry? entry(String vanillaClass) =>
      entriesByClass[vanillaClass];

  factory AuthoringRevision3ItemCatalog._fromCanonicalJson(String source) {
    if (source.isEmpty ||
        utf8.encode(source).length >
            _maxAuthoringRevision3ItemCatalogJsonBytes) {
      throw const FormatException('item catalog JSON is not bounded');
    }
    final json = _authoringDecodeDuplicateSafeObject(
      source,
      'revision-3 item catalog',
    );
    final itemScalars = _itemPatchCatalogScalarEnvelopes(
      json,
    ).toList(growable: false);
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 item catalog',
      closedItemScalars: itemScalars,
    );
    if (_authoringEncodeCanonicalJsonWithItemFloats(json, itemScalars) !=
        source) {
      throw const FormatException('item catalog JSON is not canonical');
    }
    _authoringExactFields(json, const {
      'catalog_layer',
      'catalog_seal',
      'entries',
      'schema_revision',
      'target',
    }, 'revision-3 item catalog');
    if (json['schema_revision'] != 1) {
      throw const FormatException('item catalog schema is unsupported');
    }
    final catalogLayer = _itemPatchCatalogLayer(
      json['catalog_layer'],
      'item catalog layer',
    );
    final rawEntries = json['entries'];
    if (rawEntries is! List<Object?> ||
        rawEntries.isEmpty ||
        rawEntries.length > _maxAuthoringRevision3ItemCatalogEntries) {
      throw const FormatException('item catalog entries are not bounded');
    }
    final entries = <AuthoringRevision3ItemCatalogEntry>[];
    String? previousClass;
    for (var index = 0; index < rawEntries.length; index++) {
      final entry = AuthoringRevision3ItemCatalogEntry._fromJson(
        rawEntries[index],
        'item catalog entry $index',
      );
      if (previousClass != null &&
          previousClass.compareTo(entry.vanillaClass) >= 0) {
        throw const FormatException(
          'item catalog entries are not unique canonical order',
        );
      }
      previousClass = entry.vanillaClass;
      entries.add(entry);
    }
    final target = _authoringRequiredObject(
      json['target'],
      'item catalog target',
    );
    _authoringExactFields(target, const {'executable'}, 'item catalog target');
    return AuthoringRevision3ItemCatalog._(
      catalogLayer: catalogLayer,
      catalogSeal: _itemPatchSeal(json['catalog_seal'], 'item catalog seal'),
      entries: entries,
      targetExecutable: _itemPatchSeal(
        target['executable'],
        'item catalog target executable',
      ),
      canonicalJson: source,
    );
  }
}

enum AuthoringRevision3ItemCatalogAuthority {
  nativeEmbeddedSchemaExactCurrentProject,
}

enum AuthoringRevision3ItemCatalogBuildStatus { notEvaluated }

enum AuthoringRevision3ItemRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3ItemCatalogPublicationStatus { notApplicable }

final class AuthoringRevision3ItemCatalogReadResult {
  const AuthoringRevision3ItemCatalogReadResult._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.catalog,
    required this.catalogAuthority,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final AuthoringRevision3ItemCatalog catalog;
  final AuthoringRevision3ItemCatalogAuthority catalogAuthority;
  final AuthoringRevision3ItemCatalogBuildStatus buildStatus;
  final AuthoringRevision3ItemRuntimeStatus runtimeStatus;
  final AuthoringRevision3ItemCatalogPublicationStatus publicationStatus;

  factory AuthoringRevision3ItemCatalogReadResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
  }) {
    _authoringExactFields(json, const {
      'ok',
      'head_json',
      'project_id',
      'project_revision',
      'catalog_json',
      'catalog_seal',
      'catalog_authority',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 item catalog response');
    if (json['ok'] != true) {
      throw const FormatException('item catalog response is not successful');
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson != expectedHead.canonicalJson) {
      throw const FormatException('item catalog response changed its head');
    }
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'item catalog project ID',
    );
    if (projectId == '00000000000000000000000000000000') {
      throw const FormatException('item catalog project ID is zero');
    }
    final catalog = AuthoringRevision3ItemCatalog._fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'catalog_json',
        maxBytes: _maxAuthoringRevision3ItemCatalogJsonBytes,
      ),
    );
    final outerSeal = _itemPatchSeal(
      json['catalog_seal'],
      'item catalog response seal',
    );
    // These are native provenance seals, not hashes of `catalog_json` itself:
    // source seals cover each native entry's dedicated source document and the
    // catalog seal covers the native seal-source document (which excludes the
    // target/envelope). Keep the DTO closed and cross-check both copies here;
    // prepare independently recomputes and enforces the native seals.
    if (!_itemPatchSameSeal(outerSeal, catalog.catalogSeal)) {
      throw const FormatException(
        'item catalog response seal disagrees with its document',
      );
    }
    return AuthoringRevision3ItemCatalogReadResult._(
      head: head,
      projectId: projectId,
      projectRevision: _authoringRequiredInt(
        json,
        'project_revision',
        max: _maxAuthoringSignedJsonInteger,
      ),
      catalog: catalog,
      catalogAuthority: switch (json['catalog_authority']) {
        'native_embedded_schema_exact_current_project' =>
          AuthoringRevision3ItemCatalogAuthority
              .nativeEmbeddedSchemaExactCurrentProject,
        _ => throw const FormatException(
          'item catalog response grants unknown catalog authority',
        ),
      },
      buildStatus: switch (json['build_status']) {
        'not_evaluated' =>
          AuthoringRevision3ItemCatalogBuildStatus.notEvaluated,
        _ => throw const FormatException(
          'item catalog response grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3ItemRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'item catalog response grants runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_applicable' =>
          AuthoringRevision3ItemCatalogPublicationStatus.notApplicable,
        _ => throw const FormatException(
          'item catalog response grants publication authority',
        ),
      },
    );
  }
}

enum AuthoringRevision3ItemPatchAction { upsert, remove }

/// Exact canonical request for one native prepare-only ItemPatch transaction.
final class AuthoringRevision3ItemPatchRequestV1 {
  AuthoringRevision3ItemPatchRequestV1._({
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.action,
    required this.entityId,
    required this.expectedEntityRevision,
    required this.displayName,
    required this.catalogLayer,
    required this.vanillaClass,
    required this.sourceSeal,
    required this.fields,
    required this.expectedCatalogSeal,
  });

  factory AuthoringRevision3ItemPatchRequestV1.upsertForProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required AuthoringRevision3ItemCatalogReadResult catalogRead,
    required AuthoringRevision3ItemCatalogEntry catalogEntry,
    required String entityId,
    required int? expectedEntityRevision,
    required String displayName,
    required Map<String, AuthoringRevision3ItemScalarValue> fields,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    _itemPatchRequireCatalogBinding(
      current: current,
      expectedHead: expectedHead,
      catalogRead: catalogRead,
      catalogEntry: catalogEntry,
    );
    _itemPatchRequireCurrentProjectCatalog(
      current: current,
      catalog: catalogRead.catalog,
    );
    final normalizedFields = _itemPatchFieldsForCatalog(fields, catalogEntry);
    final request = AuthoringRevision3ItemPatchRequestV1._(
      expectedHead: expectedHead,
      expectedProjectId: current.projectId,
      expectedRevision: current.revision,
      expectedTargetCanonicalJson: jsonEncode(current.project['target']),
      action: AuthoringRevision3ItemPatchAction.upsert,
      entityId: _itemPatchEntityId(entityId, 'item-patch entity ID'),
      expectedEntityRevision: _itemPatchNullableIncrementableRevision(
        expectedEntityRevision,
        'item-patch expected entity revision',
      ),
      displayName: _itemPatchDisplayName(displayName),
      catalogLayer: catalogRead.catalog.catalogLayer,
      vanillaClass: catalogEntry.vanillaClass,
      sourceSeal: catalogEntry.sourceSeal,
      fields: normalizedFields,
      expectedCatalogSeal: catalogRead.catalog.catalogSeal,
    );
    request._requireExactProjectBinding(current);
    return request;
  }

  factory AuthoringRevision3ItemPatchRequestV1.removeForProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required AuthoringRevision3ItemCatalogReadResult catalogRead,
    required AuthoringRevision3ItemCatalogEntry currentCatalogEntry,
    required String entityId,
    required int expectedEntityRevision,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    _itemPatchRequireCatalogBinding(
      current: current,
      expectedHead: expectedHead,
      catalogRead: catalogRead,
      catalogEntry: currentCatalogEntry,
    );
    _itemPatchRequireCurrentProjectCatalog(
      current: current,
      catalog: catalogRead.catalog,
    );
    final request = AuthoringRevision3ItemPatchRequestV1._(
      expectedHead: expectedHead,
      expectedProjectId: current.projectId,
      expectedRevision: current.revision,
      expectedTargetCanonicalJson: jsonEncode(current.project['target']),
      action: AuthoringRevision3ItemPatchAction.remove,
      entityId: _itemPatchEntityId(entityId, 'item-patch entity ID'),
      expectedEntityRevision: _itemPatchCurrentEntityRevision(
        expectedEntityRevision,
        'item-patch expected entity revision',
      ),
      displayName: null,
      catalogLayer: catalogRead.catalog.catalogLayer,
      vanillaClass: currentCatalogEntry.vanillaClass,
      sourceSeal: currentCatalogEntry.sourceSeal,
      fields: const <String, AuthoringRevision3ItemScalarValue>{},
      expectedCatalogSeal: catalogRead.catalog.catalogSeal,
    );
    request._requireExactProjectBinding(current);
    return request;
  }

  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final AuthoringRevision3ItemPatchAction action;
  final String entityId;
  final int? expectedEntityRevision;
  final String? displayName;
  final String catalogLayer;
  final String vanillaClass;
  final AuthoringDraftContentSeal sourceSeal;
  final Map<String, AuthoringRevision3ItemScalarValue> fields;
  final AuthoringDraftContentSeal expectedCatalogSeal;

  late final String canonicalJson = _buildCanonicalJson();

  String _buildCanonicalJson() {
    final fieldJson = <String, Object?>{
      for (final entry in fields.entries) entry.key: entry.value._toJson(),
    };
    final mutation = switch (action) {
      AuthoringRevision3ItemPatchAction.upsert => <String, Object?>{
        'action': 'upsert',
        'entity_id': entityId,
        if (expectedEntityRevision != null)
          'expected_entity_revision': expectedEntityRevision,
        'display_name': displayName,
        'catalog_layer': catalogLayer,
        'vanilla_class': vanillaClass,
        'source_seal': _itemPatchSealJson(sourceSeal),
        'fields': fieldJson,
      },
      AuthoringRevision3ItemPatchAction.remove => <String, Object?>{
        'action': 'remove',
        'entity_id': entityId,
        'expected_entity_revision': expectedEntityRevision,
        'expected_catalog_layer': catalogLayer,
        'expected_vanilla_class': vanillaClass,
        'expected_source_seal': _itemPatchSealJson(sourceSeal),
      },
    };
    final document = <String, Object?>{
      'expected_head': jsonDecode(expectedHead.canonicalJson),
      'expected_project_id': expectedProjectId,
      'expected_revision': expectedRevision,
      'expected_target': jsonDecode(expectedTargetCanonicalJson),
      'mutation': mutation,
    };
    final value = _authoringEncodeCanonicalJsonWithItemFloats(
      document,
      fieldJson.values,
    );
    if (utf8.encode(value).length >
        _maxAuthoringRevision3ItemPatchRequestBytes) {
      throw const FormatException('item-patch request exceeds its byte budget');
    }
    return value;
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedRevision > _maxAuthoringStoryBaseRevision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'item-patch request does not bind the exact current project',
      );
    }
    final entities = _authoringRequiredObject(
      current.project['entities'],
      'item-patch current entities',
    );
    final rawEntity = entities[entityId];
    if (action == AuthoringRevision3ItemPatchAction.upsert &&
        expectedEntityRevision == null) {
      if (rawEntity != null ||
          _itemPatchFindTarget(entities, vanillaClass, except: entityId) !=
              null) {
        throw const FormatException(
          'item-patch create conflicts with the exact current project',
        );
      }
      return;
    }
    final facts = _itemPatchEntityFacts(
      rawEntity,
      expectedId: entityId,
      context: 'item-patch current entity',
    );
    if (facts.revision != expectedEntityRevision ||
        facts.vanillaClass != vanillaClass ||
        facts.generationCanonicalJson != expectedTargetCanonicalJson ||
        facts.catalogLayer != catalogLayer ||
        !_itemPatchSameSeal(facts.sourceSeal, sourceSeal) ||
        _itemPatchFindTarget(entities, vanillaClass, except: entityId) !=
            null) {
      throw const FormatException(
        'item-patch request disagrees with the exact current entity',
      );
    }
  }
}

enum AuthoringRevision3ItemPatchChange { created, updated, removed }

enum AuthoringRevision3ItemPatchBuildStatus { blocked }

enum AuthoringRevision3ItemPatchPublicationStatus { notSupported }

/// Fully reopened unpublished candidate returned by native ItemPatch prepare.
final class AuthoringRevision3ItemPatchPreparation {
  const AuthoringRevision3ItemPatchPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.entityId,
    required this.entityRevision,
    required this.change,
    required this.catalogLayer,
    required this.vanillaClass,
    required this.sourceSeal,
    required this.catalogSeal,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String entityId;
  final int? entityRevision;
  final AuthoringRevision3ItemPatchChange change;
  final String catalogLayer;
  final String vanillaClass;
  final AuthoringDraftContentSeal sourceSeal;
  final AuthoringDraftContentSeal catalogSeal;
  final AuthoringRevision3ItemPatchBuildStatus buildStatus;
  final AuthoringRevision3ItemRuntimeStatus runtimeStatus;
  final AuthoringRevision3ItemPatchPublicationStatus publicationStatus;

  factory AuthoringRevision3ItemPatchPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3ItemPatchRequestV1 request,
  }) {
    final basis = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(basis);
    _authoringExactFields(json, const {
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'project_id',
      'revision',
      'entity_id',
      'entity_revision',
      'change',
      'catalog_layer',
      'vanilla_class',
      'source_seal',
      'catalog_seal',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 item-patch preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'item-patch response is not an unpublished preparation',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson ||
        head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'item-patch response has an invalid head transition',
      );
    }
    final projectJson = _authoringRevision3ResponseString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'item-patch response project ID',
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringSignedJsonInteger,
    );
    final entityId = _itemPatchEntityId(
      _authoringRequiredString(json, 'entity_id', maxBytes: 32),
      'item-patch response entity ID',
    );
    final rawEntityRevision = json['entity_revision'];
    final entityRevision = rawEntityRevision == null
        ? null
        : _itemPatchCurrentEntityRevision(
            rawEntityRevision,
            'item-patch response entity revision',
          );
    final change = switch (json['change']) {
      'created' => AuthoringRevision3ItemPatchChange.created,
      'updated' => AuthoringRevision3ItemPatchChange.updated,
      'removed' => AuthoringRevision3ItemPatchChange.removed,
      _ => throw const FormatException(
        'item-patch response has an invalid change',
      ),
    };
    final expectedChange = switch ((
      request.action,
      request.expectedEntityRevision,
    )) {
      (AuthoringRevision3ItemPatchAction.remove, _) =>
        AuthoringRevision3ItemPatchChange.removed,
      (AuthoringRevision3ItemPatchAction.upsert, null) =>
        AuthoringRevision3ItemPatchChange.created,
      _ => AuthoringRevision3ItemPatchChange.updated,
    };
    final expectedEntityRevision = switch (expectedChange) {
      AuthoringRevision3ItemPatchChange.created => 0,
      AuthoringRevision3ItemPatchChange.updated =>
        request.expectedEntityRevision! + 1,
      AuthoringRevision3ItemPatchChange.removed => null,
    };
    final catalogLayer = _itemPatchCatalogLayer(
      json['catalog_layer'],
      'item-patch response catalog layer',
    );
    final vanillaClass = _itemPatchIdentifier(
      json['vanilla_class'],
      'item-patch response vanilla class',
      maxBytes: _maxAuthoringRevision3ItemClassBytes,
    );
    final sourceSeal = _itemPatchSeal(
      json['source_seal'],
      'item-patch response source seal',
    );
    final catalogSeal = _itemPatchSeal(
      json['catalog_seal'],
      'item-patch response catalog seal',
    );
    if (projectId != basis.projectId ||
        projectId != candidate.projectId ||
        revision != basis.revision + 1 ||
        revision != candidate.revision ||
        entityId != request.entityId ||
        change != expectedChange ||
        entityRevision != expectedEntityRevision ||
        catalogLayer != request.catalogLayer ||
        vanillaClass != request.vanillaClass ||
        !_itemPatchSameSeal(sourceSeal, request.sourceSeal) ||
        !_itemPatchSameSeal(catalogSeal, request.expectedCatalogSeal)) {
      throw const FormatException(
        'item-patch response disagrees with its exact request',
      );
    }
    _itemPatchRequireExactCandidate(
      basis.project,
      candidate.project,
      request: request,
      expectedEntityRevision: entityRevision,
    );
    return AuthoringRevision3ItemPatchPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      entityId: entityId,
      entityRevision: entityRevision,
      change: change,
      catalogLayer: catalogLayer,
      vanillaClass: vanillaClass,
      sourceSeal: sourceSeal,
      catalogSeal: catalogSeal,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3ItemPatchBuildStatus.blocked,
        _ => throw const FormatException(
          'item-patch response grants build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3ItemRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'item-patch response grants runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3ItemPatchPublicationStatus.notSupported,
        _ => throw const FormatException(
          'item-patch response grants publication authority',
        ),
      },
    );
  }
}

Iterable<Object?> _itemPatchCatalogScalarEnvelopes(
  Map<String, Object?> catalog,
) sync* {
  final entries = catalog['entries'];
  if (entries is! List) return;
  for (final rawEntry in entries) {
    if (rawEntry is! Map) continue;
    final fields = rawEntry['fields'];
    if (fields is! List) continue;
    for (final rawField in fields) {
      if (rawField is! Map) continue;
      for (final key in const <String>[
        'minimum_value',
        'maximum_value',
        'default_value',
      ]) {
        if (rawField.containsKey(key)) yield rawField[key];
      }
    }
  }
}

int _itemPatchSignedInteger(Object? value, String context) {
  if (value is! int ||
      value < _minAuthoringSignedJsonInteger ||
      value > _maxAuthoringSignedJsonInteger) {
    throw FormatException('$context is not a signed 64-bit integer');
  }
  return value;
}

double _itemPatchFiniteFloat(Object? value, String context) {
  if (value is! double || !value.isFinite) {
    throw FormatException('$context is not a finite float');
  }
  return value == 0.0 ? 0.0 : value;
}

bool _itemPatchBoolean(Object? value, String context) {
  if (value is! bool) throw FormatException('$context is not a boolean');
  return value;
}

bool _itemPatchContainsControl(String value) =>
    value.runes.any((rune) => rune <= 0x1f || (rune >= 0x7f && rune <= 0x9f));

String _itemPatchText(Object? value, String context, {required int maxBytes}) {
  if (value is! String ||
      value.isEmpty ||
      value.trim() != value ||
      utf8.encode(value).length > maxBytes ||
      _itemPatchContainsControl(value)) {
    throw FormatException('$context is not bounded canonical text');
  }
  return value;
}

String _itemPatchIdentifier(
  Object? value,
  String context, {
  required int maxBytes,
}) {
  final result = _itemPatchText(value, context, maxBytes: maxBytes);
  if (!_authoringRevision3ItemIdentifierPattern.hasMatch(result)) {
    throw FormatException('$context is not a canonical identifier');
  }
  return result;
}

String _itemPatchEntityId(Object? value, String context) {
  if (value is! String ||
      !_authoringEntityIdPattern.hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw FormatException('$context is not a nonzero entity ID');
  }
  return value;
}

String _itemPatchDisplayName(Object? value) => _itemPatchText(
  value,
  'item-patch display name',
  maxBytes: _maxAuthoringRevision3ItemClassBytes,
);

String _itemPatchCatalogLayer(Object? value, String context) => _itemPatchText(
  value,
  context,
  maxBytes: _maxAuthoringRevision3ItemCatalogLayerBytes,
);

int _itemPatchCurrentEntityRevision(Object? value, String context) {
  if (value is! int || value < 0 || value > _maxAuthoringSignedJsonInteger) {
    throw FormatException('$context is not a signed-safe entity revision');
  }
  return value;
}

int _itemPatchIncrementableEntityRevision(Object? value, String context) {
  if (value is! int || value < 0 || value > _maxAuthoringStoryBaseRevision) {
    throw FormatException('$context is not an incrementable revision');
  }
  return value;
}

int? _itemPatchNullableIncrementableRevision(Object? value, String context) =>
    value == null
    ? null
    : _itemPatchIncrementableEntityRevision(value, context);

AuthoringDraftContentSeal _itemPatchSeal(Object? value, String context) {
  try {
    return AuthoringDraftContentSeal.fromJson(
      _authoringRequiredObject(value, context),
    );
  } on FormatException catch (error) {
    throw FormatException('$context is invalid: ${error.message}');
  }
}

Map<String, Object?> _itemPatchSealJson(AuthoringDraftContentSeal seal) =>
    <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};

bool _itemPatchSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

void _itemPatchRequireCatalogBinding({
  required ({Map<String, Object?> project, String projectId, int revision})
  current,
  required AuthoringWorkingHead expectedHead,
  required AuthoringRevision3ItemCatalogReadResult catalogRead,
  required AuthoringRevision3ItemCatalogEntry catalogEntry,
}) {
  final target = _authoringRequiredObject(
    current.project['target'],
    'item-patch project target',
  );
  _authoringExactFields(target, const {'executable'}, 'item-patch target');
  final executable = _itemPatchSeal(
    target['executable'],
    'item-patch target executable',
  );
  if (current.revision > _maxAuthoringStoryBaseRevision ||
      catalogRead.head.canonicalJson != expectedHead.canonicalJson ||
      catalogRead.projectId != current.projectId ||
      catalogRead.projectRevision != current.revision ||
      !_itemPatchSameSeal(catalogRead.catalog.targetExecutable, executable) ||
      !identical(
        catalogRead.catalog.entry(catalogEntry.vanillaClass),
        catalogEntry,
      ) ||
      catalogRead.catalogAuthority !=
          AuthoringRevision3ItemCatalogAuthority
              .nativeEmbeddedSchemaExactCurrentProject ||
      catalogRead.buildStatus !=
          AuthoringRevision3ItemCatalogBuildStatus.notEvaluated ||
      catalogRead.runtimeStatus !=
          AuthoringRevision3ItemRuntimeStatus.runtimeUnqualified ||
      catalogRead.publicationStatus !=
          AuthoringRevision3ItemCatalogPublicationStatus.notApplicable) {
    throw const FormatException(
      'item-patch catalog does not bind the exact current project',
    );
  }
}

void _itemPatchRequireCurrentProjectCatalog({
  required ({Map<String, Object?> project, String projectId, int revision})
  current,
  required AuthoringRevision3ItemCatalog catalog,
}) {
  final entities = _authoringRequiredObject(
    current.project['entities'],
    'item-patch current entities',
  );
  final targetCanonicalJson = jsonEncode(current.project['target']);
  final targets = <String>{};
  for (final raw in entities.entries) {
    final entity = _authoringRequiredObject(
      raw.value,
      'item-patch current entity',
    );
    final payload = _authoringRequiredObject(
      entity['payload'],
      'item-patch current payload',
    );
    if (payload['kind'] != 'item_patch') continue;
    final facts = _itemPatchEntityFacts(
      raw.value,
      expectedId: raw.key,
      context: 'item-patch current entity',
    );
    final entry = catalog.entry(facts.vanillaClass);
    if (entry == null ||
        !targets.add(facts.vanillaClass) ||
        facts.generationCanonicalJson != targetCanonicalJson ||
        facts.catalogLayer != catalog.catalogLayer ||
        !_itemPatchSameSeal(facts.sourceSeal, entry.sourceSeal) ||
        facts.fields.length > entry.fields.length) {
      throw const FormatException(
        'current ItemPatch is stale or unsupported by the native catalog',
      );
    }
    for (final field in facts.fields.entries) {
      final schema = entry.fieldsByName[field.key];
      if (schema == null || !schema.accepts(field.value)) {
        throw const FormatException(
          'current ItemPatch is stale or unsupported by the native catalog',
        );
      }
    }
  }
}

Map<String, AuthoringRevision3ItemScalarValue> _itemPatchFieldsForCatalog(
  Map<String, AuthoringRevision3ItemScalarValue> input,
  AuthoringRevision3ItemCatalogEntry catalogEntry,
) {
  if (input.isEmpty || input.length > _maxAuthoringRevision3ItemPatchFields) {
    throw const FormatException(
      'item-patch fields are not a bounded nonempty map',
    );
  }
  final names = input.keys.toList(growable: false)..sort();
  final result = <String, AuthoringRevision3ItemScalarValue>{};
  for (final name in names) {
    _itemPatchIdentifier(
      name,
      'item-patch field name',
      maxBytes: _maxAuthoringRevision3ItemFieldBytes,
    );
    final schema = catalogEntry.fieldsByName[name];
    final value = input[name]!;
    if (schema == null || !schema.accepts(value)) {
      throw FormatException(
        'item-patch field $name is outside the native catalog numeric domain',
      );
    }
    result[name] = value;
  }
  return Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(result);
}

({
  int revision,
  String displayName,
  String generationCanonicalJson,
  String catalogLayer,
  String vanillaClass,
  AuthoringDraftContentSeal sourceSeal,
  Map<String, AuthoringRevision3ItemScalarValue> fields,
})
_itemPatchEntityFacts(
  Object? value, {
  required String expectedId,
  required String context,
}) {
  final entity = _authoringRequiredObject(value, context);
  _authoringExactFields(entity, const {
    'id',
    'display_name',
    'origin',
    'revision',
    'payload',
  }, context);
  if (_itemPatchEntityId(entity['id'], '$context ID') != expectedId) {
    throw FormatException('$context ID disagrees with its map key');
  }
  final origin = _authoringRequiredObject(entity['origin'], '$context origin');
  _authoringExactFields(origin, const {
    'type',
    'generation',
    'catalog_layer',
    'canonical_selector',
    'source_seal',
  }, '$context origin');
  if (origin['type'] != 'vanilla') {
    throw FormatException('$context is not a vanilla ItemPatch');
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    '$context payload',
  );
  _authoringExactFields(payload, const {'kind', 'data'}, '$context payload');
  if (payload['kind'] != 'item_patch') {
    throw FormatException('$context has the wrong payload kind');
  }
  final data = _authoringRequiredObject(
    payload['data'],
    '$context payload data',
  );
  _authoringExactFields(data, const {
    'vanilla_class',
    'fields',
  }, '$context payload data');
  final vanillaClass = _itemPatchIdentifier(
    data['vanilla_class'],
    '$context vanilla class',
    maxBytes: _maxAuthoringRevision3ItemClassBytes,
  );
  if (origin['canonical_selector'] != vanillaClass) {
    throw FormatException('$context origin selector is false');
  }
  final fields = _authoringRequiredObject(data['fields'], '$context fields');
  if (fields.isEmpty || fields.length > _maxAuthoringRevision3ItemPatchFields) {
    throw FormatException('$context fields are not bounded');
  }
  final parsedFields = <String, AuthoringRevision3ItemScalarValue>{};
  for (final field in fields.entries) {
    final name = _itemPatchIdentifier(
      field.key,
      '$context field name',
      maxBytes: _maxAuthoringRevision3ItemFieldBytes,
    );
    parsedFields[name] = AuthoringRevision3ItemScalarValue._fromJson(
      field.value,
      '$context field $name',
    );
  }
  return (
    revision: _itemPatchCurrentEntityRevision(
      entity['revision'],
      '$context revision',
    ),
    displayName: _itemPatchDisplayName(entity['display_name']),
    generationCanonicalJson: jsonEncode(origin['generation']),
    catalogLayer: _itemPatchCatalogLayer(
      origin['catalog_layer'],
      '$context catalog layer',
    ),
    vanillaClass: vanillaClass,
    sourceSeal: _itemPatchSeal(origin['source_seal'], '$context source seal'),
    fields: Map<String, AuthoringRevision3ItemScalarValue>.unmodifiable(
      parsedFields,
    ),
  );
}

String? _itemPatchFindTarget(
  Map<String, Object?> entities,
  String vanillaClass, {
  required String except,
}) {
  for (final entry in entities.entries) {
    if (entry.key == except) continue;
    final entity = _authoringRequiredObject(
      entry.value,
      'item-patch scanned entity',
    );
    final payload = _authoringRequiredObject(
      entity['payload'],
      'item-patch scanned payload',
    );
    if (payload['kind'] != 'item_patch') continue;
    final facts = _itemPatchEntityFacts(
      entry.value,
      expectedId: entry.key,
      context: 'item-patch scanned entity',
    );
    if (facts.vanillaClass == vanillaClass) return entry.key;
  }
  return null;
}

void _itemPatchRequireExactCandidate(
  Map<String, Object?> basis,
  Map<String, Object?> candidate, {
  required AuthoringRevision3ItemPatchRequestV1 request,
  required int? expectedEntityRevision,
}) {
  for (final field in const <String>[
    'format',
    'schema_revision',
    'project_id',
    'meta',
    'target',
    'authoring_locales',
    'asset_store',
  ]) {
    if (jsonEncode(basis[field]) != jsonEncode(candidate[field])) {
      throw FormatException(
        'item-patch candidate changed unrelated project field $field',
      );
    }
  }
  final basisRevision = basis['revision'];
  if (basisRevision is! int || candidate['revision'] != basisRevision + 1) {
    throw const FormatException(
      'item-patch candidate did not advance the project revision exactly',
    );
  }
  final basisEntities = _authoringRequiredObject(
    basis['entities'],
    'item-patch basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'item-patch candidate entities',
  );
  final expectedDelta = switch (request.action) {
    AuthoringRevision3ItemPatchAction.remove => -1,
    AuthoringRevision3ItemPatchAction.upsert
        when request.expectedEntityRevision == null =>
      1,
    AuthoringRevision3ItemPatchAction.upsert => 0,
  };
  if (candidateEntities.length != basisEntities.length + expectedDelta) {
    throw const FormatException(
      'item-patch candidate changed the entity count incorrectly',
    );
  }
  for (final entry in basisEntities.entries) {
    if (entry.key == request.entityId) continue;
    if (jsonEncode(candidateEntities[entry.key]) != jsonEncode(entry.value)) {
      throw const FormatException(
        'item-patch candidate changed an unrelated entity',
      );
    }
  }
  if (request.action == AuthoringRevision3ItemPatchAction.remove) {
    if (candidateEntities.containsKey(request.entityId)) {
      throw const FormatException(
        'item-patch removal candidate retained its entity',
      );
    }
    return;
  }
  final expectedEntity = <String, Object?>{
    'id': request.entityId,
    'display_name': request.displayName,
    'origin': <String, Object?>{
      'type': 'vanilla',
      'generation': jsonDecode(request.expectedTargetCanonicalJson),
      'catalog_layer': request.catalogLayer,
      'canonical_selector': request.vanillaClass,
      'source_seal': _itemPatchSealJson(request.sourceSeal),
    },
    'revision': expectedEntityRevision,
    'payload': <String, Object?>{
      'kind': 'item_patch',
      'data': <String, Object?>{
        'vanilla_class': request.vanillaClass,
        'fields': <String, Object?>{
          for (final entry in request.fields.entries)
            entry.key: entry.value._toJson(),
        },
      },
    },
  };
  if (jsonEncode(candidateEntities[request.entityId]) !=
      jsonEncode(expectedEntity)) {
    throw const FormatException(
      'item-patch candidate entity disagrees with its exact request',
    );
  }
}
