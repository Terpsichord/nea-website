use eyre::bail;
use std::{process::{Child, Stdio}, sync::{Arc, Mutex, MutexGuard}, thread::{self, JoinHandle}};
use crossbeam_channel as crossbeam;
use eyre::OptionExt;

use super::{pipe_reader::{PipedLine, read_piped}, Project, ProjectSettings};

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
    pub fn run(&mut self, project: &mut Project, output: Arc<Mutex<String>>) -> eyre::Result<()> {
        // update project settings
        match ProjectSettings::read_from(&project.path) {
            Ok(settings) => project.settings = settings,
            Err(err) => Err(err)?,
        }

        let settings = project
            .settings
            .as_ref()
            .ok_or_eyre("No run command set\n\nA project.toml file is needed to set it")?;

        let mut words = match shell_words::split(&settings.run_command) {
            Ok(words) => words.into_iter(),
            Err(_) => bail!("Invalid run command"),
        };
        let command = words.next().ok_or_eyre("Run command is empty")?;

        let args = words.collect::<Vec<String>>();


        // TODO: error handling for this should include handling "program not found" and "invalid input"
        let mut child = std::process::Command::new(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&project.path)
            .args(args)
            .spawn()
            .expect("failed to start subprocess");

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

    pub fn update(&mut self) {
        if self
            .running_command
            .as_ref()
            .is_some_and(|cmd| cmd.thread.is_finished())
        {
            self.running_command = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running_command.is_some()
    }

    pub fn stop(&mut self) {
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
