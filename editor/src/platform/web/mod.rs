use eyre::OptionExt;
use futures::{
    StreamExt as _,
    future::FutureExt as _,
    sink::{Send, SinkExt as _},
};
use poll_promise::Promise;
use std::{
    cell::{OnceCell, RefCell, RefMut},
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
    pub async fn new(url: &str) -> Result<Self, WsErr> {
        Ok(Self {
            ws: WebSocketHandle::new(url).await?,
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
        let msg = ClientMessage::new(cmd.clone());
        let binary = msg.encode().expect("TODO: return encode error");

        self.pending.send(msg, self.ws.clone(), WsMessage::Binary(binary));
    }
}

#[derive(Default, Debug, Clone)]
struct WebSocketHandle(Option<Rc<RefCell<(WsMeta, WsStream)>>>);

impl WebSocketHandle {
    pub async fn new(url: &str) -> Result<Self, WsErr> {
        let url = url.to_string();
        let ws = WsMeta::connect(url.clone(), None).await?;
    
        web_sys::console::log_1(&format!("connected to {url}: {ws:?}").into());

        Ok(Self(Some(Rc::new(RefCell::new(ws)))))
    }

    fn stream(&self) -> RefMut<'_, WsStream> {
        RefMut::map(self.0.as_ref().expect("no websocket").borrow_mut(), |(_, stream)| stream)
    }

    pub fn send(&self, msg: WsMessage) -> impl Future<Output = Result<(), WsErr>> {
        let handle = self.clone();
        async move {
            let mut stream = handle.stream();
            stream.send(msg).await
        }
    }

    pub fn next_ready(&mut self) -> Option<WsMessage> {
        let mut stream = self.stream();
        let mut next = stream.next();
        (&mut next).now_or_never().flatten()
    }
}

#[derive(Default, Debug, Clone)]
pub struct PendingOperations(Rc<RefCell<PendingInner>>);

impl PendingOperations {
    pub fn new() -> Self {
        Self::default()
    }

    fn send(&self, client_msg: ClientMessage, ws: WebSocketHandle, ws_msg: WsMessage) {
        self.0.borrow_mut().messages.insert(client_msg.id, client_msg.cmd);

        let sender = self.0.clone();
        spawn_local(async move {
            if let Err(err) = ws.send(ws_msg).await {
                sender.borrow_mut().push_send_err((client_msg.id, err));
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
        web_sys::console::log_1(&format!("finding pair for {:?} in {:?}", msg, self.messages).into());

        Ok((
            self.messages
                .get(&msg.id)
                .ok_or_eyre("received invalid message from server")?
                .clone(),
            msg.resp,
        ))
    }
}
