//! The error of a failed connection, split for the tooltip of its profile
//! row.

/// The two lines of the failure tooltip, taken from the connection layer's
/// error: its first line as the summary, the remaining lines joined as the
/// detail line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConnectionFailure {
    pub summary: String,
    pub detail: Option<String>,
}

impl ConnectionFailure {
    pub(crate) fn from_error(error: &str) -> Self {
        let mut lines = error.lines().map(str::trim).filter(|line| !line.is_empty());

        let summary = lines.next().unwrap_or_default().to_string();
        let rest: Vec<&str> = lines.collect();
        let detail = (!rest.is_empty()).then(|| rest.join(" \u{b7} "));

        Self { summary, detail }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_splits_the_error_into_summary_and_detail() {
        let failure =
            ConnectionFailure::from_error("connection refused\n10.0.4.12:27017\n  3 tries\n");
        assert_eq!(failure.summary, "connection refused");
        assert_eq!(
            failure.detail.as_deref(),
            Some("10.0.4.12:27017 \u{b7} 3 tries")
        );

        let single = ConnectionFailure::from_error("timed out");
        assert_eq!(single.summary, "timed out");
        assert_eq!(single.detail, None);
    }
}
