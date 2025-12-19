use eyre::OptionExt;
use futures::{
    StreamExt as _,
    future::FutureExt as _,
    sink::{Send, SinkExt as _},
};
use pollster::FutureExt as _;
use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    panic::AssertUnwindSafe,
    rc::Rc,
    slice::Iter,
    thread,
    vec::Drain,
};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use web_sys::{WebSocket, wasm_bindgen::JsValue};
use ws_messages::{ClientMessage, Command, Response, ServerMessage};
use ws_stream_wasm::{WsErr, WsMessage, WsMeta, WsStream};

mod filesystem;
mod project;
mod runner;

pub use filesystem::*;
pub use project::*;
pub use runner::*;

pub struct Task<T>(Rc<OnceCell<thread::Result<T>>>);

impl<T: 'static> Task<T> {
    pub fn spawn<F: Future<Output = T> + 'static>(future: F) -> Self {
        let tx = Rc::new(OnceCell::new());
        let rx = tx.clone();

        spawn_local(async move {
            let future = AssertUnwindSafe(future).catch_unwind();
            let _ = tx.set(future.await);
        });

        Self(rx)
    }

    pub fn output(&self) -> Option<&thread::Result<T>> {
        self.0.get()
    }
}

#[derive(Default, Debug, Clone)]
pub struct BackendHandle {
    ws: WebSocketHandle,
    pending: PendingOperations,
}

impl BackendHandle {
    pub fn new(url: &str) -> Result<Self, WsErr> {
        Ok(Self {
            ws: WebSocketHandle::new(url)?,
            pending: Default::default(),
        })
    }

    pub fn update(&mut self) {
        while let Some(msg) = self.ws.next_ready() {
            match msg {
                WsMessage::Binary(bytes) => {
                    let resp = ServerMessage::decode(&bytes).expect("TODO: failed to decode");
                    self.pending.add_resp(resp);
                }
                WsMessage::Text(_text) => todo!(),
            }
        }
    }

    pub fn responses(&mut self) -> impl Iterator<Item = eyre::Result<(Command, Response)>> {
        self.pending.responses()
    }

    pub fn send(&self, cmd: Command) {
        let msg = ClientMessage::new(cmd);
        let binary = msg.encode().expect("TODO: return encode error");

        let send = self.ws.stream().send(WsMessage::Binary(binary));
        self.pending.send(send);
    }
}

#[derive(Default, Debug, Clone)]
struct WebSocketHandle(Option<Rc<(WsMeta, WsStream)>>);

impl WebSocketHandle {
    fn new(url: &str) -> Result<Self, WsErr> {
        let ws = WsMeta::connect(url).block_on()?;

        Ok(Self(Some(Rc::new(ws))))
    }

    // TODO: rename get stream
    fn stream(&mut self) -> &mut WsStream {
        &mut self.0.as_mut().expect("no websocket").1
    }

    fn next_ready(&mut self) -> Option<WsMessage> {
        let mut next = self.stream().next();
        (&mut next).now_or_never().flatten()
    }
}

#[derive(Default, Debug, Clone)]
pub struct PendingOperations(Rc<RefCell<PendingInner>>);

impl PendingOperations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send(&self, cmd: Command, send: Send<'static, WsStream, WsMessage>, id: Uuid) {
        self.0.borrow_mut().messages.insert(id, cmd);

        let sender = self.0.clone();
        spawn_local(async move {
            if let Err(err) = send.await {
                sender.borrow_mut().push_send_err((id, err));
            }
        });
    }

    fn add_resp(&self, resp: ServerMessage) {
        self.0.borrow_mut().push_resp(resp);
    }

    fn responses(&self) -> impl Iterator<Item = eyre::Result<(Command, Response)>> {
        let drained: Vec<_> = self.0.borrow_mut().responses.drain(..).collect();
        drained
            .into_iter()
            .map(|msg| self.0.borrow_mut().response_pair(msg))
    }
}

#[derive(Default, Debug, Clone)]
pub struct PendingInner {
    messages: HashMap<Uuid, Command>,
    send_errs: Vec<(Uuid, WsErr)>,
    responses: Vec<ServerMessage>,
}

impl PendingInner {
    fn push_send_err(&mut self, err: (Uuid, WsErr)) {
        self.send_errs.push(err);
    }

    fn push_resp(&mut self, resp: ServerMessage) {
        self.responses.push(resp);
    }

    fn response_pair(&self, msg: ServerMessage) -> eyre::Result<(Command, Response)> {
        Ok((
            self.messages
                .get(&msg.id)
                .ok_or_eyre("recieved invalid message from server")?
                .clone(),
            msg.resp,
        ))
    }
}
