use std::{collections::BTreeMap, io::Write};

struct Frame {
    symbol: String,
    samples: u64,
    children: u64,
}

fn finish(
    stack: &mut Vec<Frame>,
    folded: &mut BTreeMap<String, u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let frame = stack.pop().ok_or("empty sample stack")?;
    let own = frame
        .samples
        .checked_sub(frame.children)
        .ok_or("child samples exceed parent samples")?;
    if own != 0 {
        let symbols: Vec<_> = stack
            .iter()
            .map(|frame| frame.symbol.as_str())
            .chain(std::iter::once(frame.symbol.as_str()))
            .collect();
        *folded.entry(symbols.join(";")).or_default() += own;
    }
    Ok(())
}

fn collapse(input: &str) -> Result<BTreeMap<String, u64>, Box<dyn std::error::Error>> {
    let body = input
        .split_once("Call graph:\n")
        .ok_or("sample has no call graph")?
        .1
        .split_once("Total number in stack")
        .ok_or("sample has no call graph end")?
        .0;
    let mut stack: Vec<Frame> = Vec::new();
    let mut folded = BTreeMap::new();
    let mut total = 0;
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let indent = line
            .find(|character: char| character.is_ascii_digit())
            .ok_or("sample line has no count")?;
        if indent < 4 || (indent - 4) % 2 != 0 {
            return Err("invalid sample indentation".into());
        }
        let depth = (indent - 4) / 2;
        while stack.len() > depth {
            finish(&mut stack, &mut folded)?;
        }
        if stack.len() != depth {
            return Err("sample skips a stack level".into());
        }
        let (samples, symbol) = line
            .get(indent..)
            .ok_or("invalid sample line")?
            .split_once(' ')
            .ok_or("sample line has no symbol")?;
        let samples: u64 = samples.parse()?;
        if let Some(parent) = stack.last_mut() {
            parent.children += samples;
        } else {
            total += samples;
        }
        let symbol = symbol.split("(in ").next().ok_or("missing symbol")?.trim();
        let symbol = if depth == 0 {
            "thread".to_owned()
        } else {
            format!("{:#}", rustc_demangle::demangle(symbol)).replace(';', ":")
        };
        stack.push(Frame {
            symbol,
            samples,
            children: 0,
        });
    }
    while !stack.is_empty() {
        finish(&mut stack, &mut folded)?;
    }
    if total == 0 || folded.values().sum::<u64>() != total {
        return Err("folded sample count differs from call graph roots".into());
    }
    eprintln!("Preserved {total} samples");
    Ok(folded)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("expected macOS sample path")?;
    let folded = collapse(&std::fs::read_to_string(path)?)?;
    let mut output = std::io::stdout().lock();
    for (stack, samples) in folded {
        writeln!(output, "{stack} {samples}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::collapse;

    #[test]
    fn preserves_parent_samples_and_duplicate_paths() -> Result<(), Box<dyn std::error::Error>> {
        let input =
            "Call graph:\n    10 root\n      3 child\n      4 child\nTotal number in stack\n";
        let folded = collapse(input)?;
        assert_eq!(folded.get("thread"), Some(&3));
        assert_eq!(folded.get("thread;child"), Some(&7));
        Ok(())
    }

    #[test]
    fn rejects_inconsistent_counts() {
        let input = "Call graph:\n    2 root\n      3 child\nTotal number in stack\n";
        assert!(collapse(input).is_err());
    }
}
