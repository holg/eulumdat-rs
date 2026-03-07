//! Diagram panel rendering

use eframe::egui::{self, Color32, Sense, TextureHandle, Ui};
use eulumdat::diagram::{
    ButterflyDiagram, CartesianDiagram, ConeDiagram, FloodlightCartesianDiagram, HeatmapDiagram,
    IsocandelaDiagram, IsoluxDiagram, IsoluxParams, PolarDiagram, SvgTheme, YScale,
};
use eulumdat::{BugDiagram, Eulumdat, PhotometricCalculations, PhotometricSummary};
use eulumdat_i18n::Locale;

use crate::diagram::Butterfly3DRenderer;
use crate::render::{render_svg_to_rgba, rgba_to_color_image};

/// Available diagram types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramType {
    Polar,
    Cartesian,
    Butterfly,
    Butterfly3D,
    Heatmap,
    Bug,
    Lcs,
    Cone,
    BeamAngle,
    Isolux,
    Isocandela,
    Floodlight,
    // ATLA-specific types (handled separately)
    Spectral,
    Greenhouse,
}

impl DiagramType {
    pub fn label(&self) -> &'static str {
        match self {
            DiagramType::Polar => "Polar",
            DiagramType::Cartesian => "Cartesian",
            DiagramType::Butterfly => "Butterfly",
            DiagramType::Butterfly3D => "3D",
            DiagramType::Heatmap => "Heatmap",
            DiagramType::Bug => "BUG",
            DiagramType::Lcs => "LCS",
            DiagramType::Cone => "Cone",
            DiagramType::BeamAngle => "Beam",
            DiagramType::Isolux => "Isolux",
            DiagramType::Isocandela => "Isocandela",
            DiagramType::Floodlight => "Floodlight",
            DiagramType::Spectral => "Spectral",
            DiagramType::Greenhouse => "Greenhouse",
        }
    }

    pub fn all() -> &'static [DiagramType] {
        &[
            DiagramType::Polar,
            DiagramType::Cartesian,
            DiagramType::Butterfly,
            DiagramType::Butterfly3D,
            DiagramType::Heatmap,
            DiagramType::Bug,
            DiagramType::Lcs,
            DiagramType::Cone,
            DiagramType::BeamAngle,
            DiagramType::Isolux,
            DiagramType::Isocandela,
            DiagramType::Floodlight,
        ]
    }
}

/// Diagram generation parameters
pub struct DiagramParams {
    pub mounting_height: f64,
    pub tilt_angle: f64,
    pub area_size: f64,
    pub log_scale: bool,
    pub c_plane: Option<f64>,
}

impl Default for DiagramParams {
    fn default() -> Self {
        Self {
            mounting_height: 3.0,
            tilt_angle: 0.0,
            area_size: 20.0,
            log_scale: false,
            c_plane: None,
        }
    }
}

/// Generate SVG for a diagram type
pub fn generate_svg(
    ldt: &Eulumdat,
    diagram_type: DiagramType,
    width: f64,
    height: f64,
    dark_theme: bool,
    locale: &Locale,
) -> Option<String> {
    generate_svg_with_height(
        ldt,
        diagram_type,
        width,
        height,
        dark_theme,
        3.0,
        locale,
        &DiagramParams::default(),
    )
}

/// Generate SVG for a diagram type with configurable mounting height
pub fn generate_svg_with_height(
    ldt: &Eulumdat,
    diagram_type: DiagramType,
    width: f64,
    height: f64,
    dark_theme: bool,
    mounting_height: f64,
    locale: &Locale,
    params: &DiagramParams,
) -> Option<String> {
    let theme = if dark_theme {
        SvgTheme::dark_with_locale(locale)
    } else {
        SvgTheme::light_with_locale(locale)
    };

    let summary = PhotometricSummary::from_eulumdat(ldt);

    Some(match diagram_type {
        DiagramType::Polar => {
            if let Some(cp) = params.c_plane {
                let polar = PolarDiagram::from_eulumdat_for_plane(ldt, cp);
                polar.to_svg_with_summary(width, height, &theme, &summary)
            } else {
                let polar = PolarDiagram::from_eulumdat(ldt);
                polar.to_svg_with_summary(width, height, &theme, &summary)
            }
        }
        DiagramType::Cartesian => {
            if let Some(cp) = params.c_plane {
                let cartesian =
                    CartesianDiagram::from_eulumdat_for_plane(ldt, cp, width, height * 0.75);
                cartesian.to_svg_with_summary(width, height * 0.75, &theme, &summary)
            } else {
                let cartesian = CartesianDiagram::from_eulumdat(ldt, width, height * 0.75, 8);
                cartesian.to_svg_with_summary(width, height * 0.75, &theme, &summary)
            }
        }
        DiagramType::Butterfly | DiagramType::Butterfly3D => {
            let butterfly = ButterflyDiagram::from_eulumdat(ldt, width, height * 0.8, 60.0);
            butterfly.to_svg(width, height * 0.8, &theme)
        }
        DiagramType::Heatmap => {
            let heatmap = HeatmapDiagram::from_eulumdat(ldt, width, height * 0.7);
            heatmap.to_svg_with_summary(width, height * 0.7, &theme, &summary)
        }
        DiagramType::Bug => {
            let bug = BugDiagram::from_eulumdat(ldt);
            bug.to_svg(width, height * 0.85, &theme)
        }
        DiagramType::Lcs => {
            let bug = BugDiagram::from_eulumdat(ldt);
            bug.to_lcs_svg(width, height * 0.75, &theme)
        }
        DiagramType::Cone => {
            if let Some(cp) = params.c_plane {
                let cone = ConeDiagram::from_eulumdat_for_plane(ldt, mounting_height, cp);
                cone.to_svg(width, height * 0.85, &theme)
            } else {
                let cone = ConeDiagram::from_eulumdat(ldt, mounting_height);
                cone.to_svg(width, height * 0.85, &theme)
            }
        }
        DiagramType::BeamAngle => {
            let polar = PolarDiagram::from_eulumdat(ldt);
            let analysis = PhotometricCalculations::beam_field_analysis(ldt);
            let show_both = analysis.is_batwing;
            polar.to_svg_with_beam_field_angles(width, height, &theme, &analysis, show_both)
        }
        DiagramType::Isolux => {
            let isolux_params = IsoluxParams {
                mounting_height: params.mounting_height,
                tilt_angle: params.tilt_angle,
                area_half_width: params.area_size,
                area_half_depth: params.area_size,
                grid_resolution: 60,
            };
            let diagram = IsoluxDiagram::from_eulumdat(ldt, width, height, isolux_params);
            diagram.to_svg(width, height, &theme)
        }
        DiagramType::Isocandela => {
            let diagram = IsocandelaDiagram::from_eulumdat(ldt, width, height * 0.85);
            diagram.to_svg(width, height * 0.85, &theme)
        }
        DiagramType::Floodlight => {
            let y_scale = if params.log_scale {
                YScale::Logarithmic
            } else {
                YScale::Linear
            };
            let diagram =
                FloodlightCartesianDiagram::from_eulumdat(ldt, width, height * 0.75, y_scale);
            diagram.to_svg(width, height * 0.75, &theme)
        }
        // ATLA-specific types are handled separately via generate_current_svg
        DiagramType::Spectral | DiagramType::Greenhouse => return None,
    })
}

/// Render the diagram type selector
pub fn render_diagram_selector(ui: &mut Ui, diagram_type: &mut DiagramType) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for dt in DiagramType::all() {
            if ui
                .selectable_label(*diagram_type == *dt, dt.label())
                .clicked()
            {
                *diagram_type = *dt;
                changed = true;
            }
        }
    });
    changed
}

/// Render the diagram panel
pub fn render_diagram_panel(
    ui: &mut Ui,
    ldt: &Eulumdat,
    diagram_type: DiagramType,
    dark_theme: bool,
    texture: &mut Option<TextureHandle>,
    texture_dirty: &mut bool,
    butterfly_3d: &mut Butterfly3DRenderer,
    locale: &Locale,
) {
    let available_size = ui.available_size();
    let size = available_size.min_elem() * 0.95;

    // Handle 3D diagram separately
    if diagram_type == DiagramType::Butterfly3D {
        render_3d_diagram(ui, size, dark_theme, butterfly_3d);
        return;
    }

    if *texture_dirty || texture.is_none() {
        if let Some(svg) = generate_svg(
            ldt,
            diagram_type,
            size as f64,
            size as f64,
            dark_theme,
            locale,
        ) {
            match render_svg_to_rgba(&svg, size as u32, size as u32) {
                Ok((pixels, w, h)) => {
                    let image = rgba_to_color_image(pixels, w, h);
                    *texture = Some(ui.ctx().load_texture(
                        "diagram",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                    *texture_dirty = false;
                }
                Err(e) => {
                    ui.colored_label(Color32::RED, format!("Render error: {}", e));
                }
            }
        }
    }

    if let Some(tex) = texture {
        let texture_size = tex.size_vec2();
        let scale = (available_size.x / texture_size.x).min(available_size.y / texture_size.y);
        let display_size = texture_size * scale;

        // Center the image but only occupy the diagram's actual size (not the whole panel)
        ui.allocate_new_ui(
            egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(
                ui.available_rect_before_wrap().center(),
                display_size,
            )),
            |ui| {
                ui.image((tex.id(), display_size));
            },
        );
    }
}

/// Render 3D diagram with rotation controls
fn render_3d_diagram(
    ui: &mut Ui,
    size: f32,
    dark_theme: bool,
    butterfly_3d: &mut Butterfly3DRenderer,
) {
    let bg_color = if dark_theme {
        Color32::from_rgb(26, 26, 46)
    } else {
        Color32::WHITE
    };

    // Create a frame for the 3D view
    egui::Frame::none()
        .fill(bg_color)
        .rounding(8.0)
        .show(ui, |ui| {
            let (response, painter) =
                ui.allocate_painter(egui::vec2(size, size * 0.8), Sense::drag());

            let rect = response.rect;

            // Handle drag for rotation
            if response.dragged() {
                let delta = response.drag_delta();
                butterfly_3d.rotation_y += delta.x as f64 * 0.01;
                butterfly_3d.rotation_x += delta.y as f64 * 0.01;
                butterfly_3d.rotation_x = butterfly_3d.rotation_x.clamp(-1.5, 1.5);
            }

            // Auto-rotate
            if butterfly_3d.auto_rotate && !response.dragged() {
                butterfly_3d.rotation_y += 0.005;
                ui.ctx().request_repaint();
            }

            // Render the 3D view
            butterfly_3d.render(&painter, rect, dark_theme);
        });

    // Controls
    ui.horizontal(|ui| {
        if ui
            .button(if butterfly_3d.auto_rotate {
                "Pause"
            } else {
                "Auto"
            })
            .clicked()
        {
            butterfly_3d.auto_rotate = !butterfly_3d.auto_rotate;
        }
        if ui.button("Reset").clicked() {
            butterfly_3d.reset_view();
        }
        ui.label("Drag to rotate");
    });
}
