use std::net::SocketAddr;
use std::sync::LazyLock;

use futures::SinkExt;
use futures::StreamExt;
use linkme::distributed_slice;
use tokio_util::codec::FramedRead;
use tokio_util::codec::FramedWrite;
use tokio_util::codec::LengthDelimitedCodec;
use ygopro_data::complex::Complex;
use ygopro_data::message::ctos;
use ygopro_data::message::stoc;
use ygopro_derive::Configuration;
use ygopro_derive::after;
use ygopro_derive::register_to;

use crate::GlobalHandler;
use crate::room::Player;

static CODEC: LazyLock<LengthDelimitedCodec> = LazyLock::new(|| LengthDelimitedCodec::builder()
    .length_field_type::<u16>()
    .little_endian()
    .new_codec());

#[distributed_slice(crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS)]
pub static NAME: &'static str = module_path!();

#[derive(Clone, Configuration)]
#[config(sync)]
pub struct Configuration {
    #[config(default = "7911")]
    pub port: u16,
}

#[after(crate::message::Init)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as GlobalHandler)]
fn on_init() {
    tokio::spawn(async {
        let configuration = crate::configuration::get();
        let port = configuration.configurations.get::<Configuration>().map(|c| c.port).unwrap_or(7911);
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.expect("cannot bind port");
        log::info!("listening on port {}", port);
        loop {
            let (socket, addr) = listener.accept().await.expect("accept failed");
            handle_connection(socket, addr);
        }
    });
}

type CtosStream = futures::stream::Map<
    futures::stream::FilterMap<
        tokio_util::codec::FramedRead<tokio::net::tcp::OwnedReadHalf, tokio_util::codec::LengthDelimitedCodec>,
        std::future::Ready<Option<bytes::BytesMut>>,
        fn(Result<bytes::BytesMut, std::io::Error>) -> std::future::Ready<Option<bytes::BytesMut>>,
    >,
    fn(bytes::BytesMut) -> Complex<ctos::Message>,
>;

fn filter_error_frames(frame: Result<bytes::BytesMut, std::io::Error>) -> std::future::Ready<Option<bytes::BytesMut>> {
    std::future::ready(frame.ok())
}

fn wrap_in_complex(frame: bytes::BytesMut) -> Complex<ctos::Message> {
    Complex::<ctos::Message>::new(frame.freeze())
}

fn handle_connection(socket: tokio::net::TcpStream, addr: SocketAddr) {
    let (reader, writer) = socket.into_split();
    let framed_read = FramedRead::new(reader, CODEC.clone());
    let framed_write = FramedWrite::new(writer, CODEC.clone());

    let ctos_stream: CtosStream = framed_read
        .filter_map(filter_error_frames as fn(Result<bytes::BytesMut, std::io::Error>) -> std::future::Ready<Option<bytes::BytesMut>>)
        .map(wrap_in_complex as fn(bytes::BytesMut) -> Complex<ctos::Message>);
    let stoc_sink = Box::pin(framed_write.with(|message: Complex<stoc::Message>| async move { Ok::<_, std::io::Error>(message.data) }));
    let mut player = Player::new(stoc_sink);
    player.states.insert(addr);
    player.run(ctos_stream);
}

