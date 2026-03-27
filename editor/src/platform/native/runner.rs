use crossbeam_channel as crossbeam;
use eyre::OptionExt;
use eyre::bail;
use std::path::Path;
use std::{
    process::{Child, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::platform::RunnerTrait;
use crate::platform::native::project;

use super::{
    Project, ProjectSettings,
    pipe_reader::{PipedLine, read_piped},
};

#[derive(Default)]
pub struct Runner {
    running_command: Option<RunningCommand>,
}

#[derive(Debug)]
struct RunningCommand {
    process: Child,
    thread: JoinHandle<()>,
}

impl Runner {
    fn execute(&mut self, shell_command: &str, path: &Path) -> eyre::Result<Child> {
        let mut words = match shell_words::split(shell_command) {
            Ok(words) => words.into_iter(),
            Err(_) => bail!("Invalid command"),
        };
        let command = words.next().ok_or_eyre("Command is empty")?;

        let args = words.collect::<Vec<String>>();

        Ok(std::process::Command::new(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(path)
            .args(args)
            .spawn()
            .expect("failed to start subprocess"))
    }
}

impl RunnerTrait for Runner {
    fn run(&mut self, project: &mut Project, output: Arc<Mutex<String>>) -> eyre::Result<()> {
        // update project settings
        match ProjectSettings::read_from(&project.path) {
            Ok(settings) => project.settings = settings,
            Err(err) => Err(err)?,
        }

        let settings = project
            .settings
            .as_ref()
            .ok_or_eyre("No run command set\n\nA project.toml file is needed to set it")?;

        let mut child = self.execute(&settings.run_command, &project.path)?;

        output.lock().expect("failed to lock output").clear();

        // should be able to unwrap these, as we set stdout and stderr in the Command
        let out = read_piped(child.stdout.take().unwrap());
        let err = read_piped(child.stderr.take().unwrap());

        let thread = thread::spawn(move || {
            loop {
                // waits to receive the next line from either stdout or stderr, and processes which ever one is received first
                crossbeam::select! {
                    recv(out) -> msg => match msg {
                        Ok(Ok(PipedLine::Line(line))) => {
                            println!("{:?}", &line);
                            output.lock().expect("failed to lock output").push_str(&(line));
                        }
                        Ok(Ok(PipedLine::Eof)) | Err(_) => break,
                        // TODO: handle this error
                        Ok(Err(err)) => eprintln!("Error: {:?}", err),
                    },
                    recv(err) -> msg => match msg {
                        Ok(Ok(PipedLine::Line(line))) => output.lock().expect("failed to lock output").push_str(&format!("** {} **\n", &line)),
                        Ok(Ok(PipedLine::Eof)) | Err(_) => break,
                        Ok(Err(err)) => eprintln!("Error: {:?}", err),
                    },
                }
            }
        });

        self.running_command = Some(RunningCommand {
            process: child,
            thread,
        });

        Ok(())
    }

    fn format(&mut self, project: &mut Project) -> eyre::Result<()> {
        // update project settings
        match ProjectSettings::read_from(&project.path) {
            Ok(settings) => project.settings = settings,
            Err(err) => Err(err)?,
        }

        let settings = project
            .settings
            .as_ref()
            .ok_or_eyre("No format command set\n\nA project.toml file is needed to set it")?;

        if let Some(cmd) = &settings.format_command {
            let mut child = self.execute(cmd, &project.path)?;

            child.wait().expect("failed to wait for subprocess");
        }

        Ok(())
    }

    fn update(&mut self) {
        if self
            .running_command
            .as_ref()
            .is_some_and(|cmd| cmd.thread.is_finished())
        {
            self.running_command = None;
        }
    }

    fn is_running(&self) -> bool {
        self.running_command.is_some()
    }

    fn stop(&mut self) {
        if let Some(mut running_command) = self.running_command.take() {
            running_command
                .process
                .kill()
                .expect("failed to kill process");
            running_command
                .thread
                .join()
                .expect("failed to join thread");
        }
    }
}
