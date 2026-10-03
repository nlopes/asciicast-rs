use std::{collections::BTreeMap, path::Path};

use asciicast_rs::Error;
use plotters::prelude::*;
use serde::{Deserialize, Serialize};

mod support;

#[derive(Serialize, Deserialize)]
struct Measurement {
    input_bytes: usize,
    input_sha256: String,
    events: usize,
    allocations: u64,
    allocated_bytes: u64,
    peak_bytes: u64,
    retained_bytes: i64,
}

fn plot(
    path: &Path,
    title: &str,
    unit: &str,
    before: &BTreeMap<String, Measurement>,
    after: &BTreeMap<String, Measurement>,
    value: impl Fn(&Measurement) -> u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let names = [
        "v1-large/auto",
        "v2-large/eager",
        "v3-large/eager",
        "v3-large/stream",
        "v2-unicode/eager",
        "v3-unicode/eager",
        "v3-structured/eager",
        "v3-large-zstd/eager",
    ];
    let rows: Vec<_> = names
        .iter()
        .filter_map(|name| Some((*name, before.get(*name)?, after.get(*name)?)))
        .collect();
    let maximum = rows
        .iter()
        .map(|(_, old, new)| value(old).max(value(new)))
        .max()
        .unwrap_or(1)
        .max(1);
    let drawing = SVGBackend::new(path, (1100, 650)).into_drawing_area();
    drawing.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&drawing)
        .caption(title, ("sans-serif", 25))
        .margin(20)
        .x_label_area_size(45)
        .y_label_area_size(220)
        .build_cartesian_2d(
            0..maximum.saturating_add(maximum / 10 + 1),
            (0..rows.len() * 2)
                .with_key_points((0..rows.len()).map(|index| index * 2 + 1).collect()),
        )?;
    chart
        .configure_mesh()
        .disable_mesh()
        .x_desc(unit)
        .y_labels(rows.len())
        .y_label_formatter(&|position| {
            rows.get(*position / 2)
                .map_or(String::new(), |(name, _, _)| (*name).to_owned())
        })
        .draw()?;
    chart
        .draw_series(rows.iter().enumerate().map(|(index, (_, old, _))| {
            Rectangle::new(
                [(0, index * 2), (value(old), index * 2 + 1)],
                RGBColor(105, 120, 138).filled(),
            )
        }))?
        .label("Before")
        .legend(|(x, y)| {
            Rectangle::new(
                [(x, y - 5), (x + 15, y + 5)],
                RGBColor(105, 120, 138).filled(),
            )
        });
    chart
        .draw_series(rows.iter().enumerate().map(|(index, (_, _, new))| {
            Rectangle::new(
                [(0, index * 2 + 1), (value(new), index * 2 + 2)],
                RGBColor(32, 132, 112).filled(),
            )
        }))?
        .label("After")
        .legend(|(x, y)| {
            Rectangle::new(
                [(x, y - 5), (x + 15, y + 5)],
                RGBColor(32, 132, 112).filled(),
            )
        });
    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK)
        .position(SeriesLabelPosition::UpperRight)
        .draw()?;
    drawing.present()?;
    Ok(())
}

fn collect() -> Result<BTreeMap<String, Measurement>, Box<dyn std::error::Error>> {
    let mut measurements = BTreeMap::new();
    for case in support::recordings()? {
        let apis: &[&str] = if case.version == 1 {
            &["eager", "auto"]
        } else {
            &["eager", "auto", "stream"]
        };
        for api in apis {
            let mut result: Result<usize, Error> = Ok(0);
            let info = allocation_counter::measure(|| {
                result = support::parse(case.version, api, case.bytes.as_slice());
            });
            assert_eq!(result?, case.events);
            assert_eq!(
                info.bytes_current, 0,
                "parser must drop its result inside the measured operation"
            );
            measurements.insert(
                format!("{}/{api}", case.name),
                Measurement {
                    input_bytes: case.bytes.len(),
                    input_sha256: support::digest(&case.bytes),
                    events: case.events,
                    allocations: info.count_total,
                    allocated_bytes: info.bytes_total,
                    peak_bytes: info.bytes_max,
                    retained_bytes: info.bytes_current,
                },
            );
        }
    }
    let bytes = support::long_recording()?;
    let mut reader = None;
    let mut result: Result<(), Error> = Ok(());
    let retained = allocation_counter::measure(|| {
        result = (|| {
            let mut stream = asciicast_rs::v3::stream(bytes.as_slice())?;
            drop(stream.next().transpose()?);
            reader = Some(stream);
            Ok(())
        })();
    });
    result?;
    drop(reader);
    measurements.insert(
        "v3-long/retained-reader".to_owned(),
        Measurement {
            input_bytes: bytes.len(),
            input_sha256: support::digest(&bytes),
            events: 1,
            allocations: retained.count_total,
            allocated_bytes: retained.bytes_total,
            peak_bytes: retained.bytes_max,
            retained_bytes: retained.bytes_current,
        },
    );
    Ok(measurements)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut name = "current".to_owned();
    let mut baseline = None;
    let mut plot_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--save-baseline" => name = args.next().ok_or("expected baseline name")?,
            "--baseline" => baseline = Some(args.next().ok_or("expected baseline name")?),
            "--plot-only" => plot_only = true,
            "--bench" | "--test" => {}
            _ => return Err(format!("unknown allocation benchmark argument: {arg}").into()),
        }
    }
    if name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err("invalid baseline name".into());
    }
    let directory = std::env::var_os("ASCIICAST_MEMORY_OUTPUT")
        .map_or_else(|| "target/allocations".into(), std::path::PathBuf::from);
    std::fs::create_dir_all(&directory)?;
    let measurements = if plot_only {
        serde_json::from_slice(&std::fs::read(directory.join(format!("{name}.json")))?)?
    } else {
        let measurements = collect()?;
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&measurements)?,
        )?;
        measurements
    };
    if let Some(baseline) = baseline {
        if baseline.contains(['/', '\\']) || baseline == "." || baseline == ".." {
            return Err("invalid baseline name".into());
        }
        let before: BTreeMap<String, Measurement> =
            serde_json::from_slice(&std::fs::read(directory.join(format!("{baseline}.json")))?)?;
        assert_eq!(
            before.len(),
            measurements.len(),
            "memory benchmark cases differ"
        );
        for (name, old) in &before {
            let new = measurements
                .get(name)
                .ok_or("memory benchmark cases differ")?;
            assert_eq!((old.input_bytes, old.events), (new.input_bytes, new.events));
            assert_eq!(old.input_sha256, new.input_sha256);
        }
        plot(
            &directory.join("allocations.svg"),
            "Allocation calls",
            "allocation calls (including growth)",
            &before,
            &measurements,
            |value| value.allocations,
        )?;
        plot(
            &directory.join("peak-memory.svg"),
            "Peak requested heap",
            "bytes (input excluded)",
            &before,
            &measurements,
            |value| value.peak_bytes,
        )?;
    }
    println!(
        "Saved {} cases to {}",
        measurements.len(),
        directory.join(format!("{name}.json")).display()
    );
    Ok(())
}
