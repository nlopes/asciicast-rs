use std::{
    hint::black_box,
    time::{Duration, Instant},
};

#[path = "../benches/support/mod.rs"]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected version (1, 2 or 3), or --generate")?;
    if command == "--generate" {
        let directory = std::path::PathBuf::from(args.next().ok_or("expected output directory")?);
        std::fs::create_dir(&directory)?;
        let mut manifest = Vec::new();
        for case in support::recordings()? {
            std::fs::write(directory.join(&case.name), &case.bytes)?;
            manifest.push(serde_json::json!({"name":case.name,"version":case.version,"events":case.events,"bytes":case.bytes.len(),"sha256":support::digest(&case.bytes)}));
        }
        std::fs::write(
            directory.join("inputs.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        return Ok(());
    }
    let version: u8 = command.parse()?;
    let api = args.next().ok_or("expected API (eager, auto, stream)")?;
    let bytes = std::fs::read(args.next().ok_or("expected recording path")?)?;
    let seconds: u64 = args
        .next()
        .ok_or("expected profile duration in seconds")?
        .parse()?;
    let start = Instant::now();
    let mut iterations = 0;
    let events = loop {
        let events = black_box(support::parse(version, &api, bytes.as_slice())?);
        iterations += 1;
        if seconds == 0 || start.elapsed() >= Duration::from_secs(seconds) {
            break events;
        }
    };
    println!(
        "{}",
        serde_json::json!({"iterations":iterations,"events_per_iteration":events,"elapsed_ns":start.elapsed().as_nanos()})
    );
    Ok(())
}
