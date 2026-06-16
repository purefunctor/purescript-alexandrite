pub use console;

pub mod test_runner;

pub mod snapshots {
    use console::style;
    use similar::{ChangeTag, TextDiff};

    /// Strip insta frontmatter (YAML between --- markers) from snapshot content
    pub fn strip_frontmatter(content: &str) -> &str {
        let lines: Vec<&str> = content.lines().collect();
        if lines.first() != Some(&"---") {
            return content;
        }
        if let Some(end_idx) = lines.iter().skip(1).position(|&l| l == "---") {
            let start_byte: usize = lines[..end_idx + 2].iter().map(|l| l.len() + 1).sum();
            if start_byte <= content.len() {
                return &content[start_byte..];
            }
        }
        content
    }

    /// Print a colored diff between two strings with 2 lines of context
    pub fn print_diff(old: &str, new: &str) {
        let diff = TextDiff::from_lines(old, new);
        let groups = diff.grouped_ops(2);

        for (group_idx, group) in groups.iter().enumerate() {
            if group_idx > 0 {
                println!("{}", style("  ···").dim());
            }

            for op in group {
                for change in diff.iter_changes(op) {
                    let line_no = match change.tag() {
                        ChangeTag::Delete => change.old_index().map(|i| i + 1),
                        ChangeTag::Insert => change.new_index().map(|i| i + 1),
                        ChangeTag::Equal => change.new_index().map(|i| i + 1),
                    };
                    let line_no_str =
                        line_no.map(|n| format!("{:3}", n)).unwrap_or_else(|| "   ".into());

                    match change.tag() {
                        ChangeTag::Delete => {
                            print!("{}", style(format!("{} -{}", line_no_str, change)).red())
                        }
                        ChangeTag::Insert => {
                            print!("{}", style(format!("{} +{}", line_no_str, change)).green())
                        }
                        ChangeTag::Equal => {
                            print!("{}", style(format!("{}  {}", line_no_str, change)).dim())
                        }
                    }
                }
            }
        }
    }
}
