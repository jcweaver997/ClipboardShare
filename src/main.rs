use arboard::{Clipboard, ImageData};
use bincode;
use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::thread;
use std::time::Duration;

const GROUP_ADDR: &str = "234.21.76.126";
const PORT: u16 = 8473;
const SECRET_CODE: [u8; 4] = [44, 76, 123, 65];
const MAX_DATAGRAM: usize = 60_000;
const CHUNK_SIZE: usize = MAX_DATAGRAM - 50;

#[derive(Serialize, Deserialize)]
enum ClipboardData {
    Text(String),
    Image(Image),
}

#[derive(Serialize, Deserialize, Clone)]
struct Image {
    width: usize,
    height: usize,
    bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct Packet {
    secret: [u8; 4],
    id: u32,
    seq: u16,
    total: u16,
    chunk: Vec<u8>,
}

struct Partial {
    total: u16,
    chunks: Vec<Option<Vec<u8>>>,
    received: u16,
}

fn main() -> std::io::Result<()> {
    // Create UDP socket bound to the multicast port and join group
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT);
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.join_multicast_v4(&GROUP_ADDR.parse().unwrap(), &Ipv4Addr::UNSPECIFIED)?;

    let socket: UdpSocket = socket.into();

    let listener = socket.try_clone()?;
    thread::spawn(move || listen(listener));
    check_clipboard_change(socket);
    Ok(())
}

fn check_clipboard_change(socket: UdpSocket) {
    let mut clipboard = Clipboard::new().unwrap();
    let mut last_text = clipboard.get_text().unwrap_or_default();
    let mut last_img_hash = clipboard
        .get_image()
        .ok()
        .map(|img| hash_bytes(img.bytes.as_ref()));
    loop {
        thread::sleep(Duration::from_millis(100));
        if let Ok(text) = clipboard.get_text() {
            if text != last_text && !text.is_empty() {
                last_text = text.clone();
                send_clipboard_data(&socket, ClipboardData::Text(text));
                continue;
            }
        }

        if let Ok(img) = clipboard.get_image() {
            let hash = hash_bytes(img.bytes.as_ref());
            if Some(hash) != last_img_hash {
                last_img_hash = Some(hash);
                let owned = Image {
                    width: img.width,
                    height: img.height,
                    bytes: img.bytes.into_owned(),
                };
                send_clipboard_data(&socket, ClipboardData::Image(owned));
            }
        }
    }
}

fn listen(socket: UdpSocket) {
    let mut clipboard = Clipboard::new().unwrap();
    let mut buf = [0u8; 65535];
    let mut partials: HashMap<u32, Partial> = HashMap::new();
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, _src)) => {
                if let Ok(packet) = bincode::deserialize::<Packet>(&buf[..len]) {
                    if packet.secret != SECRET_CODE {
                        continue;
                    }

                    let entry = partials.entry(packet.id).or_insert_with(|| Partial {
                        total: packet.total,
                        chunks: vec![None; packet.total as usize],
                        received: 0,
                    });

                    if packet.seq < packet.total && entry.chunks[packet.seq as usize].is_none() {
                        entry.chunks[packet.seq as usize] = Some(packet.chunk);
                        entry.received += 1;
                    }

                    if entry.received == entry.total {
                        let mut data = Vec::new();
                        for c in entry.chunks.iter_mut() {
                            if let Some(chunk) = c.take() {
                                data.extend_from_slice(&chunk);
                            }
                        }
                        if let Ok(msg) = bincode::deserialize::<ClipboardData>(&data) {
                            apply_clipboard(&mut clipboard, msg);
                        }
                        partials.remove(&packet.id);
                    }
                }
            }
            Err(e) => {
                eprintln!("socket error: {}", e);
                break;
            }
        }
    }
}

fn send_clipboard_data(socket: &UdpSocket, data: ClipboardData) {
    if let Ok(encoded) = bincode::serialize(&data) {
        let id: u32 = rand::random();
        let total = ((encoded.len() + CHUNK_SIZE - 1) / CHUNK_SIZE) as u16;
        for seq in 0..total {
            let start = seq as usize * CHUNK_SIZE;
            let end = std::cmp::min(start + CHUNK_SIZE, encoded.len());
            let packet = Packet {
                secret: SECRET_CODE,
                id,
                seq,
                total,
                chunk: encoded[start..end].to_vec(),
            };
            if let Ok(bytes) = bincode::serialize(&packet) {
                let _ = socket.send_to(&bytes, (GROUP_ADDR, PORT));
            }
        }
    }
}

fn apply_clipboard(clipboard: &mut Clipboard, data: ClipboardData) {
    match data {
        ClipboardData::Text(t) => {
            let _ = clipboard.set_text(t);
        }
        ClipboardData::Image(img) => {
            let img = ImageData {
                width: img.width,
                height: img.height,
                bytes: img.bytes.into(),
            };
            let _ = clipboard.set_image(img);
        }
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
