//! Fast path for the usual string-valued events. Anything outside this subset
//! goes through the original serde parser, retaining its validation and errors.

use std::borrow::Cow;

pub(super) struct TextEvent<'a> {
    pub(super) time: f64,
    pub(super) code: &'a str,
    pub(super) data: Cow<'a, str>,
}

fn whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

pub(super) fn parse(line: &str) -> Option<TextEvent<'_>> {
    let line = line.trim_matches(whitespace).strip_prefix('[')?;
    let comma = memchr::memchr(b',', line.as_bytes())?;
    let (time, rest) = line.split_at(comma);
    let rest = rest.strip_prefix(',')?.trim_start_matches(whitespace);
    let rest = rest.strip_prefix('"')?;
    let code = rest.get(..1)?;
    if !matches!(code, "o" | "i" | "m" | "r" | "x") {
        return None;
    }
    let rest = rest.get(1..)?.strip_prefix('"')?;
    let rest = rest.trim_start_matches(whitespace).strip_prefix(',')?;
    let data = rest
        .trim_start_matches(whitespace)
        .strip_suffix(']')?
        .trim_end_matches(whitespace);

    // Keep serde's exact floating point semantics, including feature unification
    // with callers enabling serde_json/float_roundtrip. Negative zero is invalid.
    let time: f64 = serde_json::from_str(time).ok()?;
    if time.is_sign_negative() {
        return None;
    }
    Some(TextEvent {
        time,
        code,
        data: string(data)?,
    })
}

fn string(raw: &str) -> Option<Cow<'_, str>> {
    let raw = raw.strip_prefix('"')?.strip_suffix('"')?;
    // A non-short-circuiting reduction lets LLVM vectorize the control-byte
    // check. A scalar early-exit loop dominates parsing for long payloads.
    #[allow(clippy::needless_bitwise_bool)]
    let contains_control = raw
        .as_bytes()
        .iter()
        .fold(false, |found, &byte| found | (byte < 0x20));
    if contains_control {
        return None;
    }
    let Some(first) = memchr::memchr2(b'"', b'\\', raw.as_bytes()) else {
        return Some(Cow::Borrowed(raw));
    };
    // Allocate once at the encoded length, an upper bound for the decoded text.
    // Write escapes directly into the String returned to the caller.
    let mut decoded = String::with_capacity(raw.len());
    let (prefix, mut rest) = raw.split_at(first);
    decoded.push_str(prefix);
    loop {
        rest = rest.strip_prefix('\\')?;
        let (&escape, _) = rest.as_bytes().split_first()?;
        rest = rest.get(1..)?;
        decoded.push(match escape {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{0008}',
            b'f' => '\u{000c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                // Terminal control characters are commonly encoded as \u00XX.
                if let Some(digits) = rest.strip_prefix("00") {
                    let digits = digits.get(..2)?;
                    let character = u8::from_str_radix(digits, 16).ok()?;
                    // from_str_radix accepts a leading '+', which JSON does not.
                    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                        return None;
                    }
                    rest = rest.get(4..)?;
                    char::from(character)
                } else {
                    let (character, consumed) = unicode_escape(rest)?;
                    rest = rest.get(consumed..)?;
                    character
                }
            }
            _ => return None,
        });
        // Unicode-escaped text often has adjacent escapes with no literal
        // span to copy. Avoid a memchr call for each of those empty spans.
        if rest.starts_with('\\') {
            continue;
        }
        let Some(next) = memchr::memchr2(b'"', b'\\', rest.as_bytes()) else {
            decoded.push_str(rest);
            return Some(Cow::Owned(decoded));
        };
        let (text, tail) = rest.split_at(next);
        decoded.push_str(text);
        rest = tail;
    }
}

// The leading \u has already been consumed. Invalid escapes return None so
// the original event deserializer supplies the same error and source position.
// Keep full Unicode handling out of the common terminal-control decoding loop.
#[inline(never)]
fn unicode_escape(rest: &str) -> Option<(char, usize)> {
    let first = hex_quad(rest)?;
    let (scalar, consumed) = if (0xd800..=0xdbff).contains(&first) {
        let rest = rest.get(4..)?.strip_prefix("\\u")?;
        let second = hex_quad(rest)?;
        if !(0xdc00..=0xdfff).contains(&second) {
            return None;
        }
        (
            0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00),
            10,
        )
    } else {
        (u32::from(first), 4)
    };
    // Also rejects an unpaired low surrogate.
    Some((char::from_u32(scalar)?, consumed))
}

fn hex_quad(rest: &str) -> Option<u16> {
    let &[a, b, c, d, ..] = rest.as_bytes() else {
        return None;
    };
    let value = (hex_digit(a)? << 12) | (hex_digit(b)? << 8) | (hex_digit(c)? << 4) | hex_digit(d)?;
    Some(value)
}

fn hex_digit(byte: u8) -> Option<u16> {
    match byte {
        b'0'..=b'9' => Some(u16::from(byte - b'0')),
        b'a'..=b'f' => Some(u16::from(byte - b'a' + 10)),
        b'A'..=b'F' => Some(u16::from(byte - b'A' + 10)),
        _ => None,
    }
}
