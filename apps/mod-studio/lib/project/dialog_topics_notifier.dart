import 'package:flutter_riverpod/legacy.dart';

/// One explicitly authored conversation-topic registration.
///
/// The identity values and visibility policy are passed through verbatim to
/// `BuildSpec.dialog_topics`. In particular, the editor never guesses a
/// participant or sentinel from the authored topic class.
class DialogTopicDefinition {
  const DialogTopicDefinition({
    required this.id,
    required this.participantName,
    required this.topicClass,
    required this.sentinelClass,
    this.allowHidden = false,
  });

  /// Human-readable diagnostic identifier. IDs are unique case-insensitively.
  final String id;

  /// Exact stable value returned by `ConversationGroup.GetParticipantName`.
  final String participantName;

  /// Exact reflected UClass path for the authored AngelScript topic.
  final String topicClass;

  /// Exact reflected UClass path for a vanilla topic proving locality.
  final String sentinelClass;

  /// Whether a state-dependent topic may be cleanly absent from the visible
  /// choice array after registration.
  final bool allowHidden;

  String get key => id.toLowerCase();

  Map<String, Object?> toJson() => {
    'id': id,
    'participant_name': participantName,
    'topic_class': topicClass,
    'sentinel_class': sentinelClass,
    if (allowHidden) 'allow_hidden': true,
  };

  factory DialogTopicDefinition.fromJson(Map<String, Object?> json) =>
      DialogTopicDefinition(
        id: json['id'] as String,
        participantName: json['participant_name'] as String,
        topicClass: json['topic_class'] as String,
        sentinelClass: json['sentinel_class'] as String,
        allowHidden: (json['allow_hidden'] as bool?) ?? false,
      );
}

class DialogTopicsState {
  const DialogTopicsState({this.items = const {}});

  /// Case-folded diagnostic ID to definition, in user-authored insertion order.
  final Map<String, DialogTopicDefinition> items;

  int get count => items.length;
  List<DialogTopicDefinition> get entries =>
      items.values.toList(growable: false);

  DialogTopicsState copyWith({Map<String, DialogTopicDefinition>? items}) =>
      DialogTopicsState(items: items ?? this.items);
}

class DialogTopicsNotifier extends StateNotifier<DialogTopicsState> {
  DialogTopicsNotifier() : super(const DialogTopicsState());

  /// Add [topic], or update the same case-insensitive ID in place.
  void setTopic(DialogTopicDefinition topic) {
    final items = Map<String, DialogTopicDefinition>.from(state.items);
    items[topic.key] = topic;
    state = state.copyWith(items: items);
  }

  /// Replace [originalId] while retaining its position, including when its ID
  /// changes. If [originalId] is absent this behaves like [setTopic].
  void replaceTopic(String originalId, DialogTopicDefinition topic) {
    final originalKey = originalId.toLowerCase();
    if (!state.items.containsKey(originalKey)) {
      setTopic(topic);
      return;
    }

    final replacementKey = topic.key;
    if (replacementKey != originalKey &&
        state.items.containsKey(replacementKey)) {
      throw ArgumentError.value(
        topic.id,
        'topic.id',
        'a dialog topic with this ID already exists',
      );
    }
    final items = <String, DialogTopicDefinition>{};
    for (final entry in state.items.entries) {
      if (entry.key == originalKey) {
        items[replacementKey] = topic;
      } else if (entry.key != replacementKey) {
        items[entry.key] = entry.value;
      }
    }
    state = state.copyWith(items: items);
  }

  void remove(String id) {
    final key = id.toLowerCase();
    if (!state.items.containsKey(key)) return;
    final items = Map<String, DialogTopicDefinition>.from(state.items)
      ..remove(key);
    state = state.copyWith(items: items);
  }

  void clearAll() {
    if (state.items.isEmpty) return;
    state = const DialogTopicsState();
  }

  void loadAll(List<DialogTopicDefinition> topics) {
    final items = <String, DialogTopicDefinition>{};
    for (final topic in topics) {
      if (items.containsKey(topic.key)) {
        throw FormatException('duplicate dialog topic id: ${topic.id}');
      }
      items[topic.key] = topic;
    }
    state = DialogTopicsState(items: items);
  }
}

final dialogTopicsProvider =
    StateNotifierProvider<DialogTopicsNotifier, DialogTopicsState>(
      (ref) => DialogTopicsNotifier(),
    );
