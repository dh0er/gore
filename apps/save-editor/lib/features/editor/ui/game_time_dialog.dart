import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Shared day/hour/minute/second editor used by the world clock and trader
/// activity timestamps.
class GameTimeDialog extends StatefulWidget {
  const GameTimeDialog({super.key, required this.initialValue, this.title});

  final GameTimeParts initialValue;
  final String? title;

  @override
  State<GameTimeDialog> createState() => _GameTimeDialogState();
}

class _GameTimeDialogState extends State<GameTimeDialog> {
  late final TextEditingController _day;
  late final TextEditingController _hour;
  late final TextEditingController _minute;
  late final TextEditingController _second;
  String? _error;

  @override
  void initState() {
    super.initState();
    _day = TextEditingController(text: widget.initialValue.day.toString());
    _hour = TextEditingController(text: widget.initialValue.hour.toString());
    _minute = TextEditingController(
      text: widget.initialValue.minute.toString(),
    );
    _second = TextEditingController(
      text: widget.initialValue.second.toString(),
    );
  }

  @override
  void dispose() {
    _day.dispose();
    _hour.dispose();
    _minute.dispose();
    _second.dispose();
    super.dispose();
  }

  int? _field(TextEditingController controller, int max) {
    final value = int.tryParse(controller.text.trim());
    if (value == null || value < 0 || value > max) return null;
    return value;
  }

  void _submit() {
    final day = _field(_day, 1 << 30);
    final hour = _field(_hour, 23);
    final minute = _field(_minute, 59);
    final second = _field(_second, 59);
    if (day == null || hour == null || minute == null || second == null) {
      setState(() => _error = AppLocalizations.of(context).gameTimeInvalid);
      return;
    }
    Navigator.of(
      context,
    ).pop(GameTimeParts(day: day, hour: hour, minute: minute, second: second));
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);

    Widget unit({
      required Key key,
      required String label,
      required TextEditingController controller,
      TextInputAction textInputAction = TextInputAction.next,
    }) {
      return SizedBox(
        width: 112,
        child: TextField(
          key: key,
          controller: controller,
          autofocus: identical(controller, _day),
          keyboardType: TextInputType.number,
          inputFormatters: [FilteringTextInputFormatter.digitsOnly],
          textInputAction: textInputAction,
          onSubmitted: textInputAction == TextInputAction.done
              ? (_) => _submit()
              : null,
          decoration: InputDecoration(labelText: label),
        ),
      );
    }

    return AlertDialog(
      key: const ValueKey('game-time-dialog'),
      title: Row(
        children: [
          const Icon(Icons.schedule_outlined),
          const SizedBox(width: 10),
          Text(widget.title ?? l10n.gameTimeTitle),
        ],
      ),
      content: SizedBox(
        width: 500,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                unit(
                  key: const ValueKey('game-time-day-field'),
                  label: l10n.gameTimeDay,
                  controller: _day,
                ),
                unit(
                  key: const ValueKey('game-time-hour-field'),
                  label: l10n.gameTimeHours,
                  controller: _hour,
                ),
                unit(
                  key: const ValueKey('game-time-minute-field'),
                  label: l10n.gameTimeMinutes,
                  controller: _minute,
                ),
                unit(
                  key: const ValueKey('game-time-second-field'),
                  label: l10n.gameTimeSeconds,
                  controller: _second,
                  textInputAction: TextInputAction.done,
                ),
              ],
            ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 10),
                child: Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          key: const ValueKey('confirm-game-time'),
          onPressed: _submit,
          child: Text(l10n.save),
        ),
      ],
    );
  }
}
