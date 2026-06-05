# asciicast-rs

A library that provides the [`asciicast` file
format](https://docs.asciinema.org/manual/asciicast/v3/) data structures, and parsing
functionality for the various versions (v1, v2, and v3).

## What is this for

Purely a way to parse and deserialize `asciicast` formats. We support:

- [asciicast v1 format](https://docs.asciinema.org/manual/asciicast/v1/)
- [asciicast v2 format](https://docs.asciinema.org/manual/asciicast/v2/)
- [asciicast v3 format](https://docs.asciinema.org/manual/asciicast/v3/)

### Why support all 3?

I wanted to be able to parse old files as well for the project that I need this for
([acdc](https://github.com/nlopes/acdc)).

## Installation

```sh
cargo add asciicast-rs
```

## Usage

### Parsing a known version

The version is part of the type system, so you can parse directly into `Asciicast<V>`.

```rust
use asciicast_rs::{Asciicast, V2};

let cast = Asciicast::<V2>::from_path("recording.cast")?;

println!("{}x{}", cast.header.width, cast.header.height);
for event in &cast.events {
    if let Some(text) = event.as_output() {
        print!("{text}");
    }
}
```

### Auto-detecting the version

When the version is not known ahead of time, you can use `AsciicastVersioned`, which
detects it from the content and yields the matching variant, each wrapping a fully typed
`Asciicast<V>`.

```rust
use asciicast_rs::AsciicastVersioned;

match AsciicastVersioned::from_path("recording.cast")? {
    AsciicastVersioned::V1(cast) => println!("v1, {} frames", cast.events.len()),
    AsciicastVersioned::V2(cast) => println!("v2, {} events", cast.events.len()),
    AsciicastVersioned::V3(cast) => println!("v3, {} events", cast.events.len()),
}
```

### Parsing from bytes / reader

```rust
use asciicast_rs::{Asciicast, V3};

let bytes: &[u8] = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n";
let cast = Asciicast::<V3>::from_slice(bytes)?;
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
> In v1, the nomenclature is attributes and frames instead of header and events (_roughly_). I thought that keeping to header and events across any of the versions was fine.

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
