use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};
/// Read a half-open sample interval, clamped to the available signed 16-bit little-endian file.
pub fn read_pcm(path: &Path, start_sample: u64, end_sample: u64) -> io::Result<Vec<f32>> {
    let mut file = File::open(path)?;
    let total = file.metadata()?.len() / 2;
    let start = start_sample.min(total);
    let end = end_sample.min(total).max(start);
    let bytes =
        usize::try_from((end - start) * 2).map_err(|_| io::Error::other("PCM range too large"))?;
    file.seek(SeekFrom::Start(start * 2))?;
    let mut buf = vec![0; bytes];
    file.read_exact(&mut buf)?;
    Ok(buf
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]) as f32 / 32768.)
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn exact_clamped_ranges() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for n in [10i16, -32768, 16384] {
            f.write_all(&n.to_le_bytes()).unwrap()
        }
        assert_eq!(read_pcm(f.path(), 1, 9).unwrap(), [-1., 0.5]);
        assert!(read_pcm(f.path(), 5, 1).unwrap().is_empty());
    }
}
