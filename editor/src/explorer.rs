use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

use egui::{CollapsingHeader, Popup, Response, ScrollArea};
use eyre::WrapErr;

use crate::platform::{FileSystem, FileSystemTrait as _};

// FIXME
// #[derive(Serialize, Deserialize, Debug)]
#[derive(Debug)]
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
    // NewFile {
    //     name: String,
    // },
}

impl TreeNode {
    const INITIAL_DEPTH: usize = 2;

    fn new(path: PathBuf, fs: &FileSystem) -> eyre::Result<Self> {
        Self::new_recursive(path, Self::INITIAL_DEPTH, fs)
    }

    // TODO: post-order recursive tree traversal algorithm, i think
    fn new_recursive(path: PathBuf, max_depth: usize, fs: &FileSystem) -> eyre::Result<Self> {
        Ok(if Self::path_is_file(&path) {
            TreeNode::File { path }
        } else if max_depth == 0 {
            TreeNode::UnexploredDir { path }
        } else {
            let children = Self::read_children(&path, max_depth, fs)?;
            TreeNode::ExploredDir { path, children }
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn path_is_file(path: &Path) -> bool {
        path.is_file()
    }

    #[cfg(target_arch = "wasm32")]
    fn path_is_file(path: &Path) -> bool {
        !path.to_string_lossy().ends_with("/")
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

    fn read_children(
        path: &Path,
        max_depth: usize,
        fs: &FileSystem,
    ) -> eyre::Result<Vec<TreeNode>> {
        let err_msg = || format!("Failed to read directory: {}", path.to_string_lossy());

        let mut children = vec![];
        let dir_paths = fs.read_dir(path).wrap_err_with(err_msg)?;
        for path in dir_paths {
            children.push(Self::new_recursive(
                path.wrap_err_with(err_msg)?,
                max_depth - 1,
                fs,
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

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        highlighted: &mut Option<PathBuf>,
        fs: &FileSystem,
    ) -> eyre::Result<ExplorerResponse> {
        Ok(match self {
            TreeNode::UnexploredDir { .. } => {
                unreachable!(); // hopefully
            }
            TreeNode::ExploredDir { children, path } => Self::directory_ui(
                ui,
                TreeNode::name_from_path(path),
                path,
                children,
                highlighted,
                fs,
            )?,
            TreeNode::File { path } => {
                Self::file_ui(ui, TreeNode::name_from_path(path), path, highlighted)
            } // TreeNode::NewFile { path, name } => {
              //     self.new_file_ui(ui, path, name)
              // }
        })
    }
    fn explore(&mut self, fs: &FileSystem) -> eyre::Result<()> {
        if let TreeNode::UnexploredDir { path } = self {
            // TODO: only read children when the header is clicked
            let children = TreeNode::read_children(path, 1, fs)?;
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
        path: &Path,
        children: &mut [TreeNode],
        highlighted: &mut Option<PathBuf>,
        fs: &FileSystem,
    ) -> eyre::Result<ExplorerResponse> {
        let mut action = None;
        let response = CollapsingHeader::new(name).show(ui, |ui| {
            for child in children {
                child.explore(fs)?;
                let response = child.ui(ui, highlighted, fs)?;

                if let Some(child_action) = response.action {
                    action = Some(child_action);
                }
            }
            Ok(())
        });

        if response.header_response.clicked() {
            *highlighted = Some(path.to_owned());
        }

        if let Some(Err(err)) = response.body_returned {
            return Err(err);
        }

        Popup::context_menu(&response.header_response).show(|ui| {
            // if ui.button("New file").clicked() {
            //     action = Some(ExplorerAction::NewFile(path.to_owned()));
            // }
            if ui.button("Rename").clicked() {
                todo!();
            }
            if ui.button("Delete").clicked() {
                action = Some(ExplorerAction::Delete(path.to_owned()));
            }
        });

        Ok(ExplorerResponse {
            action,
            response: response.header_response,
        })
    }

    fn file_ui(
        ui: &mut egui::Ui,
        name: &str,
        path: &Path,
        highlighted: &mut Option<PathBuf>,
    ) -> ExplorerResponse {
        ui.scope(|ui| {
            if highlighted.as_deref() == Some(path) {
                ui.visuals_mut().button_frame = true;
            }

            let button = ui.button(name);

            if button.clicked() {
                *highlighted = Some(path.to_owned());
            }

            ExplorerResponse {
                action: button
                    .double_clicked()
                    .then(|| ExplorerAction::OpenFile(path.to_owned())),
                response: button,
            }
        })
        .inner
    }
}

pub enum ExplorerAction {
    OpenFile(PathBuf),
    // NewFile(PathBuf),
    NewFolder(PathBuf),
    Delete(PathBuf),
}

pub struct ExplorerResponse {
    pub response: Response,
    /// The file to be opened, if any
    pub action: Option<ExplorerAction>,
}

// #[derive(Serialize, Deserialize, Debug)]
// FIXME
#[derive(Debug)]
pub struct Explorer {
    pub root_node: TreeNode,
    pub highlighted: Option<PathBuf>,
    pub new_item: Option<(PathBuf, String)>,
}

impl Explorer {
    pub fn new(path: PathBuf, fs: &FileSystem) -> eyre::Result<Self> {
        Ok(Self {
            root_node: TreeNode::new(path, fs)?,
            highlighted: None,
            new_item: None,
        })
    }

    pub fn root_path(&self) -> &Path {
        match &self.root_node {
            TreeNode::UnexploredDir { path } | TreeNode::ExploredDir { path, .. } => path,
            TreeNode::File { .. } => panic!("explorer root node isn't a directory"),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, fs: &FileSystem) -> eyre::Result<ExplorerResponse> {
        let explorer = ScrollArea::vertical()
            .show(ui, |ui| {
                ui.style_mut().visuals.button_frame = false;
                self.root_node.ui(ui, &mut self.highlighted, fs)
            })
            .inner?;

        // FIXME
        // if explorer.response.clicked_elsewhere() {
        //     self.highlighted = None;
        // }

        Ok(explorer)
    }

    // fn new_file_ui(&mut self, path: &Path, name: &mut String, ui: &mut egui::Ui, fs: &FileSystem) -> ExplorerResponse {
    //     let response = ui.text_edit_singleline(name);

    //     if response.lost_focus() {
    //         let new_path = path.join(name);
    //         fs.new_file(&new_path);

    //     }

    //     ExplorerResponse {
    //         action: None,
    //         response,
    //     }
    // }
}
