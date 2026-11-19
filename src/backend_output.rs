//! Cancellation-safe bounded line capture. Oversized lines are drained, never allowed
//! to block the independent installation worker by abandoning its output pipe.
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use zeroize::{Zeroize, Zeroizing};
pub(crate) const LINE_LIMIT: usize = 16 * 1024;
const LOOKAHEAD: usize = 1024; // validated secrets are at most 1000 bytes
pub(crate) struct Line {
    pub(crate) text: String,
    pub(crate) passed: bool,
    pub(crate) cleanup_incomplete: bool,
    pub(crate) stage: Option<usize>,
}
pub(crate) struct Lines<R> {
    reader: BufReader<R>,
    prefix: Zeroizing<Vec<u8>>,
    truncated: bool,
}
impl<R: AsyncRead + Unpin> Lines<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            prefix: Zeroizing::new(Vec::new()),
            truncated: false,
        }
    }
    pub(crate) async fn next_line(&mut self, secrets: &[&str]) -> std::io::Result<Option<Line>> {
        loop {
            let bytes = self.reader.fill_buf().await?;
            if bytes.is_empty() {
                return Ok(
                    (!self.prefix.is_empty() || self.truncated).then(|| self.finish(secrets))
                );
            }
            let newline = bytes.iter().position(|b| *b == b'\n');
            let n = newline.map_or(bytes.len(), |n| n + 1);
            let keep = n.min((LINE_LIMIT + LOOKAHEAD).saturating_sub(self.prefix.len()));
            self.prefix.extend_from_slice(&bytes[..keep]);
            self.truncated |= keep < n;
            self.reader.consume(n);
            if newline.is_some() {
                return Ok(Some(self.finish(secrets)));
            }
        }
    }
    fn finish(&mut self, secrets: &[&str]) -> Line {
        let raw = Zeroizing::new(String::from_utf8_lossy(&self.prefix).into_owned());
        let passed = !self.truncated && raw.starts_with("ENZIMOS_INSTALL_PASS ");
        let cleanup_incomplete = raw
            .strip_prefix("ENZIMOS_INSTALL_CLEANUP_INCOMPLETE ")
            .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
            .is_some_and(|v| {
                v["schema"] == 1
                    && v["installation_completed"] == true
                    && v["status"] == "installed-cleanup-incomplete"
            });
        let stage = crate::system::stage_of(&raw);
        let line = redact(&raw, secrets);
        self.prefix.zeroize();
        let truncated = std::mem::take(&mut self.truncated);
        Line {
            text: bounded(line, truncated),
            passed,
            cleanup_incomplete,
            stage,
        }
    }
}
pub(crate) fn redact(value: &str, secrets: &[&str]) -> String {
    let mut clean = Zeroizing::new(value.to_owned());
    let mut secrets: Vec<_> = secrets.iter().copied().filter(|s| !s.is_empty()).collect();
    secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    for secret in secrets {
        clean = Zeroizing::new(clean.replace(secret, "[redacted]"));
    }
    std::mem::take(&mut *clean)
}
pub(crate) fn bounded(mut line: String, mut truncated: bool) -> String {
    if line.len() > LINE_LIMIT {
        let mut end = LINE_LIMIT;
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        line.truncate(end);
        truncated = true;
    }
    if truncated {
        line.push_str(" [line truncated]");
    }
    line.trim_end_matches(['\r', '\n']).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn oversized_output_stays_bounded_redacted_and_preserves_protocol() {
        let secret = "canary-secret";
        let mut bytes = vec![b'x'; LINE_LIMIT - 3];
        bytes.extend_from_slice(secret.as_bytes());
        bytes.extend(vec![b'y'; LINE_LIMIT * 4]);
        bytes.extend_from_slice(
            b"\nENZIMOS_INSTALL_PASS_FAILED\nENZIMOS_INSTALL_PASS disk=/dev/fixture\n",
        );
        let mut lines = Lines::new(bytes.as_slice());
        let line = lines.next_line(&[secret]).await.unwrap().unwrap();
        assert!(line.text.len() <= LINE_LIMIT + 32 && line.text.ends_with("[line truncated]"));
        assert!(!line.text.contains("canary"));
        assert!(!lines.next_line(&[]).await.unwrap().unwrap().passed);
        // Even a one-character password cannot corrupt the machine success marker.
        let line = lines.next_line(&["P"]).await.unwrap().unwrap();
        assert!(line.passed);
        assert!(lines.next_line(&[]).await.unwrap().is_none());
    }
}
