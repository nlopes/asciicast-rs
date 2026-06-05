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

### Working with the parsed data

```rust
use asciicast_rs::{Asciicast, V3};

let recording = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n";
let cast = Asciicast::<V3>::from_slice(recording).expect("valid v3 recording");
assert_eq!(cast.header.term.cols, 80);
```

## Data model

- `Asciicast<V>` is `{ header, events }`, parameterised by a version marker (`V1`, `V2`,
  `V3`).
- Each version has its own `Header` and event type under `asciicast_rs::{v1, v2, v3}`.
  Events expose a typed payload plus accessors (`as_output`, `as_input`, `as_marker`,
  `as_resize`, and also `as_exit` for v3).
- Shared types live in `asciicast_rs::common` (`Theme`, `Rgb`, `Resize`, `ExitStatus`,
  `Env`, and the colour error types).
- Timing semantics follow the spec: v2 event `time` is absolute (seconds since start),
  while v1 frame `delay` and v3 event `interval` are relative to the previous entry.

> [!NOTE]
> In v1, the nomenclature used is attributes and frames instead of header and events (_roughly_). I thought that keeping to header and events across the versions was fine but isn't strictly accurate.

## Feature flags

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
