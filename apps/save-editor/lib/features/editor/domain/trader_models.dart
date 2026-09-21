// Models for the Handel (trade) sub-tab: the `private.traders.list` /
// `private.traders.detail` read results and the three edit intents.
//
// A merchant's shop is not part of his inventory. It lives in one global array
// (`m_Traders`) keyed by the NPC's unique name, and it holds two maps: what he
// currently offers, and the baseline he restocks back toward. His ore sits in
// the same map as an ordinary line — ore is the colony's currency, and the
// amount he holds IS his purchasing power.
//
// Rows are addressed by ARRAY INDEX, never by name: two shipped rows are named
// `None` and belong to no NPC at all.

import 'package:goresave/features/editor/domain/game_time.dart';

/// The item class path of ore, which doubles as a merchant's purse.
const String kTraderOrePath = '/Script/Angelscript.ItMi_Orenugget';

/// The value used by the game before a merchant has any recorded activity.
const double kTraderNeverActiveSeconds = -1000;

/// The shipped merchant-restock interval for each known Resources difficulty.
/// Unknown future/modded levels return null rather than showing an invented
/// Gothic forecast.
int? traderRestockDays(String resourcesLevel) => switch (resourcesLevel) {
  'Novice' => 2,
  'Gothic' => 3,
  'Hard' => 5,
  _ => null,
};

enum TraderRestockForecastState {
  unavailable,
  neverActive,
  clockAhead,
  beforeWindow,
  boundaryOnly,
  eligibleBoth,
}

/// Save-visible timing around one merchant's lazy restock maintenance.
///
/// The supplied saves strongly support a calendar-day comparison, while an
/// elapsed-duration comparison is also plausible from the persisted data alone.
/// Exposing both bounds keeps the UI honest: neither one is called the reset
/// time, because the native game performs maintenance lazily when it next
/// processes the merchant.
class TraderRestockTiming {
  const TraderRestockTiming({
    required this.activitySeconds,
    required this.worldSeconds,
    required this.intervalDays,
  });

  final double activitySeconds;
  final double? worldSeconds;
  final int intervalDays;

  bool get hasRecordedActivity =>
      activitySeconds.isFinite && activitySeconds > kTraderNeverActiveSeconds;

  bool get isNeverActive =>
      activitySeconds.isFinite && activitySeconds <= kTraderNeverActiveSeconds;

  bool get hasValidInputs =>
      activitySeconds.isFinite &&
      activitySeconds >= 0 &&
      intervalDays > 0 &&
      (worldSeconds == null || (worldSeconds!.isFinite && worldSeconds! >= 0));

  bool get isFutureDated =>
      hasRecordedActivity &&
      worldSeconds != null &&
      activitySeconds > worldSeconds!;

  /// Earliest forecast: start of the Nth later calendar day.
  double? get calendarBoundarySeconds {
    if (!hasRecordedActivity || !hasValidInputs) return null;
    final activityDay = activitySeconds.floor() ~/ secondsPerDay;
    return ((activityDay + intervalDays) * secondsPerDay).toDouble();
  }

  /// Conservative forecast: a full N × 24 hours after the activity timestamp.
  double? get elapsedBoundarySeconds => !hasRecordedActivity || !hasValidInputs
      ? null
      : activitySeconds + intervalDays * secondsPerDay;

  TraderRestockForecastState get state {
    if (isNeverActive) return TraderRestockForecastState.neverActive;
    if (!hasValidInputs || worldSeconds == null) {
      return TraderRestockForecastState.unavailable;
    }
    if (isFutureDated) return TraderRestockForecastState.clockAhead;
    if (worldSeconds! < calendarBoundarySeconds!) {
      return TraderRestockForecastState.beforeWindow;
    }
    if (worldSeconds! < elapsedBoundarySeconds!) {
      return TraderRestockForecastState.boundaryOnly;
    }
    return TraderRestockForecastState.eligibleBoth;
  }

  /// Timestamp that makes this merchant eligible on the current calendar day.
  /// Returns null during the first [intervalDays] days: no non-negative activity
  /// timestamp can honestly express an already elapsed interval there.
  double? get makeDueActivitySeconds {
    final world = worldSeconds;
    if (world == null || !world.isFinite || world < 0 || intervalDays <= 0) {
      return null;
    }
    final activity = world - intervalDays * secondsPerDay - 1;
    return activity < 0 ? null : activity;
  }
}

/// Which of a trader's two stock maps an edit targets.
enum TraderStockMap {
  /// `m_Items` — what he offers right now.
  current,

  /// `m_DefaultItems` — the baseline he restocks toward.
  base;

  /// The wire value the core expects for `value.map`.
  String get wire => this == TraderStockMap.current ? 'current' : 'default';
}

/// One line of a merchant's stock: an item class and how many he holds.
class TraderItem {
  const TraderItem({
    required this.path,
    required this.id,
    required this.count,
    required this.unknownItem,
  });

  factory TraderItem.fromJson(Map<String, Object?> json) {
    return TraderItem(
      path: json['path'] as String? ?? '',
      id: json['id'] as String? ?? '',
      count: (json['count'] as num?)?.toInt() ?? 0,
      unknownItem: json['unknownItem'] as bool? ?? false,
    );
  }

  /// Full class path, i.e. the map key an edit addresses.
  final String path;

  /// Bare class name, e.g. `ItFo_Loaf`.
  final String id;
  final int count;

  /// The class is not in the bundled catalog — shown, but not offered as an
  /// edit target, because we cannot vouch for what the game does with it.
  final bool unknownItem;

  bool get isOre => path == kTraderOrePath;
}

/// A merchant as listed: enough to find one and see his purchasing power.
class TraderSummary {
  const TraderSummary({
    required this.index,
    required this.uniqueName,
    required this.itemCount,
    required this.defaultItemCount,
    required this.ore,
    required this.totalSeconds,
    required this.traded,
    required this.generatedEventCount,
    required this.placeholder,
    this.stockMapsPresent = true,
  });

  factory TraderSummary.fromJson(Map<String, Object?> json) {
    return TraderSummary(
      index: (json['index'] as num?)?.toInt() ?? 0,
      uniqueName: json['uniqueName'] as String? ?? '',
      itemCount: (json['itemCount'] as num?)?.toInt() ?? 0,
      defaultItemCount: (json['defaultItemCount'] as num?)?.toInt() ?? 0,
      ore: (json['ore'] as num?)?.toInt(),
      totalSeconds: (json['totalSeconds'] as num?)?.toDouble() ?? -1000,
      traded: json['traded'] as bool? ?? false,
      generatedEventCount: (json['generatedEventCount'] as num?)?.toInt() ?? 0,
      placeholder: json['placeholder'] as bool? ?? false,
      // Absent on an older core, where the maps were always assumed present.
      stockMapsPresent: json['stockMapsPresent'] as bool? ?? true,
    );
  }

  /// Position in `m_Traders` — the only safe address for an edit.
  final int index;
  final String uniqueName;
  final int itemCount;
  final int defaultItemCount;

  /// His ore. `null` means the record carries no ore line at all, which is a
  /// real state (Riordian, Scorpio, Xardas) and NOT the same as zero.
  final int? ore;
  final double totalSeconds;

  /// Whether the player has ever traded here. Derived from [totalSeconds]'s
  /// never-traded sentinel by the core.
  final bool traded;
  final int generatedEventCount;

  /// One of the unnamed sentinel rows, which belongs to no NPC.
  final bool placeholder;

  /// Both stock maps are present on the record. An omitted one reads as empty,
  /// which would look editable and then fail at save time — the structural
  /// appliers resolve the property and cannot create it.
  final bool stockMapsPresent;
}

/// Everything stored for one merchant.
class TraderDetail {
  const TraderDetail({
    required this.summary,
    required this.items,
    required this.defaultItems,
    required this.generatedEvents,
    required this.totalSecondsPath,
    required this.hasItemsByDifficulty,
  });

  factory TraderDetail.fromJson(Map<String, Object?> json) {
    List<TraderItem> stock(String key) =>
        (json[key] as List?)
            ?.whereType<Map>()
            .map((e) => TraderItem.fromJson(e.cast<String, Object?>()))
            .toList(growable: false) ??
        const [];
    return TraderDetail(
      summary: TraderSummary.fromJson(json),
      items: stock('items'),
      defaultItems: stock('defaultItems'),
      generatedEvents:
          (json['generatedEvents'] as List?)
              ?.whereType<String>()
              .toList(growable: false) ??
          const [],
      totalSecondsPath: (json['totalSecondsPath'] as List?)
          ?.whereType<String>()
          .toList(growable: false),
      hasItemsByDifficulty: json['hasItemsByDifficulty'] as bool? ?? false,
    );
  }

  final TraderSummary summary;

  /// Live stock. Note it also contains the ore line.
  final List<TraderItem> items;

  /// Saved input used by the game's runtime maintenance. It diverges from
  /// [items] in played saves, but it is not a durable custom-stock definition:
  /// the runtime can rebuild it from its own trader configuration. The editor
  /// therefore displays this map read-only.
  final List<TraderItem> defaultItems;
  final List<String> generatedEvents;

  /// Exact typed path returned by the core. Null against an older core or when
  /// this record has no writable DoubleProperty timestamp.
  final List<String>? totalSecondsPath;

  /// The per-difficulty staging map holds entries. Empty in every save observed
  /// so far; if this is ever true the UI must not pretend it edited everything.
  final bool hasItemsByDifficulty;

  List<TraderItem> stock(TraderStockMap map) =>
      map == TraderStockMap.current ? items : defaultItems;
}

/// Result of `private.traders.list`, carrying an inline [error] rather than
/// throwing so the panel can render a message in place.
class TradersResult {
  const TradersResult({
    this.traders = const [],
    this.writable = const {},
    this.error,
  });

  factory TradersResult.fromJson(Map<String, Object?> json) {
    return TradersResult(
      traders:
          (json['traders'] as List?)
              ?.whereType<Map>()
              .map((e) => TraderSummary.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      writable:
          (json['writable'] as List?)?.whereType<String>().toSet() ?? const {},
    );
  }

  final List<TraderSummary> traders;

  /// Which trader commands this core build offers. The app feature-detects on
  /// these instead of assuming, so an older core degrades to read-only.
  final Set<String> writable;
  final String? error;

  bool get canSetStock => writable.contains('private.traders.setStock');
  bool get canAddItem => writable.contains('private.traders.addItem');
  bool get canRemoveItem => writable.contains('private.traders.removeItem');

  /// Every non-placeholder record carrying [uniqueName].
  ///
  /// Case-insensitively, the way the core joins these names: a character's
  /// unique name is the stored knowledge key where one exists, whose casing can
  /// differ from the trader row's. An exact compare would leave a character the
  /// list badges as a merchant reading "does not trade".
  ///
  /// Placeholder rows belong to no NPC and are deliberately not matched.
  List<TraderSummary> allForUniqueName(String uniqueName) {
    final wanted = uniqueName.toLowerCase();
    return [
      for (final t in traders)
        if (!t.placeholder && t.uniqueName.toLowerCase() == wanted) t,
    ];
  }

  /// The record for an NPC, or null when he is not a merchant OR when the name
  /// is ambiguous.
  ///
  /// Ambiguity is not resolved by taking the first hit: the index this returns
  /// is what every edit is addressed by, so guessing would edit an arbitrary
  /// shop. The core refuses the same case; [isAmbiguous] tells the two apart so
  /// the panel can say which one it is.
  TraderSummary? forUniqueName(String uniqueName) {
    final matches = allForUniqueName(uniqueName);
    return matches.length == 1 ? matches.first : null;
  }

  /// More than one record carries this name, so no edit may be addressed by it.
  bool isAmbiguous(String uniqueName) =>
      allForUniqueName(uniqueName).length > 1;
}

/// Result of `private.traders.detail`.
class TraderDetailResult {
  const TraderDetailResult({this.detail, this.error});

  final TraderDetail? detail;
  final String? error;
}

/// A queued change to one stock line.
///
/// [count] is the new count for [TraderEditKind.setStock] and
/// [TraderEditKind.addItem], and unused for a removal.
class TraderStockEdit {
  const TraderStockEdit({
    required this.kind,
    required this.index,
    required this.map,
    required this.path,
    this.count = 0,
  });

  final TraderEditKind kind;
  final int index;
  final TraderStockMap map;
  final String path;
  final int count;

  String get commandPath => switch (kind) {
    TraderEditKind.setStock => 'private.traders.setStock',
    TraderEditKind.addItem => 'private.traders.addItem',
    TraderEditKind.removeItem => 'private.traders.removeItem',
  };

  /// A stable per-line key so re-editing the same line replaces its pending
  /// edit instead of queueing a second one.
  String get pendingKey => 'traders:$index:${map.wire}:$path';

  /// Insert and remove splice the map body; the core refuses to batch them with
  /// anything else, and the notifier splits them into their own writes.
  bool get isStructural => kind != TraderEditKind.setStock;

  Map<String, Object?> toEdit() {
    final value = <String, Object?>{
      'index': index,
      'path': path,
      'map': map.wire,
    };
    if (kind != TraderEditKind.removeItem) value['count'] = count;
    return {'path': commandPath, 'value': value};
  }
}

enum TraderEditKind { setStock, addItem, removeItem }

/// A queued edit of one merchant's stored activity timestamp.
///
/// This is a fixed-size DoubleProperty write. The notifier orders fixed writes
/// before structural stock additions/removals, so both can safely be saved in
/// one user operation without resolving this row after a splice.
class TraderActivityTimeEdit {
  const TraderActivityTimeEdit({
    required this.index,
    required this.propertyPath,
    required this.totalSeconds,
  });

  final int index;
  final List<String> propertyPath;
  final double totalSeconds;

  String get pendingKey => 'traders:$index:activityTime';

  Map<String, Object?> toEdit() => {
    'path': 'private.typed.setValue',
    'value': {'path': propertyPath, 'value': totalSeconds},
  };
}
