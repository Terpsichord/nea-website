use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use egui::{CollapsingHeader, Response, ScrollArea};
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

    fn new(path: PathBuf) -> Self {
        Self::new_recursive(path, Self::INITIAL_DEPTH)
    }

    fn new_recursive(path: PathBuf, max_depth: usize) -> Self {
        if path.is_file() {
            TreeNode::File { path }
        } else if max_depth == 0 {
            TreeNode::UnexploredDir { path }
        } else {
            let children = Self::read_children(&path, max_depth);
            TreeNode::ExploredDir { path, children }
        }
    }

    fn path(&self) -> &PathBuf {
        match self {
            TreeNode::UnexploredDir { path } => path,
            TreeNode::ExploredDir { path, .. } => path,
            TreeNode::File { path } => path,
        }
    }

    fn name_from_path(path: &Path) -> &str {
        // TODO: error handling
        path.file_name()
            .expect("failed to get file name")
            .to_str()
            .expect("failed to get file name")
    }

    fn name(&self) -> &str {
        Self::name_from_path(self.path())
    }

    fn read_children(path: &Path, max_depth: usize) -> Vec<TreeNode> {
        let mut children: Vec<_> = std::fs::read_dir(path)
            .expect("failed to read directory")
            .map(|entry| {
                Self::new_recursive(entry.expect("failed to get entry").path(), max_depth - 1)
            })
            .collect();
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
        children
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> ExplorerResponse {
        match self {
            TreeNode::UnexploredDir { .. } => {
                unreachable!(); // hopefully
            }
            TreeNode::ExploredDir { children, path } => {
                Self::directory_ui(ui, Self::name_from_path(path), children)
            }
            TreeNode::File { path } => Self::file_ui(ui, Self::name_from_path(path), path),
        }
    }

    fn explore(&mut self) {
        if let TreeNode::UnexploredDir { path } = self {
            // TODO: only read children when the header is clicked
            let children = TreeNode::read_children(path, 1);
            *self = TreeNode::ExploredDir {
                path: std::mem::take(path),
                children,
            };
        }
    }

    fn directory_ui(ui: &mut egui::Ui, name: &str, children: &mut [TreeNode]) -> ExplorerResponse {
        let mut child_open_file = None;
        let response = CollapsingHeader::new(name)
            .show(ui, |ui| {
                for child in children {
                    child.explore();
                    let response = child.ui(ui);

                    if let Some(open_file) = response.open_file {
                        child_open_file = Some(open_file);
                    }
                }
            })
            .header_response;

        ExplorerResponse {
            open_file: child_open_file,
            response,
        }
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
    pub fn new(path: PathBuf) -> Self {
        Self {
            root_node: TreeNode::new(path),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> ExplorerResponse {
        ScrollArea::vertical()
            .show(ui, |ui| {
                ui.style_mut().visuals.button_frame = false;
                self.root_node.ui(ui)
            })
            .inner
    }
}
