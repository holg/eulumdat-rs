//! Cone Diagram component - shows beam/field angle spread visualization
//! Uses eulumdat-core diagram module for SVG generation

use super::app::use_unit_system;
use crate::i18n::use_locale;
use eulumdat::{
    diagram::{ConeDiagram, ConeDiagramLabels, ConeIlluminanceTable, SvgTheme},
    Eulumdat, UnitSystem,
};
use leptos::prelude::*;

/// Cone diagram view showing beam and field angle spread
#[component]
pub fn ConeDiagramView(
    ldt: ReadSignal<Eulumdat>,
    /// Mounting height in meters
    mounting_height: ReadSignal<f64>,
    /// Selected C-plane (None = overall)
    c_plane: ReadSignal<Option<f64>>,
) -> impl IntoView {
    let locale = use_locale();
    let unit_system = use_unit_system();

    view! {
        <div class="cone-diagram" inner_html=move || {
            let ldt = ldt.get();
            let height = mounting_height.get();
            let cp = c_plane.get();

            let cone = match cp {
                Some(c) => ConeDiagram::from_eulumdat_for_plane(&ldt, height, c),
                None => ConeDiagram::from_eulumdat(&ldt, height),
            };

            let theme = SvgTheme::css_variables_with_locale(&locale.get());

            // Create localized labels from the locale
            let loc = locale.get();
            let labels = ConeDiagramLabels {
                beam_angle: loc.diagram.cone.beam_angle.clone(),
                field_angle: loc.diagram.cone.field_angle.clone(),
                mounting_height: loc.diagram.cone.mounting_height.clone(),
                beam_diameter: loc.diagram.cone.beam_diameter.clone(),
                field_diameter: loc.diagram.cone.field_diameter.clone(),
                intensity_50: loc.diagram.cone.intensity_50.clone(),
                intensity_10: loc.diagram.cone.intensity_10.clone(),
                floor: loc.diagram.cone.floor.clone(),
                meter: loc.diagram.cone.meter.clone(),
                c_plane_label: loc.diagram.cone.c_plane.clone(),
            };

            cone.to_svg_with_units(500.0, 400.0, &theme, &labels, unit_system.get())
        } />
    }
}

/// Illuminance table showing beam/field diameters and illuminance at multiple heights
#[component]
pub fn ConeIlluminanceTableView(
    ldt: ReadSignal<Eulumdat>,
    mounting_height: ReadSignal<f64>,
    c_plane: ReadSignal<Option<f64>>,
) -> impl IntoView {
    let locale = use_locale();
    let unit_system = use_unit_system();

    view! {
        <div class="illuminance-table-wrapper">
            {move || {
                let ldt_val = ldt.get();
                let _h = mounting_height.get();
                let cp = c_plane.get();
                let loc = locale.get();
                let units = unit_system.get();

                // Auto step based on max height
                let max_h = mounting_height.get();
                let step = if max_h <= 3.0 { 0.5 } else if max_h <= 8.0 { 1.0 } else { 2.0 };

                let table = match cp {
                    Some(c) => ConeIlluminanceTable::from_eulumdat_for_plane(&ldt_val, step, max_h, c),
                    None => ConeIlluminanceTable::from_eulumdat(&ldt_val, step, max_h),
                };

                if table.total_flux <= 0.0 {
                    return view! {
                        <div class="illuminance-no-data">
                            {loc.diagram.cone.illuminance_table.no_flux.clone()}
                        </div>
                    }.into_any();
                }

                let dist_label = units.distance_label();
                let illu_label = units.illuminance_label();

                view! {
                    <div class="illuminance-data-table-wrapper">
                        <h4 class="illuminance-table-title">{loc.diagram.cone.illuminance_table.title.clone()}</h4>
                        <table class="illuminance-data-table">
                            <thead>
                                <tr>
                                    <th>{loc.diagram.cone.illuminance_table.height.clone()} " (" {dist_label} ")"</th>
                                    <th>{loc.diagram.cone.illuminance_table.beam_field_diameter.clone()} " (" {dist_label} ")"</th>
                                    <th>{loc.diagram.cone.illuminance_table.e_nadir.clone()} " (" {illu_label} ")"</th>
                                    <th>{loc.diagram.cone.illuminance_table.e_c0.clone()} " (" {illu_label} ")"</th>
                                    <th>{loc.diagram.cone.illuminance_table.e_c90.clone()} " (" {illu_label} ")"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {table.rows.into_iter().map(|row| {
                                    let h = format_distance(row.height, &units);
                                    let bd = format_distance(row.beam_diameter, &units);
                                    let fd = format_distance(row.field_diameter, &units);
                                    let en = format_illuminance(row.e_nadir, &units);
                                    let ec0 = format_illuminance(row.e_beam_c0, &units);
                                    let ec90 = format_illuminance(row.e_beam_c90, &units);
                                    view! {
                                        <tr>
                                            <td>{h}</td>
                                            <td>{bd} " / " {fd}</td>
                                            <td>{en}</td>
                                            <td>{ec0}</td>
                                            <td>{ec90}</td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

fn format_distance(meters: f64, units: &UnitSystem) -> String {
    let v = units.convert_meters(meters);
    format!("{v:.2}")
}

fn format_illuminance(lux: f64, units: &UnitSystem) -> String {
    let v = units.convert_lux(lux);
    if v >= 100.0 {
        format!("{v:.0}")
    } else if v >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}
