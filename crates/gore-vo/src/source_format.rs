//! Naming the file a caller handed to the Ogg validator when it is not an Ogg.
//!
//! The first thing a voice actor tries is a WAV, because that is what every recording tool writes.
//! What came back was `invalid Ogg capture pattern at byte 0`, which names neither the format she
//! supplied nor the one the archive needs: "I know what a WAV is; I do not know what a capture
//! pattern is." Her session stopped there. Recognizing the handful of leading-byte signatures that
//! are unambiguous costs twelve bytes and turns that dead end into a command she can paste.
//!
//! Only signatures that identify a format on their own are listed. Anything else stays
//! [`SourceFormat::Unrecognized`], which still says what is required and how to convert.

/// A payload that is not an Ogg stream, as far as its leading bytes can be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    /// RIFF/WAVE, including the RF64 variant large recorders write past 4 GiB.
    Wav,
    /// FORM/AIFF or FORM/AIFC, what macOS recording tools write.
    Aiff,
    /// Native FLAC (`fLaC`), not FLAC inside an Ogg container.
    Flac,
    /// MPEG-1/2/2.5 Audio Layer III, tagged with ID3v2 or a bare frame header.
    Mp3,
    /// ISO base media (`ftyp`) — an `.m4a`, `.mp4` or `.aac` export.
    Mp4,
    /// Nothing this validator can name from its first bytes.
    Unrecognized,
}

impl SourceFormat {
    /// Identify a non-Ogg payload from its leading bytes.
    ///
    /// Reads at most twelve bytes and allocates nothing, so it is safe to run before any size
    /// limit. Callers pass the whole payload; short and empty slices fall through to
    /// [`Self::Unrecognized`].
    #[must_use]
    pub fn detect(data: &[u8]) -> Self {
        // RIFF alone is also AVI and several other containers, so the WAVE form type decides.
        if (data.starts_with(b"RIFF") || data.starts_with(b"RF64"))
            && data.get(8..12) == Some(b"WAVE".as_slice())
        {
            return Self::Wav;
        }
        if data.starts_with(b"FORM") && matches!(data.get(8..12), Some(b"AIFF") | Some(b"AIFC")) {
            return Self::Aiff;
        }
        if data.starts_with(b"fLaC") {
            return Self::Flac;
        }
        if data.starts_with(b"ID3") || is_mp3_frame_header(data) {
            return Self::Mp3;
        }
        if data.get(4..8) == Some(b"ftyp".as_slice()) {
            return Self::Mp4;
        }
        Self::Unrecognized
    }

    /// The opening clause of the error: what the payload is, in the reader's words.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Wav => "the payload is a WAV file (RIFF/WAVE), not an Ogg stream",
            Self::Aiff => "the payload is an AIFF file, not an Ogg stream",
            Self::Flac => "the payload is a bare FLAC file, not an Ogg stream",
            Self::Mp3 => "the payload is an MP3 file, not an Ogg stream",
            Self::Mp4 => "the payload is an MP4/M4A file, not an Ogg stream",
            Self::Unrecognized => {
                "the payload is not an Ogg stream, and its first bytes match no format this \
                 validator recognizes"
            }
        }
    }

    /// A conversion command that produces an accepted payload from this format.
    ///
    /// The flags are the documented ones, not remembered ones: `-c:a libvorbis` selects the
    /// libvorbis encoder, `-q:a` is that encoder's VBR quality (documented range -1.0 to 10.0),
    /// `-ar` sets the output sample rate and `-ac` the channel count.
    ///
    /// 48 kHz mono is not a guess either. Every one of the 134,297 Ogg entries in the five archives
    /// that ship under `G1R\Story\VoiceOver` — german_new, english_newer, foreign, polish, russian —
    /// is mono 48 kHz Vorbis, with a nominal bitrate of 80 kbit/s. Nothing here proves the engine
    /// rejects another rate; matching what ships is simply the choice that needs no proof.
    #[must_use]
    pub const fn ffmpeg_command(self) -> &'static str {
        match self {
            Self::Wav | Self::Unrecognized => {
                "ffmpeg -i line.wav -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg"
            }
            Self::Aiff => "ffmpeg -i line.aiff -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg",
            Self::Flac => "ffmpeg -i line.flac -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg",
            Self::Mp3 => "ffmpeg -i line.mp3 -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg",
            Self::Mp4 => "ffmpeg -i line.m4a -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg",
        }
    }
}

/// Recognize a bare MPEG Audio Layer III frame header.
///
/// Deliberately narrow: eleven sync bits, a version that is not the reserved `01`, and layer bits
/// that mean Layer III specifically. A file whose first two bytes merely look like a sync word is
/// left [`SourceFormat::Unrecognized`] rather than mislabelled as an MP3.
fn is_mp3_frame_header(data: &[u8]) -> bool {
    let Some(&[first, second]) = data.get(..2) else {
        return false;
    };
    first == 0xff
        && second & 0xe0 == 0xe0
        && second >> 3 & 0b11 != 0b01
        && second >> 1 & 0b11 == 0b01
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reported failure: a voice actor handed `replace` the WAV her recording tool wrote and
    /// was told about a capture pattern at byte 0 instead of about WAV.
    #[test]
    fn a_wav_header_is_reported_as_a_wav_with_an_ffmpeg_line() {
        let mut wav = Vec::from(*b"RIFF");
        wav.extend_from_slice(&36u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");

        let format = SourceFormat::detect(&wav);
        assert_eq!(format, SourceFormat::Wav);
        assert!(format.describe().contains("WAV"));
        assert_eq!(
            format.ffmpeg_command(),
            "ffmpeg -i line.wav -c:a libvorbis -ar 48000 -ac 1 -q:a 5 line.ogg"
        );
    }

    /// RF64 is the WAV a long take gets written as once it passes 4 GiB; it must not be reported as
    /// an unrecognized format merely because its magic differs.
    #[test]
    fn an_rf64_header_is_still_reported_as_a_wav() {
        let mut wav = Vec::from(*b"RF64");
        wav.extend_from_slice(&u32::MAX.to_le_bytes());
        wav.extend_from_slice(b"WAVEds64");
        assert_eq!(SourceFormat::detect(&wav), SourceFormat::Wav);
    }

    #[test]
    fn the_other_named_formats_are_recognized_from_their_signatures() {
        for (bytes, expected) in [
            (b"FORM\0\0\0\x24AIFFCOMM".as_slice(), SourceFormat::Aiff),
            (b"FORM\0\0\0\x24AIFCFVER".as_slice(), SourceFormat::Aiff),
            (b"fLaC\0\0\0\x22".as_slice(), SourceFormat::Flac),
            (b"ID3\x03\0\0\0\0\0\x0f".as_slice(), SourceFormat::Mp3),
            (b"\xff\xfb\x90\x00".as_slice(), SourceFormat::Mp3),
            (b"\0\0\0\x20ftypM4A ".as_slice(), SourceFormat::Mp4),
        ] {
            assert_eq!(SourceFormat::detect(bytes), expected);
        }
    }

    /// A guessed signature is worse than no signature: a file that only looks like an MPEG sync
    /// word must not be named MP3, and the generic message must still carry the conversion.
    #[test]
    fn an_unnameable_payload_says_so_and_still_carries_the_conversion() {
        for bytes in [
            b"".as_slice(),
            b"RIFF\0\0\0\0AVI ".as_slice(),
            b"\xff\xff\xff\xff".as_slice(), // sync bits, but reserved version and layer
            b"\xff\xf9\x00\x00".as_slice(), // ADTS AAC: Layer 00, not Layer III
            b"OggS".as_slice(),
        ] {
            let format = SourceFormat::detect(bytes);
            assert_eq!(format, SourceFormat::Unrecognized);
            assert!(format.describe().contains("not an Ogg stream"));
            assert!(format.ffmpeg_command().contains("-c:a libvorbis"));
        }
    }

    /// Every arm must name a rate and channel count matching the shipped archives, which are mono
    /// 48 kHz Vorbis throughout; a command that omits them silently produces a stereo 44.1 kHz file.
    #[test]
    fn every_conversion_targets_mono_48_khz_vorbis() {
        for format in [
            SourceFormat::Wav,
            SourceFormat::Aiff,
            SourceFormat::Flac,
            SourceFormat::Mp3,
            SourceFormat::Mp4,
            SourceFormat::Unrecognized,
        ] {
            let command = format.ffmpeg_command();
            assert!(command.starts_with("ffmpeg -i line."), "{command}");
            assert!(command.contains("-c:a libvorbis"), "{command}");
            assert!(command.contains("-ar 48000"), "{command}");
            assert!(command.contains("-ac 1"), "{command}");
            assert!(command.ends_with("line.ogg"), "{command}");
        }
    }
}
