use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use egui::{CollapsingHeader, Response, ScrollArea};
use eyre::WrapErr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TreeNode {
    UnexploredDir {
        path: PathBuf,
    },
    ExploredDir {
        path: PathBuf,
        children: Vec<TreeNode>,
    },
    File {
        path: PathBuf,
    },
}

impl TreeNode {
    const INITIAL_DEPTH: usize = 2;

    fn new(path: PathBuf) -> eyre::Result<Self> {
        Self::new_recursive(path, Self::INITIAL_DEPTH)
    }

    // post-order recursive tree traversal algorithm, i think
    fn new_recursive(path: PathBuf, max_depth: usize) -> eyre::Result<Self> {
        Ok(if path.is_file() {
            TreeNode::File { path }
        } else if max_depth == 0 {
            TreeNode::UnexploredDir { path }
        } else {
            let children = Self::read_children(&path, max_depth)?;
            TreeNode::ExploredDir { path, children }
        })
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            TreeNode::UnexploredDir { path } => path,
            TreeNode::ExploredDir { path, .. } => path,
            TreeNode::File { path } => path,
        }
    }

    fn name_from_path(path: &Path) -> &str {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }

    fn name(&self) -> &str {
        Self::name_from_path(self.path())
    }

    fn read_children(path: &Path, max_depth: usize) -> eyre::Result<Vec<TreeNode>> {
        let error_msg = || format!("Failed to read directory: {}", path.to_string_lossy());

        let mut children = vec![];
        let dir_entries = std::fs::read_dir(path).wrap_err_with(error_msg)?;
        for entry in dir_entries {
            children.push(Self::new_recursive(
                entry.wrap_err_with(error_msg)?.path(),
                max_depth - 1,
            )?);
        }
        children.sort_by(|a, b| match (a, b) {
            (
                TreeNode::ExploredDir { .. } | TreeNode::UnexploredDir { .. },
                TreeNode::File { .. },
            ) => Ordering::Less,
            (
                TreeNode::File { .. },
                TreeNode::ExploredDir { .. } | TreeNode::UnexploredDir { .. },
            ) => Ordering::Greater,
            _ => a.name().cmp(b.name()),
        });
        Ok(children)
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> eyre::Result<ExplorerResponse> {
        Ok(match self {
            TreeNode::UnexploredDir { .. } => {
                unreachable!(); // hopefully
            }
            TreeNode::ExploredDir { children, path } => {
                Self::directory_ui(ui, Self::name_from_path(path), children)?
            }
            TreeNode::File { path } => Self::file_ui(ui, Self::name_from_path(path), path),
        })
    }

    fn explore(&mut self) -> eyre::Result<()> {
        if let TreeNode::UnexploredDir { path } = self {
            // TODO: only read children when the header is clicked
            let children = TreeNode::read_children(path, 1)?;
            *self = TreeNode::ExploredDir {
                path: std::mem::take(path),
                children,
            };
        }

        Ok(())
    }

    fn directory_ui(
        ui: &mut egui::Ui,
        name: &str,
        children: &mut [TreeNode],
    ) -> eyre::Result<ExplorerResponse> {
        let mut child_open_file = None;
        let response = CollapsingHeader::new(name).show(ui, |ui| {
            for child in children {
                child.explore()?;
                let response = child.ui(ui)?;

                if let Some(open_file) = response.open_file {
                    child_open_file = Some(open_file);
                }
            }
            Ok(())
        });

        if let Some(Err(err)) = response.body_returned {
            return Err(err);
        }

        Ok(ExplorerResponse {
            open_file: child_open_file,
            response: response.header_response,
        })
    }

    fn file_ui(ui: &mut egui::Ui, name: &str, path: &Path) -> ExplorerResponse {
        let button = ui.button(name);
        ExplorerResponse {
            open_file: button.double_clicked().then(|| path.to_owned()),
            response: button,
        }
    }
}

pub struct ExplorerResponse {
    pub response: Response,
    /// The file to be opened, if any
    pub open_file: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Explorer {
    pub root_node: TreeNode,
}

impl Explorer {
    pub fn new(path: PathBuf) -> eyre::Result<Self> {
        Ok(Self {
            root_node: TreeNode::new(path)?,
        })
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> eyre::Result<ExplorerResponse> {
        ScrollArea::vertical()
            .show(ui, |ui| {
                ui.style_mut().visuals.button_frame = false;
                self.root_node.ui(ui)
            })
            .inner
    }
}
