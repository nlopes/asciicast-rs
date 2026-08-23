use asciicast_rs::{Asciicast, Error, Reader, V2, V3, v2, v3};
use asciicast_rs::{v2::EventPayload as V2EventPayload, v3::EventPayload as V3EventPayload};

const V2_CAST: &str = include_str!("fixtures/v2.cast");
const V3_CAST: &str = include_str!("fixtures/v3.cast");

#[test]
fn stream_into_recording_matches_eager_parse() -> Result<(), Error> {
    let streamed = v2::stream(V2_CAST.as_bytes())?.into_recording()?;
    let eager = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    assert_eq!(streamed, eager);
    Ok(())
}

#[test]
fn v3_stream_skips_comments() -> Result<(), Error> {
    let mut reader = v3::stream(V3_CAST.as_bytes())?;
    assert_eq!(reader.header().term.cols, 80);
    let streamed = (&mut reader).collect::<Result<Vec<_>, _>>()?;
    assert_eq!(streamed.len(), 6);
    Ok(())
}

#[test]
fn stream_propagates_event_errors() -> Result<(), Error> {
    let data: &[u8] = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.0,\"o\"]\n";
    let reader = v2::stream(data)?;
    let collected: Result<Vec<_>, _> = reader.collect();
    assert!(collected.is_err());
    Ok(())
}

#[test]
fn stream_rejects_version_mismatch() {
    assert!(v2::stream(V3_CAST.as_bytes()).is_err());
    // The generic `Reader::open` enforces the same check.
    assert!(Reader::<V3, _>::open(V2_CAST.as_bytes()).is_err());
}

#[test]
fn stream_is_lazy() -> Result<(), Error> {
    // The second event line is malformed; pulling only the first event must
    // succeed without ever touching the bad line.
    let data: &[u8] = b"{\"version\":2,\"width\":80,\"height\":24}\n[0.0,\"o\",\"ok\"]\n[bad\n";
    let mut reader = v2::stream(data)?;
    assert!(matches!(reader.next(), Some(Ok(_))));
    Ok(())
}

#[test]
fn streams_unknown_v2_and_v3_events() -> Result<(), Error> {
    let v2_data: &[u8] =
        b"{\"version\":2,\"width\":80,\"height\":24}\n[1.0,\"overlay\",{\"text\":\"hello\"}]\n";
    let v2_events = v2::stream(v2_data)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        v2_events.first().map(|event| &event.payload),
        Some(&V2EventPayload::Unknown {
            code: "overlay".to_owned(),
            data: serde_json::json!({"text": "hello"}),
        })
    );

    let v3_data: &[u8] =
        b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n[0.5,\"subtitle\",\"hello\"]\n";
    let v3_events = v3::stream(v3_data)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        v3_events.first().map(|event| &event.payload),
        Some(&V3EventPayload::Unknown {
            code: "subtitle".to_owned(),
            data: "hello".to_owned(),
        })
    );
    Ok(())
}
