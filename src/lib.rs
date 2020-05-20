use std::net::SocketAddr;
use std::io;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

use tokio::sync::mpsc::Sender;

use futures::future::AbortHandle;

pub mod accept;
pub mod client;
pub mod telnet;
pub mod main_loop;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ClientId(usize);

pub enum ToServer {
    NewClient(ClientHandle),
    Message(ClientId, Vec<u8>),
    FatalError(io::Error),
}
pub enum FromServer {
    Message(Vec<u8>),
}

#[derive(Clone, Debug)]
pub struct ServerHandle {
    chan: Sender<ToServer>,
    next_id: Arc<AtomicUsize>,
}
impl ServerHandle {
    pub async fn send(&mut self, msg: ToServer) {
        if self.chan.send(msg).await.is_err() {
            panic!("Game loop has shut down.");
        }
    }
    pub fn next_id(&self) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ClientId(id)
    }
}

#[derive(Debug)]
pub struct ClientHandle {
    id: ClientId,
    ip: SocketAddr,
    chan: Sender<FromServer>,
    kill: AbortHandle,
}

impl ClientHandle {
    pub fn send(&mut self, msg: FromServer) -> Result<(), io::Error> {
        if self.chan.try_send(msg).is_err() {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Can't keep up or dead"))
        } else {
            Ok(())
        }
    }
    pub fn kill(self) {
        // self goes out of scope now
    }
}

impl Drop for ClientHandle {
    fn drop(&mut self) {
        self.kill.abort()
    }
}
