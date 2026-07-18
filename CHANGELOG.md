## 0.1.0-dev.0

* Breaking: reduce the package to bounded local front-artwork extraction.
* Remove tag writing, metadata hydration, HTTP fetching, implicit
  `Isolate.run`, global FFI error state, and first-picture-only selection.
* Pin `lofty 0.22.4`; `lofty 0.24.0` does not compile on Rust 1.88.
* Rebuild stripped Android, iOS, macOS, and Windows FFI artifacts with the
  bounded artwork ABI; Linux is intentionally unsupported.

## 0.0.1

* Implement basic read/write functionality

## 0.0.2

* support read/write http file

## 0.0.3

* add format and album artist
* add support for throwing error messages
* add support for HTTP authentication

## 0.0.4

* fix: unalign len on armeabi-v7a
* use header to replace username and password
* fix: retrieving incorrect HTTP file length

## 0.0.5

* feat: add support for unsyncedlyrics in flac

## 0.0.6

* fix: update ios and macos Info.plist for app store compliance
