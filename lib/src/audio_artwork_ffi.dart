import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

/// Thrown when the native artwork extractor rejects or cannot read an input.
final class AudioArtworkException implements Exception {
  const AudioArtworkException(this.message);

  final String message;

  @override
  String toString() => 'AudioArtworkException: $message';
}

final class _ArtworkResult extends Struct {
  external Pointer<Uint8> data;

  @Size()
  external int len;

  external Pointer<Utf8> error;
}

typedef _ExtractFrontArtworkNative =
    Pointer<_ArtworkResult> Function(
      Pointer<Utf8> path,
      Uint64 maxInputBytes,
      Uint64 maxArtworkBytes,
    );
typedef _ExtractFrontArtworkDart =
    Pointer<_ArtworkResult> Function(
      Pointer<Utf8> path,
      int maxInputBytes,
      int maxArtworkBytes,
    );

typedef _FreeArtworkResultNative = Void Function(Pointer<_ArtworkResult>);
typedef _FreeArtworkResultDart = void Function(Pointer<_ArtworkResult>);

DynamicLibrary _loadLibrary() {
  if (Platform.isAndroid) {
    return DynamicLibrary.open('liblofty_ffi.so');
  }
  if (Platform.isWindows) {
    return DynamicLibrary.open('lofty_ffi.dll');
  }
  if (Platform.isMacOS || Platform.isIOS) {
    return DynamicLibrary.process();
  }
  throw UnsupportedError(
    'audio_tags_lofty is unsupported on ${Platform.operatingSystem}.',
  );
}

final _library = _loadLibrary();
final _extractFrontArtwork = _library
    .lookupFunction<_ExtractFrontArtworkNative, _ExtractFrontArtworkDart>(
      'lofty_extract_front_artwork',
    );
final _freeArtworkResult = _library
    .lookupFunction<_FreeArtworkResultNative, _FreeArtworkResultDart>(
      'lofty_free_artwork_result',
    );

/// Extracts embedded front-cover bytes from a local audio file.
///
/// This is synchronous native work. Call it from a persistent background
/// worker, not a Flutter UI isolate. It never performs network I/O, decodes
/// audio samples, or writes tags. A `null` result means the file has no cover.
Uint8List? extractFrontArtwork(
  String path, {
  required int maxInputBytes,
  required int maxArtworkBytes,
}) {
  return withFrontArtwork(
    path,
    maxInputBytes: maxInputBytes,
    maxArtworkBytes: maxArtworkBytes,
    onArtwork: Uint8List.fromList,
  );
}

/// Runs [onArtwork] synchronously over native front-cover bytes.
///
/// This avoids copying the native output into a Dart-owned [Uint8List]. The
/// view is valid only for the duration of [onArtwork]; it must not be retained
/// or returned. This is the preferred API for a thumbnail worker that resizes
/// and encodes the cover before the callback returns.
T? withFrontArtwork<T>(
  String path, {
  required int maxInputBytes,
  required int maxArtworkBytes,
  required T Function(Uint8List artwork) onArtwork,
}) {
  if (path.isEmpty) {
    throw ArgumentError.value(path, 'path', 'Must not be empty.');
  }
  if (maxInputBytes <= 0 || maxArtworkBytes <= 0) {
    throw ArgumentError('Input and artwork limits must be positive.');
  }

  final nativePath = path.toNativeUtf8();
  try {
    final resultPointer = _extractFrontArtwork(
      nativePath,
      maxInputBytes,
      maxArtworkBytes,
    );
    if (resultPointer == nullptr) {
      throw const AudioArtworkException('Native extractor returned no result.');
    }
    try {
      final result = resultPointer.ref;
      if (result.error != nullptr) {
        throw AudioArtworkException(result.error.toDartString());
      }
      if (result.data == nullptr || result.len == 0) {
        return null;
      }
      return onArtwork(result.data.asTypedList(result.len));
    } finally {
      _freeArtworkResult(resultPointer);
    }
  } finally {
    calloc.free(nativePath);
  }
}
