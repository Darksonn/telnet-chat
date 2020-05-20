use std::io;
use std::collections::HashMap;

use tokio::sync::mpsc::{channel, Receiver};
use tokio::task::JoinHandle;

use crate::{ClientId, ClientHandle, FromServer, ToServer, ServerHandle};

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
                let mut to_remove = Vec::new();
                for (id, handle) in data.clients.iter_mut() {
                    let id = *id;
                    if id == from_id { continue; }

                    let msg = FromServer::Message(msg.clone());

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
                for id in to_remove {
                    data.clients.remove(&id);
                }
            },
            ToServer::FatalError(err) => return Err(err),
        }
    }

    Ok(())
}
