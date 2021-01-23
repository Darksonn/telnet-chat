use std::io;
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use tokio::sync::mpsc::Sender;

use tokio::sync::mpsc::{channel, Receiver};
use tokio::task::JoinHandle;

use crate::ClientId;
use crate::client::{ClientHandle, FromServer};

/// This struct is used by client actors to send messages to the main loop. The
/// message type is `ToServer`.
#[derive(Clone, Debug)]
pub struct ServerHandle {
    chan: Sender<ToServer>,
    next_id: Arc<AtomicUsize>,
}
impl ServerHandle {
    pub async fn send(&mut self, msg: ToServer) {
        if self.chan.send(msg).await.is_err() {
            panic!("Main loop has shut down.");
        }
    }
    pub fn next_id(&self) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ClientId(id)
    }
}

/// The message type used when a client actor sends messages to the main loop.
pub enum ToServer {
    NewClient(ClientHandle),
    Message(ClientId, Vec<u8>),
    FatalError(io::Error),
}

pub fn spawn_main_loop() -> (ServerHandle, JoinHandle<()>) {
    let (send, recv) = channel(64);

    let handle = ServerHandle {
        chan: send,
        next_id: Default::default(),
    };

    let join = tokio::spawn(async move {
        let res = main_loop(recv).await;
        match res {
            Ok(()) => {},
            Err(err) => {
                eprintln!("Oops {}.", err);
            },
        }
    });

    (handle, join)
}

#[derive(Default, Debug)]
struct Data {
    clients: HashMap<ClientId, ClientHandle>,
}

async fn main_loop(
    mut recv: Receiver<ToServer>,
) -> Result<(), io::Error> {
    let mut data = Data::default();

    while let Some(msg) = recv.recv().await {
        match msg {
            ToServer::NewClient(handle) => {
                data.clients.insert(handle.id, handle);
            },
            ToServer::Message(from_id, msg) => {
                // If we fail to send messages to any actor, we need to remove
                // it, but we can't do so while iterating.
                let mut to_remove = Vec::new();

                // Iterate through clients so we can send the message.
                for (id, handle) in data.clients.iter_mut() {
                    let id = *id;

                    // Don't send it to the client who sent it to us.
                    if id == from_id { continue; }

                    let msg = FromServer::Message(msg.clone());

                    if handle.send(msg).is_err() {
                        // Remove this client.
                        to_remove.push(id);
                    }
                }

                // Remove those clients.
                for id in to_remove {
                    // The destructor of ClientHandle will kill the actor when
                    // we remove it from the HashMap.
                    data.clients.remove(&id);
                }
            },
            // This message comes only from the accept loop.
            ToServer::FatalError(err) => return Err(err),
        }
    }

    Ok(())
}
