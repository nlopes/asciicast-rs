use std::{
    cell::Cell,
    fs::File,
    hint::black_box,
    io::{BufRead, BufReader, Cursor},
    time::Duration,
};

use asciicast_rs::Error;
use criterion::{BenchmarkGroup, Criterion, Throughput, measurement::WallTime};

mod support;

fn bench(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    expected: usize,
    mut run: impl FnMut() -> Result<usize, Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(run()?, expected);
    let failed = Cell::new(false);
    group.bench_function(name, |b| {
        b.iter(|| {
            let result = run();
            if result.is_err() {
                failed.set(true);
            }
            black_box(result)
        });
    });
    if failed.get() {
        return Err(std::io::Error::other("benchmark parse failed").into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut criterion = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .sample_size(50)
        .nresamples(10_000);
    if let Some(path) = std::env::var_os("ASCIICAST_BENCH_OUTPUT") {
        criterion = criterion.output_directory(std::path::Path::new(&path));
    }
    criterion = criterion.configure_from_args();
    let files = tempfile::tempdir()?;
    for case in support::recordings()? {
        let mut group = criterion.benchmark_group(format!("parser/{}", case.name));
        group.throughput(Throughput::Elements(u64::try_from(case.events)?));
        let apis: &[&str] = if case.version == 1 {
            &["eager", "auto"]
        } else {
            &["eager", "auto", "stream"]
        };
        for api in apis {
            bench(&mut group, api, case.events, || {
                support::parse(case.version, api, black_box(case.bytes.as_slice()))
            })?;
        }
        if case.name.ends_with("large") || case.name.ends_with("large-zstd") {
            let path = files.path().join(&case.name);
            std::fs::write(&path, &case.bytes)?;
            for api in apis.iter().filter(|api| **api != "auto") {
                bench(&mut group, &format!("{api}-file"), case.events, || {
                    support::parse(case.version, api, BufReader::new(File::open(&path)?))
                })?;
            }
        }
        if case.version != 1 && case.name.ends_with("medium") {
            for capacity in [1, 7, 64, 8192] {
                bench(
                    &mut group,
                    &format!("stream-buffer-{capacity}"),
                    case.events,
                    || {
                        support::parse(
                            case.version,
                            "stream",
                            BufReader::with_capacity(capacity, Cursor::new(&case.bytes)),
                        )
                    },
                )?;
            }
            if case.version == 3 {
                bench(&mut group, "times", case.events, || {
                    support::parse(3, "times", case.bytes.as_slice())
                })?;
            }
        }
        group.finish();
        if case.version != 1 && (case.events == 1 || case.name.ends_with("large-zstd")) {
            let mut group = criterion.benchmark_group(format!("latency/{}", case.name));
            for (api, expected) in [("header", 0), ("first", 1)] {
                bench(&mut group, api, expected, || {
                    support::parse(case.version, api, case.bytes.as_slice())
                })?;
            }
            group.finish();
        }
    }
    let bytes = support::recording(3, 10_000, "unicode")?;
    criterion.bench_function("control/utf8", |b| {
        b.iter(|| black_box(std::str::from_utf8(black_box(&bytes))));
    });
    criterion.bench_function("control/scan", |b| {
        b.iter(|| {
            let mut reader = bytes.as_slice();
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|n| n != 0) {
                black_box(&line);
                line.clear();
            }
        });
    });
    criterion.final_summary();
    Ok(())
}
