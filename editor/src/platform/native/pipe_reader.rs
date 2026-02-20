use crossbeam_channel::{self as crossbeam, Receiver};
use std::io::{self, Read};
use std::thread;

// Enum to represent possible errors when reading from the pipe
#[derive(Debug)]
pub enum PipeError {
    IO(io::Error),
    NotUtf8,
}

// Enum to represent a line of output or end of file (EOF)
#[derive(Debug)]
pub enum PipedLine {
    Line(String),
    Eof,
}

// FIXME: output doesn't update after flush if the wasn't caused by a new line
pub fn read_piped(
    mut stream: impl Read + Send + 'static,
) -> Receiver<Result<PipedLine, PipeError>> {
    let (tx, rx) = crossbeam::unbounded();

    thread::spawn(move || {
        let mut buf = Vec::new();
        let mut byte = [0u8];
        loop {
            match stream.read(&mut byte) {
                Ok(0) => {
                    // End of file reached
                    let _ = tx.send(Ok(PipedLine::Eof));
                    break;
                }
                Ok(_) => {
                    buf.push(byte[0]);
                    if byte[0] == b'\n' {
                        // Convert buffer to a string and send it
                        let line = String::from_utf8(buf.clone()).map_err(|_| PipeError::NotUtf8);
                        let _ = tx.send(line.map(PipedLine::Line));
                        buf.clear();
                    }
                }
                Err(error) => {
                    // Send error message
                    let _ = tx.send(Err(PipeError::IO(error)));
                }
            }
        }
    });

    rx
}
