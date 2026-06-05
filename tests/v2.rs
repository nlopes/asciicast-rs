// Test code legitimately uses unwrap/indexing and exact-value assertions.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::unreadable_literal,
    clippy::float_cmp
)]

use asciicast_rs::common::{Resize, Rgb};
use asciicast_rs::{Asciicast, V2};

const V2_CAST: &str = include_str!("fixtures/v2.cast");

#[test]
fn parses_v2_header_and_events() {
    let cast = Asciicast::<V2>::from_slice(V2_CAST.as_bytes()).unwrap();

    // Header
    assert_eq!(cast.header.version, 2);
    assert_eq!(cast.header.width, 80);
    assert_eq!(cast.header.height, 24);
    assert_eq!(cast.header.timestamp, Some(1504467315));
    assert_eq!(cast.header.title.as_deref(), Some("Demo"));
    let env = cast.header.env.as_ref().unwrap();
    assert_eq!(env.get("SHELL").map(String::as_str), Some("/bin/zsh"));
    assert_eq!(env.get("TERM").map(String::as_str), Some("xterm-256color"));

    // Theme: fg/bg and an 8-colour palette parsed into RGB.
    let theme = cast.header.theme.as_ref().unwrap();
    assert_eq!(theme.fg, Rgb::new(0xd0, 0xd0, 0xd0));
    assert_eq!(theme.bg, Rgb::new(0x21, 0x21, 0x21));
    assert_eq!(
        (theme.bg.r(), theme.bg.g(), theme.bg.b()),
        (0x21, 0x21, 0x21)
    );
    assert_eq!(theme.palette.len(), 8);
    assert_eq!(theme.palette[0], Rgb::new(0x15, 0x15, 0x15));
    assert_eq!(theme.palette[1], Rgb::new(0xac, 0x41, 0x42));
    assert_eq!(theme.palette[7], Rgb::new(0xd0, 0xd0, 0xd0));

    // Events: 2x output, resize, marker, input
    assert_eq!(cast.events.len(), 5);

    assert_eq!(cast.events[0].time, 0.248848);
    assert_eq!(cast.events[0].as_output(), Some("Hello World"));

    assert_eq!(cast.events[1].as_output(), Some("second line\n"));

    assert_eq!(
        cast.events[2].as_resize(),
        Some(Resize {
            cols: 100,
            rows: 40
        })
    );

    assert_eq!(cast.events[3].as_marker(), Some("checkpoint"));

    assert_eq!(cast.events[4].as_input(), Some("ls\r"));
}

#[test]
fn rejects_invalid_theme_colour() {
    let bad = r##"{"version": 2, "width": 80, "height": 24, "theme": {"fg": "#zzzzzz", "bg": "#000000", "palette": "#000000:#111111:#222222:#333333:#444444:#555555:#666666:#777777"}}"##;
    assert!(Asciicast::<V2>::from_slice(bad.as_bytes()).is_err());
}

#[test]
fn rejects_wrong_palette_length() {
    let bad = r##"{"version": 2, "width": 80, "height": 24, "theme": {"fg": "#d0d0d0", "bg": "#212121", "palette": "#000000:#111111:#222222"}}"##;
    assert!(Asciicast::<V2>::from_slice(bad.as_bytes()).is_err());
}

#[test]
fn from_path_matches_from_slice() {
    let from_path = Asciicast::<V2>::from_path("tests/fixtures/v2.cast").unwrap();
    let from_slice = Asciicast::<V2>::from_slice(V2_CAST.as_bytes()).unwrap();
    assert_eq!(from_path, from_slice);
}
