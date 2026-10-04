use std::io::{self, BufRead, BufReader, Cursor, Read};

use asciicast_rs::{Error, Streamable, V2, V3, v3};

const HEADER: &[u8] = b"{\"version\":3,\"term\":{\"cols\":80,\"rows\":24}}\n";

#[test]
fn continues_after_invalid_json_and_utf8() -> Result<(), Error> {
    for bad in [b"[bad\n".as_slice(), b"\xff\n", b"# \xff\n"] {
        let input = [HEADER, bad, b"[0.1,\"o\",\"next\"]\n"].concat();
        for capacity in [1, 7, 64, 8192] {
            let mut reader = v3::stream(BufReader::with_capacity(capacity, input.as_slice()))?;
            assert!(matches!(reader.next(), Some(Err(_))));
            let next = reader.next().transpose()?;
            assert_eq!(
                next.as_ref().and_then(|event| event.as_output()),
                Some("next")
            );
            assert!(reader.next().is_none());
        }
    }
    Ok(())
}

#[test]
fn invalid_utf8_keeps_standard_io_error_details() -> Result<(), Error> {
    let invalid = b"\xff\n".as_slice();
    let mut expected_input = invalid;
    let expected = expected_input
        .read_line(&mut String::new())
        .err()
        .ok_or_else(|| io::Error::other("expected invalid UTF8"))?;
    let input = [HEADER, invalid].concat();
    let mut reader = v3::stream(input.as_slice())?;
    let Some(Err(Error::Io(actual))) = reader.next() else {
        return Err(io::Error::other("expected an I/O error").into());
    };
    assert_eq!(actual.kind(), expected.kind());
    assert_eq!(actual.to_string(), expected.to_string());
    assert_eq!(actual.raw_os_error(), expected.raw_os_error());
    assert_eq!(actual.get_ref().is_some(), expected.get_ref().is_some());
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    assert!(reader.next().is_none());
    Ok(())
}

#[test]
fn line_endings_do_not_change_json_error_positions() -> Result<(), Error> {
    for ending in [b"\n".as_slice(), b"\r\n"] {
        let input = [HEADER, b"[0.1,\"o\",", ending].concat();
        let mut reader = v3::stream(input.as_slice())?;
        assert!(matches!(reader.next(), Some(Err(Error::Json(error))) if error.line() == 1));
    }
    Ok(())
}

#[test]
fn preserves_a_final_event_without_a_newline() -> Result<(), Error> {
    let input = [HEADER, b"[0.1,\"o\",\"first\"]\r\n[0.2,\"o\",\"last\"]\r"].concat();
    let events = v3::stream(input.as_slice())?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(events.len(), 2);
    assert_eq!(
        events.last().and_then(|event| event.as_output()),
        Some("last")
    );
    Ok(())
}

#[test]
fn every_buffer_boundary_preserves_events_and_skipped_lines() -> Result<(), Error> {
    let lines = concat!(
        "\u{2003}\r\n\t# comment with 🚀\r\n",
        "[0.1,\"o\",\"café 🚀\\r\\n\"]\r\n",
        "[0.2,\"o\",\"\\u001b[32mgreen\\u001b[0m\"]\n",
        " \u{2003}\n[0.3,\"o\",\"last\"]"
    );
    let input = [HEADER, lines.as_bytes()].concat();
    let expected = v3::stream(input.as_slice())?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(expected.len(), 3);
    for capacity in 1..=input.len() {
        let reader = BufReader::with_capacity(capacity, input.as_slice());
        assert_eq!(
            v3::stream(reader)?.collect::<Result<Vec<_>, _>>()?,
            expected,
            "buffer capacity {capacity}"
        );
    }
    Ok(())
}

struct InterruptedOnce {
    interrupted: bool,
    remaining: Cursor<&'static [u8]>,
}

impl Read for InterruptedOnce {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.remaining.read(buffer)
    }
}

#[test]
fn interrupted_reads_preserve_a_partial_event() -> Result<(), Error> {
    let prefix = [HEADER, b"[0.1,\"o\",\"par"].concat();
    let tail = InterruptedOnce {
        interrupted: false,
        remaining: Cursor::new(b"tial\"]\n[0.2,\"o\",\"next\"]"),
    };
    let input = Cursor::new(prefix).chain(tail);
    let events = v3::stream(BufReader::with_capacity(8, input))?.collect::<Result<Vec<_>, _>>()?;
    let output: Vec<_> = events
        .iter()
        .filter_map(|event| event.as_output())
        .collect();
    assert_eq!(output, ["partial", "next"]);
    Ok(())
}

struct FailsOnce {
    state: u8,
    remaining: Cursor<&'static [u8]>,
}

impl Read for FailsOnce {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.state {
            0 => {
                self.state = 1;
                buffer
                    .get_mut(..4)
                    .ok_or_else(|| io::Error::other("test requires a four-byte buffer"))?
                    .copy_from_slice(b"[bad");
                Ok(4)
            }
            1 => {
                self.state = 2;
                Err(io::Error::other("one read error"))
            }
            _ => self.remaining.read(buffer),
        }
    }
}

#[test]
fn discards_partial_line_after_an_io_error() -> Result<(), Error> {
    let tail = FailsOnce {
        state: 0,
        remaining: Cursor::new(b"[0.1,\"o\",\"recovered\"]\n"),
    };
    let input = Cursor::new(HEADER).chain(tail);
    let mut reader = v3::stream(BufReader::with_capacity(8, input))?;
    assert!(
        matches!(reader.next(), Some(Err(Error::Io(error))) if error.kind() == io::ErrorKind::Other)
    );
    let next = reader.next().transpose()?;
    assert_eq!(
        next.as_ref().and_then(|event| event.as_output()),
        Some("recovered")
    );
    Ok(())
}

#[test]
fn escaped_codes_and_unknown_codes_remain_owned() -> Result<(), Error> {
    let v2 = V2::parse_event(r#"[0.1,"\u006f","output"]"#)?;
    assert_eq!(v2.as_output(), Some("output"));
    let v3 = V3::parse_event(r#"[0.1,"\u006f","output"]"#)?;
    assert_eq!(v3.as_output(), Some("output"));

    let v2 = V2::parse_event(&String::from(r#"[0.1,"fut\u0075re",{"text":"kept"}]"#))?;
    assert!(
        matches!(v2.payload, asciicast_rs::v2::EventPayload::Unknown { ref code, .. } if code == "future")
    );
    let v3 = V3::parse_event(&String::from(r#"[0.1,"fut\u0075re","kept"]"#))?;
    assert!(
        matches!(v3.payload, asciicast_rs::v3::EventPayload::Unknown { ref code, ref data } if code == "future" && data == "kept")
    );
    Ok(())
}

#[test]
fn structured_payloads_accept_escaped_text() -> Result<(), Error> {
    let resize = V3::parse_event(r#"[0.1,"r","\u0038\u0030x24"]"#)?;
    assert_eq!(
        resize.payload,
        asciicast_rs::v3::EventPayload::Resize(asciicast_rs::common::Resize { cols: 80, rows: 24 })
    );
    let exit = V3::parse_event(r#"[0.1,"x","\u0030"]"#)?;
    assert_eq!(exit.as_exit().map(|status| status.code()), Some(0));
    Ok(())
}
