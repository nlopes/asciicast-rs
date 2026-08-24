# asciicast-rs

A library to parse [`asciicast` file
format](https://docs.asciinema.org/manual/asciicast/v3/) files across all `asciicast`
versions.

## Versions supported

- [`asciicast` v1 format](https://docs.asciinema.org/manual/asciicast/v1/)
- [`asciicast` v2 format](https://docs.asciinema.org/manual/asciicast/v2/)
- [`asciicast` v3 format](https://docs.asciinema.org/manual/asciicast/v3/)

### Why support all 3?

I wanted to be able to parse old files as well for another project I'm working on called
[acdc](https://github.com/nlopes/acdc).

## Installation

```sh
cargo add asciicast-rs
```

You can parse from:

- a byte slice using `from_slice`
- a `BufRead` using `from_reader`
- a file using `from_path`

They all return `Result<_, asciicast_rs::Error>`.

### Parsing a known version

The version is part of the type system, so you can parse directly into `Asciicast<V>`.

```rust
use asciicast_rs::{Asciicast, V2};

let recording = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.5,\"o\",\"hello\"]\n";
let cast = Asciicast::<V2>::from_slice(recording).expect("valid v2 recording");

println!("{}x{}", cast.header.width, cast.header.height);
for event in &cast.events {
    if let Some(text) = event.as_output() {
        print!("{text}");
    }
}
```

To read from a file instead, use `Asciicast::<V2>::from_path("recording.cast")`.

### Auto-detecting the version

When the version is not known ahead of time, use `AsciicastVersioned`, which detects it
from the content and yields the matching variant, each wrapping a fully typed
`Asciicast<V>`.

```rust
use asciicast_rs::AsciicastVersioned;

let recording = b"{\"version\":2,\"width\":80,\"height\":24}\n";
match AsciicastVersioned::from_slice(recording).expect("valid recording") {
    AsciicastVersioned::V1(cast) => println!("v1, {} frames", cast.events.len()),
    AsciicastVersioned::V2(cast) => println!("v2, {} events", cast.events.len()),
    AsciicastVersioned::V3(cast) => println!("v3, {} events", cast.events.len()),
}
```

### Streaming events (v2 / v3)

`from_slice`/`from_path`/`from_reader` eagerly collect every event into a `Vec`. For long
recordings, you can instead use a `Reader` (or wrappers like `v2::stream` and
`v3::stream`) which yields a `Result<Event, _>` per line as an `Iterator`, so the whole
recording is never held in memory at once. You can construct one with `Reader::<V, _>::open` or one of the wrappers mentioned.

> [!NOTE]
> Only the newline-delimited versions (v2, v3) implement `Streamable`, so `Reader<V1, _>` is a compile error (v1 is a single JSON document and is always parsed eagerly).

```rust
use asciicast_rs::v2;

let recording: &[u8] = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.5,\"o\",\"hi\"]\n";
let reader = v2::stream(recording).expect("valid v2 header");

println!("{}x{}", reader.header().width, reader.header().height);
for event in reader {
    let event = event.expect("valid event");
    if let Some(text) = event.as_output() {
        print!("{text}");
    }
}

// If you have the stream and want to materialise the eager `Asciicast` you can do:
let eager = v2::stream(recording)
    .and_then(|reader| reader.into_recording())
    .expect("valid v2 recording");
assert_eq!(eager.events.len(), 1);
```

### Working with the parsed data

```rust
use asciicast_rs::{Asciicast, V3};

let recording = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n";
let cast = Asciicast::<V3>::from_slice(recording).expect("valid v3 recording");
assert_eq!(cast.header.term.cols, 80);
```

### Absolute timestamps

Timing differs by version (see [Data model](#data-model)), so `absolute_times` normalises
it for you: it pairs each event with its absolute time in seconds since the start,
accumulating relative entries (v1, v3) and passing v2's already-absolute times through.

```rust
use asciicast_rs::{Asciicast, V3};

let recording = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n[0.1,\"o\",\"a\"]\n[0.2,\"o\",\"b\"]\n";
let cast = Asciicast::<V3>::from_slice(recording).expect("valid v3 recording");

for (at, event) in cast.absolute_times() {
    if let Some(text) = event.as_output() {
        println!("{at:.3}s: {text}");
    }
}
```

`Reader::absolute_times` is the streaming counterpart, yielding `Result<(f64, Event), _>`
so you get absolute timestamps without buffering the recording.

## Data model

- `Asciicast<V>` is `{ header, events }`, parameterised by a version marker (`V1`, `V2`,
  `V3`).
- Each version has its own `Header` and event type under `asciicast_rs::{v1, v2, v3}`.
- v1 frames expose `Frame::delay` and `Frame::data` directly. v2 and v3 events expose a
  typed payload plus accessors (`as_output`, `as_input`, `as_marker`, `as_resize`, and
  also `as_exit` for v3).
- Shared types live in `asciicast_rs::common` (`Theme`, `Rgb`, `Resize`, `ExitStatus`,
  `Env`, and the colour error types).
- Timing semantics follow the spec: v2 event `time` is absolute (seconds since start),
  while v1 frame `delay` and v3 event `interval` are relative to the previous entry. Use
  `Asciicast::absolute_times` to iterate events with a uniform absolute timestamp.

> [!NOTE]
> In v1, the nomenclature used is attributes and frames instead of header and events (_roughly_). I thought that keeping to header and events across the versions was fine but isn't strictly accurate.

## Feature flags

- `zstd` *(on by default)* — transparently reads zstd-compressed recordings. Detection is
  automatic (the zstd magic bytes) across `from_slice`/`from_reader`/`from_path`,
  `AsciicastVersioned`, and the streaming `Reader`; uncompressed input is unaffected.
  Decoding is pure Rust (via [`ruzstd`](https://crates.io/crates/ruzstd)), so there is no C
  toolchain dependency.

  Decoding is streaming: a `Reader` holds only one event at a time, while the eager `from_*`
  constructors load the whole recording into memory (just as they do for uncompressed
  input). Because compressed input can expand greatly, prefer the streaming `Reader` for
  large or untrusted recordings.

  Opt out for a smaller, JSON-only build:

  ```sh
  cargo add asciicast-rs --no-default-features
  ```

- `chrono` *(off by default)* — adds a `timestamp_datetime()` accessor to the v2 and v3
  headers, returning the recording's start time as a `chrono::DateTime<Utc>`:

  ```sh
  cargo add asciicast-rs --features chrono
  ```

## What this crate is not

A way to serialize `asciicast` format to files. Reason being that I wanted this crate to
start with the smallest possible "features", whilst being complete in terms of parsing, in
case one day the `asciinema` project decides to extract their serialization and parsing
into its own library and crate.

## Motivation

I needed a parser for `asciicast` files but realized there wasn't one (that I could easily
find) that was obvious I should use. More notes on what I found:

- `asciinema` is [built in rust](https://github.com/asciinema/asciinema) but unfortunately
  it doesn't expose the `asciicast` format publicly as a library.

- There was also a library named [`asciicast`](https://crates.io/crates/asciicast) but
  unfortunately doesn't seem to get any more updates.

Therefore I decided to create this crate to try to become the canonical rust library for
parsing `asciicast` format. If one day `asciinema` decides to provide a public crate then
I'd be happy to stop work here.
