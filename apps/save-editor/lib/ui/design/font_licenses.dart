import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Exposes bundled font licenses through Flutter's standard license page.
void registerBundledFontLicenses() {
  LicenseRegistry.addLicense(() async* {
    yield LicenseEntryWithLineBreaks(const [
      'Podkova',
    ], await rootBundle.loadString('assets/licenses/Podkova-OFL.txt'));
    yield LicenseEntryWithLineBreaks(const [
      'Noto Serif',
    ], await rootBundle.loadString('assets/licenses/NotoSerif-OFL.txt'));
    yield LicenseEntryWithLineBreaks(const [
      'Noto Serif JP',
    ], await rootBundle.loadString('assets/licenses/NotoSerifJP-OFL.txt'));
    yield LicenseEntryWithLineBreaks(const [
      'Noto Serif SC',
    ], await rootBundle.loadString('assets/licenses/NotoSerifSC-OFL.txt'));
  });
}
