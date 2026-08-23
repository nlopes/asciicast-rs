use asciicast_rs::common::{Resize, Rgb, Theme};
use asciicast_rs::v2::{Event, EventCode, EventPayload};
use asciicast_rs::{Asciicast, Error, V2};

const V2_CAST: &str = include_str!("fixtures/v2.cast");

#[test]
fn parses_v2_header_and_events() -> Result<(), Error> {
    let cast = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;

    // Header
    assert_eq!(cast.header.version, 2);
    assert_eq!(cast.header.width, 80);
    assert_eq!(cast.header.height, 24);
    assert_eq!(cast.header.timestamp, Some(1_504_467_315));
    assert_eq!(cast.header.title.as_deref(), Some("Demo"));
    assert_eq!(
        cast.header
            .env
            .as_ref()
            .and_then(|env| env.get("SHELL"))
            .map(String::as_str),
        Some("/bin/zsh")
    );

    // Theme: fg/bg and an 8-colour palette parsed into RGB.
    let palette = vec![
        Rgb::new(0x15, 0x15, 0x15),
        Rgb::new(0xac, 0x41, 0x42),
        Rgb::new(0x7e, 0x8e, 0x50),
        Rgb::new(0xe5, 0xb5, 0x67),
        Rgb::new(0x6c, 0x99, 0xbb),
        Rgb::new(0x9f, 0x4e, 0x85),
        Rgb::new(0x7d, 0xd6, 0xcf),
        Rgb::new(0xd0, 0xd0, 0xd0),
    ];
    assert_eq!(
        cast.header.theme,
        Some(Theme {
            fg: Rgb::new(0xd0, 0xd0, 0xd0),
            bg: Rgb::new(0x21, 0x21, 0x21),
            palette,
        })
    );

    // Events: 2x output, resize, marker, input.
    assert_eq!(
        cast.events,
        vec![
            Event {
                time: 0.248_848,
                payload: EventPayload::Output("Hello World".to_owned()),
            },
            Event {
                time: 1.001_376,
                payload: EventPayload::Output("second line\n".to_owned()),
            },
            Event {
                time: 2.143_733,
                payload: EventPayload::Resize(Resize {
                    cols: 100,
                    rows: 40,
                }),
            },
            Event {
                time: 3.5,
                payload: EventPayload::Marker("checkpoint".to_owned()),
            },
            Event {
                time: 4.0,
                payload: EventPayload::Input("ls\r".to_owned()),
            },
        ]
    );

    // Accessor coverage.
    assert_eq!(
        cast.events.first().and_then(Event::as_output),
        Some("Hello World")
    );
    assert_eq!(
        cast.events.get(2).and_then(Event::as_resize),
        Some(Resize {
            cols: 100,
            rows: 40
        })
    );
    assert_eq!(
        cast.events.get(3).and_then(Event::as_marker),
        Some("checkpoint")
    );
    assert_eq!(cast.events.get(4).and_then(Event::as_input), Some("ls\r"));
    Ok(())
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
fn null_environment_values_are_ignored() -> Result<(), Error> {
    let data = br#"{"version":2,"width":80,"height":24,"env":{"SHELL":"/bin/bash","TERM":null}}
"#;
    let cast = Asciicast::<V2>::from_slice(data)?;

    assert_eq!(
        cast.header
            .env
            .as_ref()
            .and_then(|env| env.get("SHELL"))
            .map(String::as_str),
        Some("/bin/bash")
    );
    assert!(
        !cast
            .header
            .env
            .as_ref()
            .is_some_and(|env| env.contains_key("TERM"))
    );
    Ok(())
}

#[test]
fn unknown_event_preserves_complete_code_and_json_data() -> Result<(), Error> {
    let data = br#"{"version":2,"width":80,"height":24}
[1.25,"overlay",{"text":"hello","position":2}]
"#;
    let cast = Asciicast::<V2>::from_slice(data)?;

    let event = cast.events.first();
    assert_eq!(event.map(Event::code), Some(EventCode::Unknown));
    assert_eq!(
        event.map(|event| &event.payload),
        Some(&EventPayload::Unknown {
            code: "overlay".to_owned(),
            data: serde_json::json!({"text": "hello", "position": 2}),
        })
    );
    Ok(())
}

#[test]
fn from_path_matches_from_slice() -> Result<(), Error> {
    let from_path = Asciicast::<V2>::from_path("tests/fixtures/v2.cast")?;
    let from_slice = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(from_path, from_slice);
    Ok(())
}
