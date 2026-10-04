# Local terminal recordings

These fixtures were recorded locally with [record-corpus.py](../../scripts/record-corpus.py).
The script runs original demonstration workloads through a Unix pseudoterminal
at 100 columns by 30 rows and captures output chunks and monotonic timestamps.
It uses Python's standard library; neither asciinema nor network access is needed.

| Session | Content |
| --- | --- |
| `typing` | Short writes, simulated command editing, quotes, backslashes and Unicode |
| `logs` | Bursts of coloured build-style messages and carriage-return progress updates |
| `dashboard` | Full-screen redraws with cursor movement, colours and Unicode |

These are scripted demonstrations, not recordings of production applications or
human typing. They complement the synthetic cases with actual PTY read boundaries
and timings. Only output events are captured; mixed input, marker, resize and exit
events are covered by the synthetic benchmarks.

Each capture is serialized twice: v2 uses absolute timestamps, and v3 uses
intervals derived from the same integer-microsecond clock values. Both variants
have identical output chunks. UTF-8 split across PTY reads is decoded incrementally.
The recorder uses a temporary working directory and a minimal environment; it
does not start a shell or capture user files, paths, environment variables or history.

All workload text, the recording script and the resulting fixtures are provided
under this repository's **MIT OR Apache-2.0** license. See [LICENSE-MIT](../../LICENSE-MIT)
and [LICENSE-APACHE](../../LICENSE-APACHE). No third-party recording content is used.
[corpus.json](corpus.json) records provenance, versions, sizes, event counts and
SHA-256 hashes, which the benchmark loader checks before measurement.

The checked-in recordings run by default, including compressed variants when
the `zstd` feature is enabled:

```sh
cargo bench --bench parser -- 'parser/local-'
cargo test --test benchmark_inputs
```

To capture replacements on Unix, run from the repository root:

```sh
python3 scripts/record-corpus.py target/recorded-corpus
```

The destination must not already exist. Review the recordings and manifest,
then copy the `.cast` files and `corpus.json` into this directory. Scheduling
affects event boundaries and timestamps, so recaptures will have different hashes.
Keep the checked-in bytes fixed when comparing parser revisions, and regenerate
both benchmark baselines when intentionally replacing these fixtures. Normal
tests and benchmarks read the checked-in corpus without invoking Python or a PTY.
