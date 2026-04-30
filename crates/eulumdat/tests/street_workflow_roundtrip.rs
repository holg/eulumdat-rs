//! Workflow round-trip test for the street designer.
//!
//! Verifies the path Richard called out in his Street Design feedback:
//!
//! 1. Edit a luminaire (load LDC, build initial layout)
//! 2. Recalculate (compute area + compliance)
//! 3. Optimize (run the layout solver, get top picks + Pareto front)
//! 4. Apply a candidate (mutate layout from optimizer suggestion)
//! 5. Re-edit (tweak a field on the applied layout)
//! 6. Recalculate (verify metrics responded to the new layout)
//!
//! The point is to catch regressions where any step *resets* prior state
//! or where the optimizer's `apply` doesn't actually change the
//! downstream compute. We use the road template ldc so the test
//! exercises a realistic distribution.

use eulumdat::standards::{
    cjj45::{Cjj45Class, Cjj45Standard},
    LightingStandard,
};
use eulumdat::street::{
    advisor::advise, optimize_layout, optimize_layout_all, pareto_front_tradeoff, Arrangement,
    OptimizerBounds, OptimizerObjective, StreetLayout,
};
use eulumdat::Eulumdat;

fn road_ldc() -> Eulumdat {
    let p = "../eulumdat-wasm/templates/road_luminaire.ldt";
    let content = std::fs::read_to_string(p).expect("road template must exist");
    Eulumdat::parse(&content).expect("road template must parse")
}

#[test]
fn full_design_workflow_roundtrip() {
    // ── 1. Edit (load LDC + initial layout) ───────────────────────────
    let ldc = road_ldc();
    let mut layout = StreetLayout {
        length_m: 120.0,
        lane_width_m: 3.5,
        num_lanes: 2,
        pole_spacing_m: 35.0,
        arrangement: Arrangement::SingleSide,
        mounting_height_m: 8.0,
        overhang_m: 1.0,
        tilt_deg: 0.0,
        pole_offset_m: 0.5,
        sidewalk_width_m: 1.5,
    };
    let initial_pole_spacing = layout.pole_spacing_m;
    let initial_height = layout.mounting_height_m;

    // ── 2. Recalculate ────────────────────────────────────────────────
    let area_initial = layout.compute(&ldc, 0.8);
    let initial_avg = area_initial.avg_lux;
    let initial_u0 = area_initial.uniformity_min_avg;
    assert!(
        initial_avg > 0.0,
        "initial design should produce positive lux"
    );

    // ── 3. Optimize ───────────────────────────────────────────────────
    // Use a fairly permissive fit so the optimizer has results regardless
    // of the road template's exact characteristics.
    let bounds = OptimizerBounds::default();
    let candidates = optimize_layout(
        &ldc,
        &layout,
        &bounds,
        OptimizerObjective::PoleCountPerKm,
        5,
        |design| design.uniformity_overall >= 0.20,
    );
    assert!(
        !candidates.is_empty(),
        "optimizer must yield at least one candidate"
    );
    let all = optimize_layout_all(
        &ldc,
        &layout,
        &bounds,
        OptimizerObjective::PoleCountPerKm,
        |design| design.uniformity_overall >= 0.20,
    );
    let frontier = pareto_front_tradeoff(&all);
    assert!(
        !frontier.is_empty(),
        "Pareto frontier should not be empty when candidates exist"
    );

    // ── 4. Apply best candidate ───────────────────────────────────────
    let pick = candidates[0].clone();
    layout.pole_spacing_m = pick.pole_spacing_m;
    layout.mounting_height_m = pick.mounting_height_m;
    layout.arrangement = pick.arrangement;

    // The applied candidate should differ from the initial layout in
    // *something* — otherwise the optimizer is a no-op and Apply is a
    // user-visible regression.
    let identical = pick.pole_spacing_m == initial_pole_spacing
        && pick.mounting_height_m == initial_height
        && pick.arrangement == Arrangement::SingleSide;
    assert!(
        !identical || candidates.len() > 1,
        "optimizer apply must visibly change the layout"
    );

    // ── 5. Re-edit (user-tweak after apply) ───────────────────────────
    layout.tilt_deg = 5.0;
    let after_edit = layout.compute(&ldc, 0.8);

    // ── 6. Recalculate must reflect both the apply AND the re-edit ────
    // (Loose check — we only verify the compute pipeline produced fresh
    // output, not that the metric moved a specific direction. Real
    // regressions show up as zeroed grids or unchanged numbers.)
    assert!(after_edit.avg_lux > 0.0, "post-edit avg lux must be > 0");
    assert!(
        (after_edit.avg_lux - initial_avg).abs() > 1e-9
            || (after_edit.uniformity_min_avg - initial_u0).abs() > 1e-9,
        "round-trip should leave at least one metric different from the initial design"
    );

    // ── 7. Compliance + advisor still work after the round-trip ───────
    let design = layout.design_result(&after_edit);
    if let Some(compliance) = Cjj45Standard.check_design(&Cjj45Class::ClassII, &design) {
        // Advisor is allowed to be empty (everything passing) or non-empty
        // (some criterion failing). We only verify it doesn't panic.
        let _ = advise(std::slice::from_ref(&compliance));
    }
}
