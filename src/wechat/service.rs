use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use salvo::conn::TcpListener;
use salvo::prelude::*;
use salvo::websocket::{WebSocketUpgrade, Message, WebSocket};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot, broadcast};
use tracing::{debug, info, warn};

use super::{Message as WxMessage, Request as WxRequest, Response as WxResponse, Event, RequestType, MessageType};
use super::{UserInfo, GroupInfo, WechatClient};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
struct Connection {
    addr: String,
    tx: mpsc::UnboundedSender<String>,
}

struct PendingRequest {
    tx: oneshot::Sender<WxResponse>,
}

#[derive(Clone)]
pub struct WechatService {
    addr: String,
    secret: String,
    connections: Arc<RwLock<HashMap<String, Connection>>>,
    pending_requests: Arc<Mutex<HashMap<i64, PendingRequest>>>,
    request_id: Arc<AtomicI64>,
    clients: Arc<RwLock<HashMap<String, WechatClient>>>,
    event_tx: broadcast::Sender<(String, Event)>,
}

impl WechatService {
    pub fn new(addr: impl Into<String>, secret: impl Into<String>) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            addr: addr.into(),
            secret: secret.into(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            request_id: Arc::new(AtomicI64::new(0)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<(String, Event)> {
        self.event_tx.subscribe()
    }

    /// Create or retrieve a WechatClient for the given MXID.
    pub async fn new_client(self: &Arc<Self>, mxid: &str) -> WechatClient {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get(mxid) {
            return client.clone();
        }
        let client = WechatClient::new(mxid.to_string(), Arc::clone(self));
        clients.insert(mxid.to_string(), client.clone());
        client
    }

    /// Graceful shutdown: close all WebSocket connections.
    pub async fn stop(&self) {
        info!("WechatService stopping");
        let conns = self.connections.read().await;
        for (addr, _conn) in conns.iter() {
            info!("Closing connection to {}", addr);
        }
        drop(conns);

        let mut conns = self.connections.write().await;
        conns.clear();
    }

    fn next_request_id(&self) -> i64 {
        self.request_id.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub async fn request(&self, mxid: &str, req: &WxRequest) -> Result<WxResponse> {
        let id = self.next_request_id();
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, PendingRequest { tx });
        }

        let msg = WxMessage {
            id,
            mxid: mxid.to_string(),
            msg_type: MessageType::Request,
            data: serde_json::to_value(req).ok(),
        };

        let conn = self.get_connection().await;
        if let Some(conn) = conn {
            debug!("Send request message #{} {}", id, req.request_type);
            let json = serde_json::to_string(&msg)?;
            conn.tx.send(json)?;
        } else {
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&id);
            return Err(anyhow!("no agent connection available"));
        }

        match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(response)) => {
                debug!("Receive response message #{} {}", id, response.response_type);
                Ok(response)
            }
            Ok(Err(_)) => Err(anyhow!("response channel closed")),
            Err(_) => {
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&id);
                Err(anyhow!("request timeout"))
            }
        }
    }

    async fn get_connection(&self) -> Option<Connection> {
        let conns = self.connections.read().await;
        conns.values().next().cloned()
    }

    async fn handle_json_message(&self, json: &str) {
        if let Ok(msg) = serde_json::from_str::<WxMessage>(json) {
            match msg.msg_type {
                MessageType::Request => {
                    if let Some(data) = &msg.data {
                        if let Ok(request) = serde_json::from_value::<WxRequest>(data.clone()) {
                            if request.request_type == RequestType::Event {
                                if let Some(event_data) = &request.data {
                                    if let Ok(event) = serde_json::from_value::<Event>(event_data.clone()) {
                                        let _ = self.event_tx.send((msg.mxid.clone(), event));
                                    }
                                }
                            }
                        }
                    }
                }
                MessageType::Response => {
                    if let Some(data) = &msg.data {
                        if let Ok(response) = serde_json::from_value::<WxResponse>(data.clone()) {
                            let mut pending = self.pending_requests.lock().await;
                            if let Some(req) = pending.remove(&msg.id) {
                                let _ = req.tx.send(response);
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        let addr = self.addr.clone();
        info!("WeChat service listening on {}", addr);
        
        let router = Router::new()
            .push(Router::with_path("/").get(WebSocketHandler {
                secret: self.secret.clone(),
                connections: self.connections.clone(),
                pending_requests: self.pending_requests.clone(),
                event_tx: self.event_tx.clone(),
            }));

        let listener = TcpListener::new(addr).bind().await;
        Server::new(listener).serve(router).await;
        
        Ok(())
    }
}

struct WebSocketHandler {
    secret: String,
    connections: Arc<RwLock<HashMap<String, Connection>>>,
    pending_requests: Arc<Mutex<HashMap<i64, PendingRequest>>>,
    event_tx: broadcast::Sender<(String, Event)>,
}

#[handler]
impl WebSocketHandler {
    async fn handle(&self, req: &mut Request, res: &mut Response) -> Result<(), StatusError> {
        let auth_header: Option<String> = req.header::<String>("Authorization");
        
        let authorized = match auth_header {
            Some(header) if header.starts_with("Basic ") => {
                &header[6..] == self.secret
            }
            _ => false,
        };
        
        if !authorized {
            return Err(StatusError::forbidden());
        }

        let addr = req.remote_addr().to_string();
        let connections = self.connections.clone();
        let pending_requests = self.pending_requests.clone();
        let event_tx = self.event_tx.clone();
        
        WebSocketUpgrade::new()
            .upgrade(req, res, move |socket: WebSocket| async move {
                handle_socket(socket, addr, connections, pending_requests, event_tx).await
            })
            .await
    }
}

async fn handle_socket(
    mut socket: WebSocket,
    addr: String,
    connections: Arc<RwLock<HashMap<String, Connection>>>,
    pending_requests: Arc<Mutex<HashMap<i64, PendingRequest>>>,
    event_tx: broadcast::Sender<(String, Event)>,
) {
    info!("Agent connected from {}", addr);
    
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
    let conn = Connection {
        addr: addr.clone(),
        tx,
    };
    {
        let mut conns = connections.write().await;
        conns.insert(addr.clone(), conn);
    }
    
    loop {
        tokio::select! {
            json = rx.recv() => {
                match json {
                    Some(json) => {
                        if socket.send(Message::text(json)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(msg)) if msg.is_text() => {
                        if let Ok(text) = msg.as_str() {
                            if let Ok(wx_msg) = serde_json::from_str::<WxMessage>(text) {
                                match wx_msg.msg_type {
                                    MessageType::Request => {
                                        if let Some(data) = &wx_msg.data {
                                            if let Ok(request) = serde_json::from_value::<WxRequest>(data.clone()) {
                                                if request.request_type == RequestType::Event {
                                                    if let Some(event_data) = &request.data {
                                                        if let Ok(event) = serde_json::from_value::<Event>(event_data.clone()) {
                                                            let _ = event_tx.send((wx_msg.mxid.clone(), event));
                                                        }
                                                    }
                                                } else {
                                                    warn!("Request {} not supported", request.request_type);
                                                }
                                            }
                                        }
                                    }
                                    MessageType::Response => {
                                        if let Some(data) = &wx_msg.data {
                                            if let Ok(response) = serde_json::from_value::<WxResponse>(data.clone()) {
                                                let mut pending = pending_requests.lock().await;
                                                if let Some(req) = pending.remove(&wx_msg.id) {
                                                    let _ = req.tx.send(response);
                                                } else {
                                                    warn!("Dropping response to {}: unknown request ID", wx_msg.id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(msg)) if msg.is_close() => break,
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }
    
    {
        let mut conns = connections.write().await;
        conns.remove(&addr);
    }
    info!("Agent disconnected from {}", addr);
}
