// Models for the hero Skills sub-tab: the `private.skills.list` read result and
// the declarative `private.skills.set` edit intent. Skills are GameplayEffects
// in the hero's ActiveEffects array; the core resolves each edit by skill base
// name (never a stale index), so a batch of these applies safely in one write.

/// One selectable tier for a skill. [value] is what the UI sends as `tier` and
/// what the UI composes the visible label from (via the game's localized tier
/// vocabulary). The core emits only the value; order is significant.
class SkillOption {
  const SkillOption({required this.value});

  factory SkillOption.fromJson(Map<String, Object?> json) {
    return SkillOption(value: json['value'] as String? ?? '');
  }

  final String value;
}

/// One hero skill: either learned (with its current tier) or a roster entry the
/// hero can learn. [current] is always one of [options]' values.
class Skill {
  const Skill({
    required this.base,
    required this.label,
    required this.category,
    required this.kind,
    required this.learned,
    required this.current,
    required this.hasUntrained,
    required this.options,
  });

  factory Skill.fromJson(Map<String, Object?> json) {
    return Skill(
      base: json['base'] as String? ?? '',
      label: json['label'] as String? ?? '',
      category: json['category'] as String? ?? '',
      kind: json['kind'] as String? ?? '',
      learned: json['learned'] as bool? ?? false,
      current: json['current'] as String? ?? 'Untrained',
      hasUntrained: json['hasUntrained'] as bool? ?? false,
      options:
          (json['options'] as List?)
              ?.whereType<Map>()
              .map((e) => SkillOption.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
    );
  }

  final String base;
  final String label;
  final String category;

  /// `ladder` | `circle` | `hunting` | `binary` | `language`.
  final String kind;
  final bool learned;
  final String current;
  final bool hasUntrained;
  final List<SkillOption> options;
}

/// Result of `private.skills.list`. Carries an optional [error] (set by the
/// notifier instead of throwing) so the panel renders failures inline.
class SkillsResult {
  const SkillsResult({
    this.actor = 'Hero',
    this.found = false,
    this.skills = const [],
    this.error,
  });

  factory SkillsResult.fromJson(Map<String, Object?> json) {
    return SkillsResult(
      actor: json['actor'] as String? ?? 'Hero',
      found: json['found'] as bool? ?? false,
      skills:
          (json['skills'] as List?)
              ?.whereType<Map>()
              .map((e) => Skill.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
    );
  }

  final String actor;
  final bool found;
  final List<Skill> skills;
  final String? error;

  /// Skills grouped by category, preserving the core's ordering.
  Map<String, List<Skill>> get byCategory {
    final map = <String, List<Skill>>{};
    for (final s in skills) {
      map.putIfAbsent(s.category, () => []).add(s);
    }
    return map;
  }
}

/// Pending skill change → `private.skills.set`. Declarative: the target tier for
/// one skill base. Keyed by [base] in the pending map so re-selecting the
/// original value drops the edit.
class SkillSetEdit {
  const SkillSetEdit({
    required this.base,
    required this.tier,
    this.actor = 'Hero',
  });

  final String base;
  final String tier;
  final String actor;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.skills.set',
      'value': {'actor': actor, 'base': base, 'tier': tier},
    };
  }
}
