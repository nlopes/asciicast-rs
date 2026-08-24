use asciicast_rs::common::{Resize, Rgb, Theme};
use asciicast_rs::v3::{Event, EventCode, EventPayload, Term};
use asciicast_rs::{Asciicast, Error, V2, V3};

const V3_CAST: &str = include_str!("fixtures/v3.cast");
const V2_CAST: &str = include_str!("fixtures/v2.cast");

#[test]
fn parses_v3_header_term_and_events() -> Result<(), Error> {
    let cast = Asciicast::<V3>::from_slice(V3_CAST.as_bytes())?;

    // Header + term (theme lives under term in v3).
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
        cast.header.term,
        Term {
            cols: 80,
            rows: 24,
            r#type: Some("xterm-256color".to_owned()),
            version: Some("xterm(389)".to_owned()),
            theme: Some(Theme {
                fg: Rgb::new(0xd0, 0xd0, 0xd0),
                bg: Rgb::new(0x21, 0x21, 0x21),
                palette,
            }),
        }
    );
    assert_eq!(cast.header.version, 3);
    assert_eq!(cast.header.timestamp, Some(1_700_000_000));
    assert_eq!(cast.header.title.as_deref(), Some("Demo v3"));
    assert_eq!(cast.header.command.as_deref(), Some("/usr/bin/htop"));
    assert_eq!(cast.header.idle_time_limit, Some(2.5));
    assert_eq!(
        cast.header
            .env
            .as_ref()
            .and_then(|env| env.get("SHELL"))
            .map(String::as_str),
        Some("/bin/zsh")
    );
    assert_eq!(
        cast.header.tags,
        Some(vec!["demo".to_owned(), "test".to_owned()])
    );

    // The comment line is ignored, leaving 6 events; v3 carries relative intervals.
    let expected_first_five = vec![
        Event {
            interval: 0.1,
            payload: EventPayload::Output("Hello v3".to_owned()),
        },
        Event {
            interval: 0.2,
            payload: EventPayload::Output("more\n".to_owned()),
        },
        Event {
            interval: 0.05,
            payload: EventPayload::Resize(Resize {
                cols: 120,
                rows: 30,
            }),
        },
        Event {
            interval: 0.0,
            payload: EventPayload::Marker("mark".to_owned()),
        },
        Event {
            interval: 0.3,
            payload: EventPayload::Input("q".to_owned()),
        },
    ];
    assert_eq!(cast.events.get(..5), Some(expected_first_five.as_slice()));
    assert_eq!(cast.events.len(), 6);

    // The exit status is opaque (no public constructor), so check it via accessors.
    let last = cast.events.last();
    assert_eq!(last.map(Event::code), Some(EventCode::Exit));
    assert_eq!(
        last.and_then(Event::as_exit).map(|status| status.code()),
        Some(0)
    );
    assert_eq!(
        last.map(|event| event.interval.to_bits()),
        Some(1.0_f64.to_bits())
    );
    Ok(())
}

#[test]
fn wrong_version_is_rejected() {
    // A v2 recording parsed as v3 must fail, and vice versa.
    assert!(Asciicast::<V3>::from_slice(V2_CAST.as_bytes()).is_err());
    assert!(Asciicast::<V2>::from_slice(V3_CAST.as_bytes()).is_err());
}

#[test]
fn negative_event_intervals_are_rejected() {
    for interval in ["-0.5", "-0.0"] {
        let data = format!(
            "{{\"version\":3,\"term\":{{\"cols\":80,\"rows\":24}}}}\n[{interval},\"o\",\"hello\"]\n"
        );
        assert!(Asciicast::<V3>::from_slice(data.as_bytes()).is_err());
    }
}

#[test]
fn unknown_event_preserves_complete_code_and_data() -> Result<(), Error> {
    let data = br#"{"version":3,"term":{"cols":80,"rows":24}}
[0.5,"subtitle","hello"]
"#;
    let cast = Asciicast::<V3>::from_slice(data)?;

    let event = cast.events.first();
    assert_eq!(event.map(Event::code), Some(EventCode::Unknown));
    assert_eq!(
        event.map(|event| &event.payload),
        Some(&EventPayload::Unknown {
            code: "subtitle".to_owned(),
            data: "hello".to_owned(),
        })
    );
    Ok(())
}
