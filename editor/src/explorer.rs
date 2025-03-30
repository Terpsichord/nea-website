use std::path::{Path, PathBuf};

use egui::{CollapsingHeader, Response};

const MAX_DEPTH: usize = 2;

pub enum TreeNode {
    Directory {
        path: PathBuf,
        children: Vec<TreeNode>,
    },
    File {
        path: PathBuf,
    },
}

impl TreeNode {
    fn new(path: PathBuf) -> Self {
        Self::new_recursive(path, 0, MAX_DEPTH)
    }

    fn new_recursive(path: PathBuf, depth: usize, max_depth: usize) -> Self {
        if path.is_file() {
            TreeNode::File { path }
        } else {
            let children = if depth == max_depth {
                vec![]
            } else {
                std::fs::read_dir(&path)
                    .unwrap()
                    .map(|entry| Self::new_recursive(entry.unwrap().path(), depth + 1, max_depth))
                    .collect()
            };

            TreeNode::Directory { path, children }
        }
    }

    fn path(&self) -> &PathBuf {
        match self {
            TreeNode::Directory { path, .. } => path,
            TreeNode::File { path } => path,
        }
    }
}

impl TreeNode {
    fn ui(self, ui: &mut egui::Ui) -> ExplorerResponse {
        self.ui_recursive(ui, 0, MAX_DEPTH)
    }

    fn ui_recursive(self, ui: &mut egui::Ui, depth: usize, max_depth: usize) -> ExplorerResponse {
        let name = self
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        match self {
            TreeNode::Directory { children, .. } => Self::show_directory(ui, &name, children, depth, max_depth),
            TreeNode::File { .. } => Self::show_file(ui, &name, self.path()),
        }
    }

    fn show_directory(
        ui: &mut egui::Ui,
        name: &str,
        children: Vec<TreeNode>,
        depth: usize,
        max_depth: usize,
    ) -> ExplorerResponse {
        let mut outer_open_file = None;
        let response = CollapsingHeader::new(name)
            .show(ui, |ui| {
                if depth != max_depth {
                    for child in children {
                        let response = child.ui_recursive(ui, depth + 1, max_depth);
                        if let Some(open_file) = response.open_file {
                            outer_open_file = Some(open_file);
                        }
                    }
                }
            })
            .header_response;

        ExplorerResponse {
            open_file: outer_open_file,
            response,
        }
    }

    fn show_file(ui: &mut egui::Ui, name: &str, path: &Path) -> ExplorerResponse {
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

pub struct Explorer {
    pub root_node: TreeNode,
}

impl Explorer {
    pub fn new(path: PathBuf) -> Self {
        Self {
            root_node: TreeNode::new(path),
        }
    }

    pub fn ui(self, ui: &mut egui::Ui) -> ExplorerResponse {
        let response = self.root_node.ui(ui);
        ui.separator();

        response
    }
}
