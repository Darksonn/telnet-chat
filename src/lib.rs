pub mod accept;
pub mod client;
pub mod telnet;
pub mod main_loop;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ClientId(usize);

