# Changelog

All notable changes to `asciicast-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-06-06

### Added
- Stream events from large recordings without loading them all into memory, via
  `v2::stream` / `v3::stream` (or `Reader`).
- Get each event's absolute timestamp regardless of version with `absolute_times`, on a
  parsed recording or a stream.

### Changed

> [!IMPORTANT]
> `EventCode`, `EventPayload`, `ColorError`, and `PaletteError` are now
> `#[non_exhaustive]`. If you `match` on any of them, add a wildcard (`_`) arm.

## [0.1.1] - 2026-06-05

- Improved the README content with details on how to use the library.


## [0.1.0] - 2026-06-05

- Initial release with support for all 3 formats of `asciicast` (v1, v2, v3)

[Unreleased]: https://github.com/nlopes/asciicast-rs/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nlopes/asciicast-rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/nlopes/asciicast-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/nlopes/asciicast-rs/releases/tag/v0.1.0
