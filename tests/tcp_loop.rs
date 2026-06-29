#![allow(dead_code)]

use async_buf::{AsyncRead, AsyncReadBuf, AsyncWrite, AsyncWriteBuf};
use std::{
    io::{self, Read, Write}, net::{SocketAddr, TcpListener, TcpStream}, sync::OnceLock, thread,
};

#[test]
fn tcp_loop() -> anyhow::Result<()> {
    let server_thread = thread::spawn(|| start_server());
    let client_thread = thread::spawn(|| start_client());
    server_thread.join().unwrap()?;
    client_thread.join().unwrap()?;
    Ok(())
}

fn start_server() -> anyhow::Result<()> {
    let mut printer = StatusPrinter::new(color_print::cstr!("<blue>Server</>"));

    printer.start("Starting server");
    let listener = TcpListener::bind("127.0.0.1:0")?;
    SERVER_PORT
        .set(listener.local_addr().unwrap().port())
        .unwrap();
    printer.done();

    printer.start("Accepting client");
    let (mut stream, _) = listener.accept()?;
    printer.done();
    stream.set_nonblocking(true)?;

    let mut seq = AsyncSequence::new();
    seq.start(&mut printer);
    loop {
        if seq.update(&mut stream, &mut printer)? {
            printer.println("Finished!");
            break;
        }
    }

    Ok(())
}

enum AsyncSequenceState {
    NotStarted,
    Recieve1(AsyncReadBuf),
    Send2(AsyncWriteBuf),
    Recieve3(AsyncReadBuf),
    Send4(AsyncWriteBuf),
    Done,
}

struct AsyncSequence {
    state: AsyncSequenceState,
}

impl<'a> AsyncSequence {
    fn new() -> Self {
        Self {
            state: AsyncSequenceState::NotStarted,
        }
    }

    fn start(&mut self, printer: &mut StatusPrinter) {
        self.state = AsyncSequenceState::Recieve1(AsyncReadBuf::new(size_of::<u64>()));
        printer.start("Recieving 1");
    }

    fn update(&mut self, stream: &mut TcpStream, printer: & mut StatusPrinter) -> io::Result<bool> {
        match self.state {
            AsyncSequenceState::NotStarted => (),
            AsyncSequenceState::Recieve1(ref mut buf) => {
                if stream.read_async(buf)? {
                    let value = u64::from_be_bytes(buf.buffer().try_into().unwrap());
                    assert_eq!(value, 1);
                    printer.done();

                    self.state = AsyncSequenceState::Send2((&2u64.to_be_bytes()[..]).into());
                    printer.start("Sending 2");
                }
            }
            AsyncSequenceState::Send2(ref mut buf) => {
                if stream.write_async(buf)? {
                    printer.done();

                    self.state = AsyncSequenceState::Recieve3(AsyncReadBuf::new(size_of::<u64>()));
                    printer.start("Recieving 3");
                }
            }
            AsyncSequenceState::Recieve3(ref mut buf) => {
                if stream.read_async(buf)? {
                    let value = u64::from_be_bytes(buf.buffer().try_into().unwrap());
                    assert_eq!(value, 3);
                    printer.done();

                    self.state = AsyncSequenceState::Send4((&4u64.to_be_bytes()[..]).into());
                    printer.start("Sending 4");
                }
            }
            AsyncSequenceState::Send4(ref mut buf) => {
                if stream.write_async(buf)? {
                    printer.done();
                    self.state = AsyncSequenceState::Done;
                }
            }
            AsyncSequenceState::Done => {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

fn start_client() -> anyhow::Result<()> {
    let mut printer = StatusPrinter::new(color_print::cstr!("<yellow>Client</>"));

    printer.start("Connecting to server");
    let mut stream = TcpStream::connect(SocketAddr::new(
        "127.0.0.1".parse().unwrap(),
        *SERVER_PORT.wait(),
    ))
    .expect("failed to start test client");
    printer.done();

    printer.start("Sending 1");
    stream.write_all(&1u64.to_be_bytes())?;
    printer.done();

    {
        printer.start("Recieving 2");
        let mut buf = [0u8; size_of::<u64>()];
        stream.read_exact(&mut buf)?;
        assert_eq!(u64::from_be_bytes(buf), 2);
        printer.done();
    }

    printer.start("Sending 3");
    stream.write_all(&3u64.to_be_bytes())?;
    printer.done();

    {
        printer.start("Recieving 4");
        let mut buf = [0u8; size_of::<u64>()];
        stream.read_exact(&mut buf)?;
        assert_eq!(u64::from_be_bytes(buf), 4);
        printer.done();
    }

    printer.println("Finished");
    Ok(())
}

static SERVER_PORT: OnceLock<u16> = OnceLock::new();

struct StatusPrinter {
    name: String,
    last_task: Option<String>,
}

impl StatusPrinter {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            last_task: None,
        }
    }

    fn start(&mut self, task: &str) {
        self.last_task = Some(task.to_owned());
        self.println(&format!("{task}..."));
    }

    fn done(&mut self) {
        if let Some(task) = &self.last_task {
            color_print::cprintln!("{}: {task}... <green>Done</>", self.name);
        }

        self.last_task = None;
    }

    fn println(&self, msg: &str) {
        println!("{}: {msg}", self.name);
    }
}