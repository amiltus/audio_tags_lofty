# audio_tags_lofty

Local embedded-audio-artwork extraction for Flutter's non-web platforms.

This fork intentionally supports one operation: extract a front cover from a
local audio file. It does not play or decode audio samples, write tags, fetch
remote URLs, or create extra Dart isolates.

## Usage

Call this from a persistent background worker, not a Flutter UI isolate:

```dart
final artwork = extractFrontArtwork(
  audioPath,
  maxInputBytes: 20 * 1024 * 1024,
  maxArtworkBytes: 8 * 1024 * 1024,
);
```

`artwork` is `null` when the file has no embedded image. Invalid, unreadable,
or over-limit files throw [AudioArtworkException]. The returned bytes are the
original cover data; callers own any resizing or JPEG encoding.

For a thumbnail worker, use `withFrontArtwork` to resize and encode within its
callback. It exposes a temporary native `Uint8List` view and avoids the final
native-to-Dart copy. Do not retain or return that view.

## Security and Limits

- Only regular local files are accepted.
- The input file is rejected before parsing when it exceeds `maxInputBytes`.
- Extracted artwork is rejected when it exceeds `maxArtworkBytes`.
- The extractor prefers a `CoverFront` image and otherwise uses the first
  embedded image.

## Development

```bash
cd rust/lofty_ffi
cargo test
cargo build --release
cargo run --release --example extract_front_artwork_benchmark -- \
  tests/fixtures/with_front_artwork.mp3 500
```

Platform artifacts must be regenerated with the scripts in `scripts/` before
publishing a release.

The checked-in macOS XCFramework is 2.4 MB universal. On an Apple M3 Pro, the
release benchmark with a 123 KB embedded JPEG measured 0.056-0.141 ms p95 over
three 500-iteration runs. That measures tag parsing and copying only; resize
and JPEG encoding must be benchmarked in the consuming thumbnail worker.
