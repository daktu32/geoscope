// data/temporal_filter.rs — Time-domain FFT spectrum & lowpass filter
//
// Uses ispack-rs RealFftPlan (rustfft/realfft wrapper with ISPACK normalization).

use ispack_rs::transform::fft::RealFftPlan;

/// Power spectrum computed from a time series.
#[derive(Debug, Clone)]
pub struct TemporalSpectrumData {
    /// Power |c_k|² for k = 0..n_freq (inclusive).
    pub energy: Vec<f64>,
    /// Frequency values: k / (N * dt) for physical units, or k if dt unknown.
    pub frequencies: Vec<f64>,
    /// Number of positive frequency bins (= N/2).
    pub n_freq: usize,
    /// Sampling interval (time step). 0.0 if unknown.
    pub dt: f64,
}

/// Compute the temporal power spectrum of a time series.
///
/// `values`: time series values (length N).
/// `time_values`: time coordinate values (length N), used to infer dt.
///
/// Returns `None` if the input is too short (< 2 points).
pub fn compute_temporal_spectrum(
    values: &[f32],
    time_values: &[f64],
) -> Option<TemporalSpectrumData> {
    let n = values.len();
    if n < 2 || time_values.len() != n {
        return None;
    }

    // Infer sampling interval from time coordinate
    let dt = if time_values.len() >= 2 {
        let d = (time_values[1] - time_values[0]).abs();
        if d > 0.0 { d } else { 1.0 }
    } else {
        1.0
    };

    let mut plan = RealFftPlan::new(n);

    // f32 -> f64
    let input: Vec<f64> = values.iter().map(|&v| v as f64).collect();
    let coeffs = plan.forward(&input);

    // coeffs has N/2+1 elements (k = 0..N/2)
    let n_freq = n / 2;
    let mut energy = Vec::with_capacity(n_freq + 1);
    let mut frequencies = Vec::with_capacity(n_freq + 1);

    for (k, c) in coeffs.iter().enumerate() {
        // Power = |c_k|² (forward is 1/N normalized)
        energy.push(c.norm_sqr());
        frequencies.push(k as f64 / (n as f64 * dt));
    }

    Some(TemporalSpectrumData {
        energy,
        frequencies,
        n_freq,
        dt,
    })
}

/// Apply a lowpass filter: zero out frequency components at indices >= cutoff_idx.
///
/// `values`: time series (length N).
/// `cutoff_idx`: frequency index cutoff. Components with k >= cutoff_idx are zeroed.
///   - cutoff_idx = 0 → only DC remains (mean value)
///   - cutoff_idx >= N/2+1 → identity (no filtering)
///
/// Returns the filtered time series, or `None` if input is too short.
pub fn temporal_lowpass_filter(values: &[f32], cutoff_idx: usize) -> Option<Vec<f32>> {
    let n = values.len();
    if n < 2 {
        return None;
    }

    let mut plan = RealFftPlan::new(n);

    let input: Vec<f64> = values.iter().map(|&v| v as f64).collect();
    let mut coeffs = plan.forward(&input);

    // Zero out frequencies at and above cutoff
    for k in cutoff_idx..coeffs.len() {
        coeffs[k] = num_complex::Complex::new(0.0, 0.0);
    }

    let output = plan.backward(&coeffs);
    Some(output.iter().map(|&v| v as f32).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Helper: generate a sine wave at a given frequency bin.
    fn sine_wave(n: usize, freq_bin: usize) -> Vec<f32> {
        (0..n)
            .map(|j| (2.0 * PI * freq_bin as f64 * j as f64 / n as f64).sin() as f32)
            .collect()
    }

    #[test]
    fn test_sine_wave_peak() {
        // A pure sine wave at frequency bin k should produce a peak at index k.
        let n = 64;
        let k = 5;
        let values = sine_wave(n, k);
        let time_values: Vec<f64> = (0..n).map(|i| i as f64).collect();

        let spec = compute_temporal_spectrum(&values, &time_values).unwrap();

        // Find the index of maximum energy (excluding DC)
        let peak_idx = spec
            .energy
            .iter()
            .enumerate()
            .skip(1)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        assert_eq!(peak_idx, k, "Peak should be at frequency bin {k}");

        // The peak should dominate: at least 100x larger than any other bin (except DC)
        let peak_energy = spec.energy[k];
        for (i, &e) in spec.energy.iter().enumerate() {
            if i != k && i != 0 {
                assert!(
                    e < peak_energy * 0.01,
                    "Bin {i} energy {e} should be << peak energy {peak_energy}"
                );
            }
        }
    }

    #[test]
    fn test_constant_input() {
        // Constant input → only DC (k=0) has energy.
        let n = 32;
        let values = vec![3.0f32; n];
        let time_values: Vec<f64> = (0..n).map(|i| i as f64).collect();

        let spec = compute_temporal_spectrum(&values, &time_values).unwrap();

        // DC component: forward gives c_0 = mean = 3.0, so |c_0|² = 9.0
        assert!((spec.energy[0] - 9.0).abs() < 1e-10, "DC energy should be 9.0");

        // All other bins should be zero
        for (i, &e) in spec.energy.iter().enumerate().skip(1) {
            assert!(e < 1e-20, "Bin {i} should be zero, got {e}");
        }
    }

    #[test]
    fn test_parseval() {
        // Parseval's theorem: Σ|c_k|² (with proper weighting) ≈ (1/N)Σx_j²
        // With ISPACK normalization (forward = 1/N), the relation is:
        //   |c_0|² + 2*Σ_{k=1}^{N/2-1}|c_k|² + |c_{N/2}|² = (1/N)Σx_j²
        let n = 64;
        let values: Vec<f32> = (0..n)
            .map(|j| {
                let t = j as f64 / n as f64;
                (2.0 * PI * 3.0 * t).sin() as f32 + 0.5 * (2.0 * PI * 7.0 * t).cos() as f32
            })
            .collect();
        let time_values: Vec<f64> = (0..n).map(|i| i as f64).collect();

        let spec = compute_temporal_spectrum(&values, &time_values).unwrap();

        // Time-domain average of x²
        let mean_sq: f64 = values.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / n as f64;

        // Frequency-domain sum (Parseval)
        let mut freq_sum = spec.energy[0]; // DC
        for k in 1..spec.n_freq {
            freq_sum += 2.0 * spec.energy[k]; // positive frequencies (mirrored)
        }
        freq_sum += spec.energy[spec.n_freq]; // Nyquist

        assert!(
            (freq_sum - mean_sq).abs() < 1e-10,
            "Parseval mismatch: freq_sum={freq_sum}, mean_sq={mean_sq}"
        );
    }

    #[test]
    fn test_filter_identity() {
        // cutoff >= N/2+1 → no filtering (identity)
        let n = 32;
        let values = sine_wave(n, 3);
        let filtered = temporal_lowpass_filter(&values, n / 2 + 1).unwrap();

        for (i, (&orig, &filt)) in values.iter().zip(filtered.iter()).enumerate() {
            assert!(
                (orig - filt).abs() < 1e-5,
                "Identity filter mismatch at index {i}: orig={orig}, filt={filt}"
            );
        }
    }

    #[test]
    fn test_filter_mean_only() {
        // cutoff=1 → only DC remains → output is the mean value
        let n = 32;
        let values: Vec<f32> = (0..n).map(|j| 2.0 + sine_wave(n, 5)[j]).collect();
        let mean: f32 = values.iter().sum::<f32>() / n as f32;

        let filtered = temporal_lowpass_filter(&values, 1).unwrap();

        for (i, &v) in filtered.iter().enumerate() {
            assert!(
                (v - mean).abs() < 1e-4,
                "Mean-only filter at index {i}: got {v}, expected {mean}"
            );
        }
    }

    #[test]
    fn test_too_short_input() {
        assert!(compute_temporal_spectrum(&[1.0], &[0.0]).is_none());
        assert!(temporal_lowpass_filter(&[1.0], 1).is_none());
    }

    #[test]
    fn test_filter_removes_high_freq() {
        // Signal = low freq (k=2) + high freq (k=10)
        // Lowpass with cutoff=5 should remove k=10
        let n = 64;
        let low: Vec<f32> = sine_wave(n, 2);
        let high: Vec<f32> = sine_wave(n, 10);
        let combined: Vec<f32> = low.iter().zip(high.iter()).map(|(&l, &h)| l + h).collect();

        let filtered = temporal_lowpass_filter(&combined, 5).unwrap();

        // Filtered should be close to the low-frequency component
        for (i, (&filt, &lo)) in filtered.iter().zip(low.iter()).enumerate() {
            assert!(
                (filt - lo).abs() < 1e-4,
                "Lowpass at index {i}: got {filt}, expected ~{lo}"
            );
        }
    }
}
