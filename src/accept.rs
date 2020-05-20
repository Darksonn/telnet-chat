use std::net::SocketAddr;
use std::io;

use crate::{ClientHandle, ServerHandle, ToServer};
use crate::client::{start_client, ClientData};

use tokio::net::TcpListener;
use tokio::sync::mpsc::channel;

use futures::future::{AbortHandle, Abortable};

pub async fn start_accept(bind: SocketAddr, mut handle: ServerHandle) {
    let res = accept_loop(bind, handle.clone()).await;
    match res {
        Ok(()) => {},
        Err(err) => {
            handle.send(ToServer::FatalError(err)).await;
        },
    }
}

pub async fn accept_loop(
    bind: SocketAddr,
    handle: ServerHandle
) -> Result<(), io::Error> {

    let mut listen = TcpListener::bind(bind).await?;

    loop {
        let (tcp, ip) = listen.accept().await?;

        let (send, recv) = channel(64);
        let (abort_handle, abort_regis) = AbortHandle::new_pair();

        let id = handle.next_id();

        let client = ClientHandle {
            id,
            ip,
            chan: send,
            kill: abort_handle,
        };

        let mut handle = handle.clone();

        let task_to_spawn = async move {

            handle.send(ToServer::NewClient(client)).await;

            start_client(ClientData {
                id,
                tcp,
                recv,
                handle,
            }).await;

        };

        tokio::spawn(Abortable::new(task_to_spawn, abort_regis));
    }
}
