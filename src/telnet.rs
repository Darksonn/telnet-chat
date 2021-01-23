use std::io;
use tokio_util::codec::Decoder;

use bytes::{Buf, BytesMut};

pub struct TelnetCodec {
    current_line: Vec<u8>,
}

impl TelnetCodec {
    pub fn new() -> Self {
        TelnetCodec {
            current_line: Vec::with_capacity(1024),
        }
    }
}

#[derive(Debug)]
pub enum Item {
    Line(Vec<u8>),
    SE,
    DataMark,
    Break,
    InterruptProcess,
    AbortOutput,
    AreYouThere,
    GoAhead,
    SB,
    Will(u8),
    Wont(u8),
    Do(u8),
    Dont(u8),
}

impl Decoder for TelnetCodec {
    type Item = Item;
    type Error = io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut
    ) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            if src.is_empty() {
                return Ok(None);
            }

            if src[0] == 0xff {
                let (res, consume) = try_parse_iac(src.chunk());
                src.advance(consume);

                match res {
                    ParseIacResult::Invalid(err) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            err,
                        ));
                    },
                    ParseIacResult::NeedMore => return Ok(None),
                    ParseIacResult::Item(item) => return Ok(Some(item)),
                    ParseIacResult::NOP => { /* go around loop */ },
                    ParseIacResult::EraseCharacter => {
                        self.current_line.pop();
                    },
                    ParseIacResult::EraseLine => {
                        self.current_line.clear();
                    },
                    ParseIacResult::Escaped => {
                        self.current_line.push(0xff);
                    },
                }
            } else {
                let byte = src.get_u8();

                match byte {
                    10 => {
                        let line = self.current_line.to_vec();
                        self.current_line.clear();

                        return Ok(Some(Item::Line(line)));
                    },
                    0 ..= 31 => {
                        // ignore
                    },
                    _ => self.current_line.push(byte),
                }
            }
        }
    }
}

enum ParseIacResult {
    Invalid(String),
    NeedMore,
    Item(Item),
    NOP,
    EraseCharacter,
    EraseLine,
    Escaped,
}

/// Returns the parsed result of the first few bytes, as well as how many bytes
/// to consume.
fn try_parse_iac(bytes: &[u8]) -> (ParseIacResult, usize) {
    if bytes.len() < 2 {
        return (ParseIacResult::NeedMore, 0);
    }
    if bytes[0] != 0xff {
        unreachable!();
    }
    if is_three_byte_iac(bytes[1]) && bytes.len() < 3 {
        return (ParseIacResult::NeedMore, 0);
    }

    match bytes[1] {
        240 => (ParseIacResult::Item(Item::SE), 2),
        241 => (ParseIacResult::NOP, 2),
        242 => (ParseIacResult::Item(Item::DataMark), 2),
        243 => (ParseIacResult::Item(Item::Break), 2),
        244 => (ParseIacResult::Item(Item::InterruptProcess), 2),
        245 => (ParseIacResult::Item(Item::AbortOutput), 2),
        246 => (ParseIacResult::Item(Item::AreYouThere), 2),
        247 => (ParseIacResult::EraseCharacter, 2),
        248 => (ParseIacResult::EraseLine, 2),
        249 => (ParseIacResult::Item(Item::GoAhead), 2),
        250 => (ParseIacResult::Item(Item::SB), 2),
        251 => (ParseIacResult::Item(Item::Will(bytes[2])), 3),
        252 => (ParseIacResult::Item(Item::Wont(bytes[2])), 3),
        253 => (ParseIacResult::Item(Item::Do(bytes[2])), 3),
        254 => (ParseIacResult::Item(Item::Dont(bytes[2])), 3),
        255 => (ParseIacResult::Escaped, 2),
        cmd => (ParseIacResult::Invalid(format!("Unknown IAC command {}.", cmd)), 0),
    }
}

fn is_three_byte_iac(byte: u8) -> bool {
    match byte {
        251 ..= 254 => true,
        _ => false,
    }
}
