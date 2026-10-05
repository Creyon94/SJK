//! The system clipboard, through the desktop's own tools: text out for `viewpos` and `mark`,
//! text in for the console's Ctrl+V. winit has no clipboard and this is a console
//! convenience, not a frame path: a missing tool only means nothing is copied or pasted.
use std::io::Write;
use std::process::{Command, Stdio};

/// Commands that read text to copy from their standard input, tried in order.
///
/// Windows tools talk in the console's OEM code page unless told otherwise, so
/// `clip` and a plain `Get-Clipboard` turned `€` or `’` into `?` or bytes that are
/// not UTF-8. PowerShell is used both ways with UTF-8 stated explicitly: copy
/// reads its input as raw UTF-8 bytes, and paste writes UTF-8 without a BOM.
const COPY: [&[&str]; 4] = [
    &["wl-copy"],
    &["xclip", "-selection", "clipboard"],
    &["pbcopy"],
    &[
        "powershell",
        "-NoProfile",
        "-Command",
        "$m = New-Object IO.MemoryStream; [Console]::OpenStandardInput().CopyTo($m); \
         Set-Clipboard -Value ([Text.Encoding]::UTF8.GetString($m.ToArray()))",
    ],
];
/// Commands that print the clipboard's text.
const PASTE: [&[&str]; 4] = [
    &["wl-paste", "--no-newline"],
    &["xclip", "-selection", "clipboard", "-o"],
    &["pbpaste"],
    &[
        "powershell",
        "-NoProfile",
        "-Command",
        "[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false; Get-Clipboard",
    ],
];

/// Put `text` on the clipboard; `false` if no tool took it.
pub(crate) fn copy(text: &str) -> bool {
    COPY.iter().any(|tool| {
        let Ok(mut child) = Command::new(tool[0])
            .args(&tool[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            return false;
        };
        let written = child
            .stdin
            .take()
            .is_some_and(|mut input| input.write_all(text.as_bytes()).is_ok());
        // The tool keeps serving the selection after it has the text; it is not waited for.
        written
    })
}

/// The clipboard's text, if a tool gave any.
pub(crate) fn paste() -> Option<String> {
    PASTE.iter().find_map(|tool| {
        let output = Command::new(tool[0])
            .args(&tool[1..])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        output.status.success().then(|| pasted_text(&output.stdout))
    })
}

/// A paste tool's output as text, without a leading byte-order mark.
fn pasted_text(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned()
}

#[cfg(test)]
mod tests {
    use super::pasted_text;

    #[test]
    fn pasted_utf8_keeps_its_symbols() {
        let symbols = "name a×¥’¡²³‘€½¼©ñæ…";
        assert_eq!(pasted_text(symbols.as_bytes()), symbols);
        let with_mark = [b"\xef\xbb\xbf".as_slice(), symbols.as_bytes()].concat();
        assert_eq!(pasted_text(&with_mark), symbols);
    }
}
