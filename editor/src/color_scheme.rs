use std::{
    fs::File,
    path::{Path, PathBuf},
};

use egui::{
    Color32, Shadow, Stroke, Style, Visuals,
    style::{Selection, WidgetVisuals, Widgets},
};
use eyre::eyre;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AvailableColorSchemes {
    pub schemes: Vec<(String, PathBuf)>,
}

impl Default for AvailableColorSchemes {
    fn default() -> Self {
        let mut schemes = Vec::new();
        if let Ok(scheme_files) = std::fs::read_dir("color_schemes") {
            schemes = scheme_files
                .filter_map(|x| x.ok())
                .map(|entry| {
                    Ok::<_, eyre::Error>((
                        Base16Scheme::read_from_yaml(&entry.path())?.name,
                        entry.path(),
                    ))
                })
                .try_collect()
                .unwrap_or_default();
        }

        Self { schemes }
    }
}

pub struct Base16Scheme {
    // 16 24-bit colors
    name: String,
    bases: [Color32; 16],
}

impl Base16Scheme {
    pub fn read_from_yaml(path: &Path) -> eyre::Result<Self> {
        let file = File::open(path)?;
        let yaml: serde_yaml::Value = serde_yaml::from_reader(file)?;

        let name = yaml["scheme"]
            .as_str()
            .ok_or(eyre!("invalid color scheme: missing name"))?
            .to_string();

        let mut scheme = Self {
            name,
            bases: [Color32::BLACK; 16],
        };

        for i in 0..16 {
            let name = format!("base0{:X}", i);
            let hex = yaml[name]
                .as_str()
                .ok_or(eyre!("invalid color scheme yaml"))?;

            scheme.bases[i] = Color32::from_hex(&format!("#{hex}"))
                .map_err(|_| eyre!("invalid color in yaml"))?;
        }

        Ok(scheme)
    }

    const SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 96);

    pub fn to_style(&self) -> Style {
        let default = Visuals::default();
        Style {
            visuals: Visuals {
                widgets: Widgets {
                    noninteractive: WidgetVisuals {
                        bg_fill: self.bases[1],
                        weak_bg_fill: self.bases[1],
                        bg_stroke: Stroke {
                            color: self.bases[2],
                            ..default.widgets.noninteractive.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: self.bases[5],
                            ..default.widgets.noninteractive.fg_stroke
                        },
                        ..default.widgets.noninteractive
                    },
                    inactive: WidgetVisuals {
                        bg_fill: self.bases[2],
                        weak_bg_fill: self.bases[2],
                        bg_stroke: Stroke {
                            color: Color32::TRANSPARENT,
                            ..default.widgets.inactive.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: self.bases[5],
                            ..default.widgets.inactive.fg_stroke
                        },
                        ..default.widgets.inactive
                    },
                    hovered: WidgetVisuals {
                        bg_fill: self.bases[2],
                        weak_bg_fill: self.bases[2],
                        bg_stroke: Stroke {
                            color: self.bases[3],
                            ..default.widgets.hovered.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: self.bases[6],
                            ..default.widgets.hovered.fg_stroke
                        },
                        ..default.widgets.hovered
                    },
                    active: WidgetVisuals {
                        bg_fill: self.bases[10],
                        weak_bg_fill: self.bases[10],
                        bg_stroke: Stroke {
                            color: self.bases[7],
                            ..default.widgets.active.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: self.bases[7],
                            ..default.widgets.active.fg_stroke
                        },
                        ..default.widgets.active
                    },
                    open: WidgetVisuals {
                        bg_fill: self.bases[1],
                        weak_bg_fill: self.bases[1],
                        bg_stroke: Stroke {
                            color: self.bases[2],
                            ..default.widgets.open.bg_stroke
                        },
                        fg_stroke: Stroke {
                            color: self.bases[6],
                            ..default.widgets.open.fg_stroke
                        },
                        ..default.widgets.open
                    },
                },
                selection: Selection {
                    bg_fill: self.bases[8],
                    stroke: Stroke {
                        color: self.bases[4],
                        ..default.selection.stroke
                    },
                },
                hyperlink_color: self.bases[8],
                faint_bg_color: Color32::TRANSPARENT,
                extreme_bg_color: self.bases[0],
                code_bg_color: self.bases[2],
                warn_fg_color: self.bases[12],
                error_fg_color: self.bases[11],
                window_shadow: Shadow {
                    color: Self::SHADOW,
                    ..default.window_shadow
                },
                window_fill: self.bases[1],
                window_stroke: Stroke {
                    color: self.bases[2],
                    ..default.window_stroke
                },
                panel_fill: self.bases[1],
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
