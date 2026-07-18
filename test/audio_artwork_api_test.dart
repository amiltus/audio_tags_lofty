import 'package:audio_tags_lofty/audio_tags_lofty.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('rejects an empty path before loading the native library', () {
    expect(
      () => extractFrontArtwork('', maxInputBytes: 1, maxArtworkBytes: 1),
      throwsArgumentError,
    );
  });

  test('rejects non-positive limits before loading the native library', () {
    expect(
      () => extractFrontArtwork(
        'audio.mp3',
        maxInputBytes: 0,
        maxArtworkBytes: 1,
      ),
      throwsArgumentError,
    );
    expect(
      () => extractFrontArtwork(
        'audio.mp3',
        maxInputBytes: 1,
        maxArtworkBytes: 0,
      ),
      throwsArgumentError,
    );
  });
}
