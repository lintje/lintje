use crate::formatter::formatted_context as formatted_context_real;
use crate::issue::Issue;
use termcolor::{BufferWriter, ColorChoice};

pub fn formatted_context(issue: &Issue) -> String {
    let bufwtr = BufferWriter::stdout(ColorChoice::Never);
    let mut out = bufwtr.buffer();
    match formatted_context_real(&mut out, issue) {
        Ok(()) => {
            // Strip off the two leading spaces per line if any
            // The indenting is tested somewhere else
            String::from_utf8_lossy(out.as_slice())
                .to_string()
                .lines()
                .into_iter()
                .map(|v| v.strip_prefix("  ").unwrap_or(v))
                .collect::<Vec<&str>>()
                .join("\n")
        }
        Err(e) => panic!("Unable to format context issue: {:?}", e),
    }
}
