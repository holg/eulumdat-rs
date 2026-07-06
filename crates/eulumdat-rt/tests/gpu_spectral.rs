//! GPU spectral parity: the GPU wavelength-sampling + weighted-channel path must
//! reproduce the CPU reference (`eulumdat-goniosim` WeightedChannels) and the
//! source spectrum's own direct colorimetry.
//!
//! Skips cleanly when no GPU adapter is available.

mod common;
use common::gpu_or_skip;
use eulumdat_rt::*;

#[test]
fn gpu_spectral_recovers_source_cct() {
    let Some(tracer) = gpu_or_skip(GpuTracer::new()) else {
        return;
    };

    for &cct in &[3000.0, 6500.0] {
        let spd = eulumdat_spectrum::synth::synthesize(cct);
        let direct = eulumdat_spectrum::analyze(&spd).cct_k;

        let result = pollster::block_on(tracer.trace_isotropic_spectral(2_000_000, 15.0, 5.0, &spd));
        let channels = result.channels().expect("spectral mode returns channels");
        let gpu_cct = channels.cct_k().expect("light collected");

        eprintln!("GPU spectral: target {cct:.0} K, direct {direct:.0} K, GPU {gpu_cct:.0} K");

        // The GPU uses a coarse 20 nm LUT vs the CPU's 5 nm tables, so allow a
        // wider tolerance than the CPU parity — but it must still track the CCT.
        assert!(
            (gpu_cct - direct).abs() < 200.0,
            "GPU CCT {gpu_cct:.0} K should track direct {direct:.0} K within 200 K"
        );
    }
}

#[test]
fn gpu_and_cpu_agree_on_sp_ratio_ordering() {
    let Some(tracer) = gpu_or_skip(GpuTracer::new()) else {
        return;
    };

    let warm = eulumdat_spectrum::synth::synthesize(2700.0);
    let cool = eulumdat_spectrum::synth::synthesize(6500.0);

    let r_warm = pollster::block_on(tracer.trace_isotropic_spectral(1_500_000, 15.0, 5.0, &warm));
    let r_cool = pollster::block_on(tracer.trace_isotropic_spectral(1_500_000, 15.0, 5.0, &cool));

    let sp_warm = r_warm.channels().unwrap().sp_ratio();
    let sp_cool = r_cool.channels().unwrap().sp_ratio();

    eprintln!("GPU S/P: warm {sp_warm:.2}, cool {sp_cool:.2}");
    assert!(
        sp_cool > sp_warm,
        "GPU must rank cool S/P above warm: {sp_cool:.2} vs {sp_warm:.2}"
    );
    // Same envelopes the CPU path enforces.
    assert!(sp_warm > 1.0 && sp_warm < 1.8, "warm S/P {sp_warm:.2}");
    assert!(sp_cool > 1.7 && sp_cool < 2.8, "cool S/P {sp_cool:.2}");
}
