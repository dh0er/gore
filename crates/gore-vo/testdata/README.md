# Synthetic Ogg fixtures

These tiny files contain an 80 ms, 440 Hz mathematically generated mono tone. They contain no
game audio or other copyrighted source recording. They are checked in so tests never require
`ffmpeg` at runtime.

They were generated with an ffmpeg build providing `libvorbis` and `libopus`:

```text
ffmpeg -hide_banner -loglevel error -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.08 -ac 1 -c:a libvorbis -q:a 2 -map_metadata -1 -fflags +bitexact -flags:a +bitexact -y crates/gore-vo/testdata/tiny-vorbis.ogg
ffmpeg -hide_banner -loglevel error -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.08 -ac 1 -c:a libopus -b:a 16k -application voip -frame_duration 20 -map_metadata -1 -fflags +bitexact -flags:a +bitexact -y crates/gore-vo/testdata/tiny-opus.ogg
```

SHA-256:

- `tiny-vorbis.ogg`: `ec058ec0bad9ef11e7c0a55f9a8b02d4fef8672599023dea84c8f32acddf57c0`
- `tiny-opus.ogg`: `9670f66510385e81d25937ffe520cb7e2e56701b4cf8e1a9be57f100a0467377`

`validate_ogg` decode-probes all Vorbis audio packets with `lewton`. For Opus it validates the
complete `OpusHead`/`OpusTags` sequence, packet framing and duration, and granule consistency, but
does not decode the SILK/CELT payload. Opus acceptance therefore remains structural decodability
evidence, not a proof of playback.
