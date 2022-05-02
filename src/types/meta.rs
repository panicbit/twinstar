use anyhow::*;
use crate::Mime;
use crate::util::Cowy;


#[derive(Debug,Clone,PartialEq,Eq,Default)]
pub struct Meta(String);

impl Meta {
    pub const MAX_LEN: usize = 1024;

    /// Creates a new "Meta" string.
    /// Fails if `meta` contains `\n`.
    pub fn new(meta: impl Cowy<str>) -> Result<Self> {
        ensure!(!meta.as_ref().contains('\n'), "Meta must not contain newlines");
        ensure!(meta.as_ref().len() <= Self::MAX_LEN, "Meta must not exceed {} bytes", Self::MAX_LEN);

        Ok(Self(meta.into()))
    }

    /// Creates a new "Meta" string.
    /// Truncates `meta` to before:
    /// - the first occurrence of `\n`
    /// - the character that makes `meta` exceed `Meta::MAX_LEN`
    pub fn new_lossy(meta: impl Cowy<str>) -> Self {
        let meta = meta.as_ref();
        let truncate_pos = meta.char_indices().position(|(i, ch)| {
            let is_newline = ch == '\n';
            let exceeds_limit = (i + ch.len_utf8()) > Self::MAX_LEN;

            is_newline || exceeds_limit
        });

        let meta: String = match truncate_pos {
            None => meta.into(),
            Some(truncate_pos) => meta.get(..truncate_pos).expect("twinstar BUG").into(),
        };

        Self(meta)
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_mime(&self) -> Result<Mime> {
        let mime = self.as_str().parse::<Mime>()
            .context("Meta is not a valid MIME")?;
        Ok(mime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::repeat;

    #[test]
    fn new_rejects_newlines() {
        let meta = "foo\nbar";
        let meta = Meta::new(meta);

        assert!(meta.is_err());
    }

    #[test]
    fn new_accepts_max_len() {
        let meta: String = repeat('x').take(Meta::MAX_LEN).collect();
        let meta = Meta::new(meta);

        assert!(meta.is_ok());
    }

    #[test]
    fn new_rejects_exceeding_max_len() {
        let meta: String = repeat('x').take(Meta::MAX_LEN + 1).collect();
        let meta = Meta::new(meta);

        assert!(meta.is_err());
    }

    #[test]
    fn new_lossy_truncates() {
        let meta = "foo\r\nbar\nquux";
        let meta = Meta::new_lossy(meta);

        assert_eq!(meta.as_str(), "foo\r");
    }

    #[test]
    fn new_lossy_no_truncate() {
        let meta = "foo bar\r";
        let meta = Meta::new_lossy(meta);

        assert_eq!(meta.as_str(), "foo bar\r");
    }

    #[test]
    fn new_lossy_empty() {
        let meta = "";
        let meta = Meta::new_lossy(meta);

        assert_eq!(meta.as_str(), "");
    }

    #[test]
    fn new_lossy_truncates_to_empty() {
        let meta = "\n\n\n";
        let meta = Meta::new_lossy(meta);

        assert_eq!(meta.as_str(), "");
    }

    #[test]
    fn new_lossy_truncates_to_max_len() {
        let meta: String = repeat('x').take(Meta::MAX_LEN + 1).collect();
        let meta = Meta::new_lossy(meta);

        assert_eq!(meta.as_str().len(), Meta::MAX_LEN);
    }

    #[test]
    fn new_lossy_truncates_multi_byte_sequences() {
        let mut meta: String = repeat('x').take(Meta::MAX_LEN - 1).collect();
        meta.push('🦀');

        assert_eq!(meta.len(), Meta::MAX_LEN + 3);

        let meta = Meta::new_lossy(meta);

        assert_eq!(meta.as_str().len(), Meta::MAX_LEN - 1);
    }
}
