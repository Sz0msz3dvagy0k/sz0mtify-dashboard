pub fn normalize_name(input: &str) -> String {
    let lower = input.to_lowercase();
    let cleaned = [
        "(remastered)",
        "deluxe edition",
        "explicit",
        "mono",
        "stereo",
        "live",
    ]
    .iter()
    .fold(lower, |acc, s| acc.replace(s, ""));
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn playlist_score(
    meta: f64,
    play_count: f64,
    recent_7d: f64,
    days_since: f64,
    repetition_14d: f64,
    artist_count: f64,
    album_count: f64,
    seed: u64,
) -> f64 {
    let rediscovery = 1.0 - (-days_since / 30.0).exp();
    let repetition_penalty = (repetition_14d.powf(1.15) / 20.0).clamp(0.0, 1.0);
    let jitter = ((seed % 100) as f64) / 10000.0;
    let base = 0.32 * meta
        + 0.22 * ((play_count + 1.0).ln() / 10.0)
        + 0.12 * ((recent_7d + 1.0).ln() / 5.0)
        + 0.18 * rediscovery
        - 0.14 * repetition_penalty
        + jitter;
    base - 0.15 * artist_count - 0.10 * album_count
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize() {
        assert_eq!(normalize_name("Song (Remastered) Live"), "song");
    }
}
