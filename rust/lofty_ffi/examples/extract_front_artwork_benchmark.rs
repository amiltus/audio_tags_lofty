use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use lofty_ffi::extract_front_artwork;

fn percentile(sorted: &[Duration], percentile: f64) -> Duration {
    let index = ((sorted.len() as f64 * percentile).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[index]
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .map(PathBuf::from)
        .expect("usage: extract_front_artwork_benchmark <audio-path> [iterations]");
    let iterations = args
        .next()
        .map(|value| value.parse::<usize>().expect("iterations must be a number"))
        .unwrap_or(200);
    assert!(iterations > 0, "iterations must be positive");

    let input_limit = std::fs::metadata(&path)
        .expect("audio fixture must exist")
        .len()
        .checked_add(1)
        .expect("input size overflow");
    let artwork_limit = 32 * 1024 * 1024;

    extract_front_artwork(&path, input_limit, artwork_limit)
        .expect("warm-up extraction must succeed")
        .expect("benchmark input must contain artwork");

    let mut samples = Vec::with_capacity(iterations);
    let mut artwork_bytes = 0usize;
    for _ in 0..iterations {
        let started = Instant::now();
        let artwork = extract_front_artwork(&path, input_limit, artwork_limit)
            .expect("benchmark extraction must succeed")
            .expect("benchmark input must contain artwork");
        artwork_bytes = artwork.len();
        samples.push(started.elapsed());
    }

    samples.sort_unstable();
    let total: Duration = samples.iter().sum();
    let average = total / iterations as u32;
    println!("iterations: {iterations}");
    println!("artwork bytes: {artwork_bytes}");
    println!("average: {:.3} ms", average.as_secs_f64() * 1_000.0);
    println!(
        "p50: {:.3} ms",
        percentile(&samples, 0.50).as_secs_f64() * 1_000.0
    );
    println!(
        "p95: {:.3} ms",
        percentile(&samples, 0.95).as_secs_f64() * 1_000.0
    );
    println!(
        "max: {:.3} ms",
        samples.last().unwrap().as_secs_f64() * 1_000.0
    );
}
