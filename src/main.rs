#[tokio::main]
async fn main() {
    let (handle, join) = telnet_chat::main_loop::spawn_main_loop();

    tokio::spawn(async move {
        let bind = ([0, 0, 0, 0], 3456).into();
        telnet_chat::accept::start_accept(bind, handle).await;
    });

    println!("Starting on port 3456");

    join.await.unwrap();
}
