use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};
const WINDOW: u64 = 4 * 1024 * 1024;
pub fn fingerprint(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let mut hash = Sha256::new();
    hash.update(len.to_le_bytes());
    let mut buf = vec![0; len.min(WINDOW) as usize];
    for offset in [0, len.saturating_sub(WINDOW)] {
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buf)?;
        hash.update(&buf);
    }
    Ok(hex::encode(&hash.finalize()[..16]))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn content_based_across_names() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::write(&a, b"sample").unwrap();
        fs::copy(&a, &b).unwrap();
        let expected = hex::encode(Sha256::digest(
            [6u64.to_le_bytes().as_slice(), b"sample", b"sample"].concat(),
        ));
        assert_eq!(fingerprint(&a).unwrap(), &expected[..32]);
        assert_eq!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
        fs::write(&b, b"Sample").unwrap();
        assert_ne!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
    }
}
