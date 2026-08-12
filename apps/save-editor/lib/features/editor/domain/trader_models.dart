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

/// The item class path of ore, which doubles as a merchant's purse.
const String kTraderOrePath = '/Script/Angelscript.ItMi_Orenugget';

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
}

/// Everything stored for one merchant.
class TraderDetail {
  const TraderDetail({
    required this.summary,
    required this.items,
    required this.defaultItems,
    required this.generatedEvents,
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
      hasItemsByDifficulty: json['hasItemsByDifficulty'] as bool? ?? false,
    );
  }

  final TraderSummary summary;

  /// Live stock. Note it also contains the ore line.
  final List<TraderItem> items;

  /// Restock baseline. Diverges from [items] in played saves in both values and
  /// key set, so it is a separate editing surface rather than a mirror.
  final List<TraderItem> defaultItems;
  final List<String> generatedEvents;

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

  /// The record for an NPC, or null when he is not a merchant. Placeholder rows
  /// belong to no NPC and are deliberately not matched.
  TraderSummary? forUniqueName(String uniqueName) {
    // Case-insensitively, the way the core joins these names: a character's
    // unique name is the stored knowledge key where one exists, whose casing can
    // differ from the trader row's. An exact compare would leave a character the
    // list badges as a merchant reading "does not trade".
    final wanted = uniqueName.toLowerCase();
    for (final t in traders) {
      if (!t.placeholder && t.uniqueName.toLowerCase() == wanted) return t;
    }
    return null;
  }
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
