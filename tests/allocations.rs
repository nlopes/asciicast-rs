use asciicast_rs::{Asciicast, Error, V3, v3};

#[path = "../benches/support/mod.rs"]
mod support;

#[test]
fn oversized_event_storage_is_released_while_reader_lives() -> Result<(), Box<dyn std::error::Error>>
{
    let bytes = support::long_recording()?;
    let mut reader = None;
    let mut result: Result<(), Error> = Ok(());
    let mut retained = allocation_counter::AllocationInfo::default();
    let total = allocation_counter::measure(|| {
        retained = allocation_counter::measure(|| {
            result = (|| {
                let mut stream = v3::stream(bytes.as_slice())?;
                let event = stream.next().transpose()?;
                assert_eq!(
                    event
                        .as_ref()
                        .and_then(|event| event.as_output())
                        .map(str::len),
                    Some(8 * 1024 * 1024)
                );
                drop(event);
                reader = Some(stream);
                Ok(())
            })();
        });
        drop(reader.take());
    });
    result?;
    assert!(
        (0..=128 * 1024).contains(&retained.bytes_current),
        "retained: {retained:?}"
    );
    assert_eq!(total.bytes_current, 0);
    Ok(())
}

#[test]
fn structured_events_do_not_allocate_per_event() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = support::recording(3, 10_000, "structured")?;
    let mut result: Result<usize, Error> = Ok(0);
    let info = allocation_counter::measure(|| {
        result =
            Asciicast::<V3>::from_slice(&bytes).map(|cast| std::hint::black_box(cast).events.len());
    });
    assert_eq!(result?, 10_000);
    assert!(info.count_total < 100, "allocations: {info:?}");
    assert_eq!(info.bytes_current, 0);
    Ok(())
}

#[test]
fn streaming_memory_stays_bounded_for_many_small_events() -> Result<(), Box<dyn std::error::Error>>
{
    let bytes = support::recording(3, 10_000, "mixed")?;
    let mut result: Result<usize, Error> = Ok(0);
    let info = allocation_counter::measure(|| {
        result = support::parse(3, "stream", bytes.as_slice());
    });
    assert_eq!(result?, 10_000);
    assert!(info.bytes_max < 128 * 1024, "streaming heap: {info:?}");
    assert_eq!(info.bytes_current, 0);
    Ok(())
}
