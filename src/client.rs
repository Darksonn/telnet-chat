use std::io;

use tokio::{try_join, select};
use tokio::sync::mpsc::{unbounded_channel, Receiver, UnboundedSender, UnboundedReceiver};
use tokio::net::{TcpStream, tcp::{ReadHalf, WriteHalf}};
use tokio::stream::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio_util::codec::FramedRead;

use crate::{ClientId, ServerHandle, ToServer, FromServer};
use crate::telnet::{TelnetCodec, Item};

pub struct ClientData {
    pub id: ClientId,
    pub recv: Receiver<FromServer>,
    pub handle: ServerHandle,
    pub tcp: TcpStream,
}

pub async fn start_client(data: ClientData) {
    let res = client_loop(data).await;
    match res {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Something went wrong: {}.", err);
        },
    }
}

async fn client_loop(mut data: ClientData) -> Result<(), io::Error> {
    let (read, write) = data.tcp.split();

    // communication between tcp_read and tcp_write
    let (send, recv) = unbounded_channel();

    let ((), ()) = try_join! {
        tcp_read(data.id, read, data.handle, send),
        tcp_write(write, data.recv, recv),
    }?;

    Ok(())
}

#[derive(Debug)]
enum InternalMsg {
    GotAreYouThere,
    SendDont(u8),
    SendWont(u8),
    SendDo(u8),
}

async fn tcp_read(
    id: ClientId,
    read: ReadHalf<'_>,
    mut handle: ServerHandle,
    to_tcp_write: UnboundedSender<InternalMsg>,
) -> Result<(), io::Error> {
    let mut telnet = FramedRead::new(read, TelnetCodec::new());

    while let Some(item) = telnet.next().await {
        match item? {
            Item::Line(line) => {
                handle.send(ToServer::Message(id, line)).await;
            },
            Item::AreYouThere => {
                to_tcp_write.send(InternalMsg::GotAreYouThere)
                    .expect("Should not be closed.");
            },
            Item::GoAhead => { /* ignore */ },
            Item::InterruptProcess => return Ok(()),
            Item::Will(3) => { // suppress go-ahead
                to_tcp_write.send(InternalMsg::SendDo(3))
                    .expect("Should not be closed.");
            },
            Item::Will(i) => {
                to_tcp_write.send(InternalMsg::SendDont(i))
                    .expect("Should not be closed.");
            },
            Item::Do(i) => {
                to_tcp_write.send(InternalMsg::SendWont(i))
                    .expect("Should not be closed.");
            },
            item => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Unable to handle {:?}", item),
                ));
            },
        }
    }

    // disconnected

    Ok(())
}

async fn tcp_write(
    mut write: WriteHalf<'_>,
    mut recv: Receiver<FromServer>,
    mut from_tcp_read: UnboundedReceiver<InternalMsg>,
) -> Result<(), io::Error> {
    loop {
        select! {
            msg = recv.recv() => match msg {
                Some(FromServer::Message(msg)) => {
                    write.write_all(&msg).await?;
                    write.write_all(&[13, 10]).await?;
                },
                None => {
                    break;
                },
            },
            msg = from_tcp_read.recv() => match msg {
                Some(InternalMsg::GotAreYouThere) => {
                    write.write_all(b"Yes.\r\n").await?;
                },
                Some(InternalMsg::SendDont(i)) => {
                    write.write_all(&[0xff, 254, i]).await?;
                },
                Some(InternalMsg::SendWont(i)) => {
                    write.write_all(&[0xff, 252, i]).await?;
                },
                Some(InternalMsg::SendDo(i)) => {
                    write.write_all(&[0xff, 253, i]).await?;
                },
                None => {
                    break;
                },
            },
        };
    }

    Ok(())
}
