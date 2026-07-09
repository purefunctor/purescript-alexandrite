use std::str::FromStr;

use anyhow::bail;

#[derive(Copy, Clone, Debug)]
pub enum TestCategory {
    Checking,
    Elaborating,
    Lowering,
    Resolving,
    Lsp,
    Docs,
}

impl TestCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestCategory::Checking => "checking",
            TestCategory::Elaborating => "elaborating",
            TestCategory::Lowering => "lowering",
            TestCategory::Resolving => "resolving",
            TestCategory::Lsp => "lsp",
            TestCategory::Docs => "docs",
        }
    }

    pub fn fixtures_subdir_fragment(&self) -> String {
        format!("tests-integration/fixtures/{}", self.as_str())
    }

    pub fn snapshot_path_fragments(&self) -> Vec<String> {
        vec![
            format!("tests-integration/fixtures/{}", self.as_str()),
            format!("tests-integration/tests/snapshots/{}__", self.as_str()),
        ]
    }
}

impl FromStr for TestCategory {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "checking" | "c" => Ok(TestCategory::Checking),
            "elaborating" | "e" => Ok(TestCategory::Elaborating),
            "lowering" | "l" => Ok(TestCategory::Lowering),
            "resolving" | "r" => Ok(TestCategory::Resolving),
            "lsp" => Ok(TestCategory::Lsp),
            "docs" => Ok(TestCategory::Docs),
            _ => bail!(
                "unknown test category '{}', expected: checking (c), elaborating (e), lowering (l), resolving (r), lsp, docs",
                s
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TestCategory;

    #[test]
    fn parses_elaborating_category_and_abbreviation() {
        for input in ["elaborating", "e", "ELABORATING"] {
            let category: TestCategory = input.parse().unwrap();
            assert_eq!(category.as_str(), "elaborating");
        }
    }
}
