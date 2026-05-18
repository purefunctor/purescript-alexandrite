#[cfg(unix)]
mod tests {
    use purescript_analyzer::lsp::formatting;

    const INPUT: &[u8] = b"module Main where\nfoo = bar\n";

    #[test]
    fn returns_full_document_text() {
        let formatted = formatting::run("tr a-z A-Z", INPUT).unwrap();

        assert_eq!(formatted, b"MODULE MAIN WHERE\nFOO = BAR\n");
    }

    #[test]
    fn returns_identical_text_for_noop_formatter() {
        let formatted = formatting::run("cat", INPUT).unwrap();

        assert_eq!(formatted, INPUT);
    }

    #[test]
    fn parses_quoted_args() {
        let formatted = formatting::run("tr 'a-z' 'A-Z'", INPUT).unwrap();

        assert_eq!(formatted, b"MODULE MAIN WHERE\nFOO = BAR\n");
    }

    #[test]
    fn tolerates_outer_quoted_command_string() {
        let formatted = formatting::run("\"tr a-z A-Z\"", INPUT).unwrap();

        assert_eq!(formatted, b"MODULE MAIN WHERE\nFOO = BAR\n");
    }
}
