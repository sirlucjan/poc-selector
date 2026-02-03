use crate::bench;
use crate::stats::StatResult;
use crate::system::BenchParams;

const PROBE_MIN_SECS: f64 = 1.0;
const PROBE_START_N: usize = 50;
const MIN_N: usize = 500;
const MAX_N: usize = 500_000;
const TARGET_PHASE_SECS: f64 = 5.0;
const WARMUP_RATIO: f64 = 0.2; // 1/5 of main phase

pub struct CalibrationResult {
    pub iterations: usize,
    pub warmup: usize,
    pub probe_mean_us: f64,
    pub probe_stddev_us: f64,
}

pub fn calibrate(params: &BenchParams) -> CalibrationResult {
    // Exponentially scale up until a single probe takes >= 1 second.
    // This avoids hard-coded iteration counts that may overshoot on slow systems.
    let mut probe_n = PROBE_START_N;
    let mut elapsed_s;
    let mut samples;

    loop {
        let warmup = (probe_n / 5).max(10);
        let t0 = std::time::Instant::now();
        samples = bench::bench_burst_sync(params, probe_n, warmup);
        elapsed_s = t0.elapsed().as_secs_f64();

        if elapsed_s >= PROBE_MIN_SECS || probe_n >= MAX_N {
            break;
        }
        // Scale up: estimate needed N, with 1.5x margin
        let factor = (PROBE_MIN_SECS / elapsed_s * 1.5).max(2.0);
        probe_n = (probe_n as f64 * factor) as usize;
    }

    let sr = StatResult::compute(&mut samples);
    let mean = sr.trimmed_mean;
    let stddev = sr.stddev;

    // Wall-clock throughput from the final probe (includes all overhead)
    let per_iter_s = elapsed_s / (probe_n + (probe_n / 5).max(10)) as f64;

    // N so that (warmup + N) = TARGET_PHASE_SECS
    // warmup = N * WARMUP_RATIO  =>  total = N * (1 + WARMUP_RATIO)
    let mut n = if per_iter_s > 0.0 {
        (TARGET_PHASE_SECS / ((1.0 + WARMUP_RATIO) * per_iter_s)) as usize
    } else {
        MIN_N
    };

    n = n.clamp(MIN_N, MAX_N);
    n = ((n + 50) / 100) * 100;

    let warmup = ((n as f64 * WARMUP_RATIO) as usize).max(100);

    CalibrationResult {
        iterations: n,
        warmup,
        probe_mean_us: mean / 1000.0,
        probe_stddev_us: stddev / 1000.0,
    }
}
