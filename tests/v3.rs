// Test code legitimately uses unwrap/indexing and exact-value assertions.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::unreadable_literal,
    clippy::float_cmp
)]

use asciicast_rs::common::Resize;
use asciicast_rs::{Asciicast, V2, V3};

const V3_CAST: &str = include_str!("fixtures/v3.cast");
const V2_CAST: &str = include_str!("fixtures/v2.cast");

#[test]
fn parses_v3_header_term_and_events() {
    let cast = Asciicast::<V3>::from_slice(V3_CAST.as_bytes()).unwrap();

    // Header + term
    assert_eq!(cast.header.version, 3);
    assert_eq!(cast.header.term.cols, 80);
    assert_eq!(cast.header.term.rows, 24);
    assert_eq!(cast.header.term.r#type.as_deref(), Some("xterm-256color"));
    assert_eq!(cast.header.timestamp, Some(1700000000));
    assert_eq!(cast.header.title.as_deref(), Some("Demo v3"));
    assert_eq!(
        cast.header.tags,
        Some(vec!["demo".to_owned(), "test".to_owned()])
    );

    // Theme lives under term in v3.
    let theme = cast.header.term.theme.as_ref().unwrap();
    assert_eq!(theme.palette.len(), 8);

    // The comment line is ignored, leaving 6 events.
    assert_eq!(cast.events.len(), 6);

    // v3 carries relative intervals.
    assert_eq!(cast.events[0].interval, 0.1);
    assert_eq!(cast.events[0].as_output(), Some("Hello v3"));
    assert_eq!(cast.events[1].as_output(), Some("more\n"));
    assert_eq!(
        cast.events[2].as_resize(),
        Some(Resize {
            cols: 120,
            rows: 30
        })
    );
    assert_eq!(cast.events[3].as_marker(), Some("mark"));
    assert_eq!(cast.events[4].as_input(), Some("q"));
    assert_eq!(cast.events[5].as_exit().map(|e| e.code()), Some(0));
}

#[test]
fn wrong_version_is_rejected() {
    // A v2 recording parsed as v3 must fail with a version mismatch.
    assert!(Asciicast::<V3>::from_slice(V2_CAST.as_bytes()).is_err());
    // And the reverse.
    assert!(Asciicast::<V2>::from_slice(V3_CAST.as_bytes()).is_err());
}
