# Changelog

All notable changes to `asciicast-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Marked the `EventCode`, `EventPayload`, `ColorError`, and `PaletteError` enums as
  `#[non_exhaustive]` so future variants can be added without breaking downstream code.

## [0.1.1] - 2026-06-05

- Improved the README content with details on how to use the library.


## [0.1.0] - 2026-06-05

- Initial release with support for all 3 formats of `asciicast` (v1, v2, v3)

[Unreleased]: https://github.com/nlopes/asciicast-rs/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/nlopes/asciicast-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/nlopes/asciicast-rs/releases/tag/v0.1.0
