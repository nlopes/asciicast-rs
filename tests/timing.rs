use asciicast_rs::{Asciicast, Error, V1, V2, V3, v3};

const V1_CAST: &str = include_str!("fixtures/v1.cast");
const V2_CAST: &str = include_str!("fixtures/v2.cast");
const V3_CAST: &str = include_str!("fixtures/v3.cast");

fn assert_close(got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "length mismatch");
    for (g, w) in got.iter().zip(want) {
        assert!((*g - *w).abs() < 1e-9, "got {g}, want {w}");
    }
}

#[test]
fn streamed_absolute_times_accumulate() -> Result<(), Error> {
    let times = v3::stream(V3_CAST.as_bytes())?
        .absolute_times()
        .map(|event| event.map(|(at, _)| at))
        .collect::<Result<Vec<_>, _>>()?;
    assert_close(&times, &[0.1, 0.3, 0.35, 0.35, 0.65, 1.65]);
    Ok(())
}

#[test]
fn v1_absolute_times_accumulate_delays() -> Result<(), Error> {
    let cast = Asciicast::<V1>::from_slice(V1_CAST.as_bytes())?;
    let times: Vec<f64> = cast.absolute_times().map(|(t, _)| t).collect();
    assert_close(&times, &[1.0, 1.5]);
    Ok(())
}

#[test]
fn v2_absolute_times_are_the_event_times() -> Result<(), Error> {
    let cast = Asciicast::<V2>::from_slice(V2_CAST.as_bytes())?;
    let times: Vec<f64> = cast.absolute_times().map(|(t, _)| t).collect();
    assert_close(&times, &[0.248_848, 1.001_376, 2.143_733, 3.5, 4.0]);
    Ok(())
}

#[test]
fn v3_absolute_times_accumulate_intervals() -> Result<(), Error> {
    let cast = Asciicast::<V3>::from_slice(V3_CAST.as_bytes())?;
    let times: Vec<f64> = cast.absolute_times().map(|(t, _)| t).collect();
    assert_close(&times, &[0.1, 0.3, 0.35, 0.35, 0.65, 1.65]);
    Ok(())
}

#[test]
fn v3_unknown_events_contribute_to_absolute_time() -> Result<(), Error> {
    let data: &[u8] = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n[0.5,\"subtitle\",\"hello\"]\n[0.25,\"o\",\"world\"]\n";

    let cast = Asciicast::<V3>::from_slice(data)?;
    let eager_times: Vec<f64> = cast.absolute_times().map(|(time, _)| time).collect();
    assert_close(&eager_times, &[0.5, 0.75]);

    let streamed_times = v3::stream(data)?
        .absolute_times()
        .map(|event| event.map(|(time, _)| time))
        .collect::<Result<Vec<_>, _>>()?;
    assert_close(&streamed_times, &[0.5, 0.75]);
    Ok(())
}
