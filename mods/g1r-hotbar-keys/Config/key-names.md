# Key names for `config.lua`

These are not Gothic identifiers — they are Unreal's own `FKey` names, fixed by the engine
(UE 5.4.3 here). The list below was read out of the shipped executable
(`G1R-Win64-Shipping.exe`), so it is exactly what this build knows.

Spelling matters: an unknown name produces an invalid key, and the slot ends up on no key
at all. The mod warns in the log if a name is not in this list.

## Mouse

```
LeftMouseButton   RightMouseButton   MiddleMouseButton
ThumbMouseButton  ThumbMouseButton2
MouseScrollUp     MouseScrollDown
```

`MouseX`, `MouseY`, `Mouse2D` and `MouseWheelAxis` also exist but are axes, not buttons —
useless for a hotbar slot.

## Letters and digits

```
A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Zero One Two Three Four Five Six Seven Eight Nine
```

The digits are the number row. Note the game already uses all ten of them: `One` and `Two`
draw the melee and ranged weapon, `Three` .. `Zero` are the eight hotbar slots.

## Numpad

```
NumPadZero NumPadOne NumPadTwo NumPadThree NumPadFour
NumPadFive NumPadSix NumPadSeven NumPadEight NumPadNine
Multiply Add Subtract Decimal Divide
```

## Function keys

```
F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12
```

Careful with `F1`: the game may already use some function keys.

## Modifiers and navigation

```
LeftShift   RightShift   LeftControl RightControl
LeftAlt     RightAlt     LeftCommand RightCommand
Tab  CapsLock  SpaceBar  Enter  BackSpace  Escape  Pause
Insert  Delete  Home  End  PageUp  PageDown
Left  Right  Up  Down
NumLock  ScrollLock
```

A modifier on its own is a poor hotbar key — Enhanced Input has no chord support in this
context, so `LeftShift` means the bare key, not "Shift + something".

## Punctuation

```
Semicolon Equals Comma Underscore Hyphen Period Slash Tilde
LeftBracket RightBracket Backslash Apostrophe Quote Colon
LeftParantheses RightParantheses Asterix Ampersand Caret Dollar Exclamation
```

The engine's own spelling errors — `Asterix`, `LeftParantheses`, `RightParantheses` — are
part of the name. Type them as shown.

These names are positional on a US layout. On a German keyboard `Tilde` is the key left of
`1`, `LeftBracket` is `ü`, `Semicolon` is `ö`, `Apostrophe` is `ä`, and `Slash` is `-`.

## Accented

```
A_AccentGrave E_AccentGrave E_AccentAigu C_Cedille
```

## Gamepad

Names beginning with `Gamepad_`, for example `Gamepad_FaceButton_Bottom`,
`Gamepad_DPad_Up`, `Gamepad_LeftShoulder`, `Gamepad_RightTrigger`. There is no gamepad
mapping context for the hotbar in this build, so putting one here has no effect.

The executable also carries names for VR controllers (Vive, Oculus Touch, Valve Index,
Mixed Reality), touch, tilt and Steam Controller. Irrelevant here.

## Checking against the game

Two ways to see names that are definitely correct for this build:

- Run the game with the mod and read the `[g1r-hotbar-keys]` lines in `ue4ss/UE4SS.log`.
  It prints every mapping of the hotbar context with its real key name.
- Re-read them from the executable yourself; the block starts at the string `AnyKey` and
  runs contiguously to `Gamepad_RightStick_Left`.
