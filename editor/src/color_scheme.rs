use std::{fs::File, path::PathBuf};
use itertools::Itertools;
use ws_messages::ColorScheme;

use egui::{
    Color32, Shadow, Stroke, Style, Visuals,
    style::{Selection, WidgetVisuals, Widgets},
};

pub struct AvailableColorSchemes {
    pub schemes: Vec<ColorScheme>,
}

impl AvailableColorSchemes {
    pub fn get_scheme(&self, name: &str) -> Option<&ColorScheme> {
        // linear search through available color schemes
       self.schemes.iter().find(|scheme| scheme.name() == name)
    }

    const SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 96);
    
    pub fn scheme_to_style(scheme: &ColorScheme) -> Style {
        let default = Visuals::default();
        Style {
            visuals: Visuals {
                widgets: Widgets {
                    noninteractive: WidgetVisuals {
                        bg_fill: scheme.bases[1],
                        weak_bg_fill: scheme.bases[1],
                        bg_stroke: Stroke {
                            color: scheme.bases[2],
                            ..default.widgets.noninteractive.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: scheme.bases[5],
                            ..default.widgets.noninteractive.fg_stroke
                        },
                        ..default.widgets.noninteractive
                    },
                    inactive: WidgetVisuals {
                        bg_fill: scheme.bases[2],
                        weak_bg_fill: scheme.bases[2],
                        bg_stroke: Stroke {
                            color: Color32::TRANSPARENT,
                            ..default.widgets.inactive.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: scheme.bases[5],
                            ..default.widgets.inactive.fg_stroke
                        },
                        ..default.widgets.inactive
                    },
                    hovered: WidgetVisuals {
                        bg_fill: scheme.bases[2],
                        weak_bg_fill: scheme.bases[2],
                        bg_stroke: Stroke {
                            color: scheme.bases[3],
                            ..default.widgets.hovered.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: scheme.bases[6],
                            ..default.widgets.hovered.fg_stroke
                        },
                        ..default.widgets.hovered
                    },
                    active: WidgetVisuals {
                        bg_fill: scheme.bases[10],
                        weak_bg_fill: scheme.bases[10],
                        bg_stroke: Stroke {
                            color: scheme.bases[7],
                            ..default.widgets.active.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: scheme.bases[7],
                            ..default.widgets.active.fg_stroke
                        },
                        ..default.widgets.active
                    },
                    open: WidgetVisuals {
                        bg_fill: scheme.bases[1],
                        weak_bg_fill: scheme.bases[1],
                        bg_stroke: Stroke {
                            color: scheme.bases[2],
                            ..default.widgets.open.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: scheme.bases[6],
                            ..default.widgets.open.fg_stroke
                        },
                        ..default.widgets.open
                    },
                },
                selection: Selection {
                    bg_fill: scheme.bases[8],
                    stroke: Stroke {
                        color: scheme.bases[4],
                        ..default.selection.stroke
                    },
                },
                hyperlink_color: scheme.bases[8],
                faint_bg_color: Color32::TRANSPARENT,
                extreme_bg_color: scheme.bases[0],
                code_bg_color: scheme.bases[2],
                warn_fg_color: scheme.bases[12],
                error_fg_color: scheme.bases[11],
                window_shadow: Shadow {
                    color: Self::SHADOW,
                    ..default.window_shadow
                },
                window_fill: scheme.bases[1],
                window_stroke: Stroke {
                    color: scheme.bases[2],
                    ..default.window_stroke
                },
                panel_fill: scheme.bases[1],
                popup_shadow: Shadow {
                    color: Self::SHADOW,
                    ..default.popup_shadow
                },
                ..default
            },
            ..Style::default()
        }
    }
}

impl Default for AvailableColorSchemes {
    #[cfg(not(target_arch = "wasm32"))]
    fn default() -> Self {
        let mut schemes = Vec::new();

        if let Ok(scheme_files) = std::fs::read_dir("color_schemes") {
            schemes = scheme_files
                .filter_map(|x| x.ok())
                .map(|entry| {
                    let file = File::open(entry.path())?;
                    ColorScheme::read_from_yaml(&file)
                })
                .try_collect()
                .unwrap_or_default();
        }

        Self { schemes }
    }

    #[cfg(target_arch = "wasm32")]
    fn default() -> Self {
        Self { schemes: vec![] }
    }
}
