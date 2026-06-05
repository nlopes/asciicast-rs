# asciicast-rs

A library that provides the [`asciicast` file
format](https://docs.asciinema.org/manual/asciicast/v3/) data structures, and parsing
functionality for the various versions (v1, v2, and v3).

## What is this for

Purely a way to parse and deserialize `asciicast` formats. We support:

- [v1](https://docs.asciinema.org/manual/asciicast/v1/)
- [v2](https://docs.asciinema.org/manual/asciicast/v2/)
- [v3](https://docs.asciinema.org/manual/asciicast/v3/)

### Why support all 3?

I wanted to be able to parse old files as well for the project that I need this for
([acdc](https://github.com/nlopes/acdc)).

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
