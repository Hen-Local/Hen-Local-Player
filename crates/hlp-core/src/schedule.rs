use crate::types::Ms;
pub fn priority(utt_start: Ms, playhead: Ms, process_whole: bool) -> Option<u64> {
    if playhead.saturating_sub(utt_start) <= 2000 {
        Some(utt_start.saturating_sub(playhead))
    } else {
        process_whole.then(|| 1_000_000_000u64.saturating_add(playhead - utt_start))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn priority_near_playhead_then_history() {
        assert_eq!(priority(3000, 5000, false), Some(0));
        assert_eq!(priority(2999, 5000, false), None);
        assert_eq!(priority(2999, 5000, true), Some(1_000_002_001));
        assert_eq!(priority(7000, 5000, false), Some(2000));
        assert!(priority(8000, 5000, true) < priority(1000, 5000, true));
        assert_eq!(priority(0, u64::MAX, true), Some(u64::MAX));
    }
}
