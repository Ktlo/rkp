use std::{fmt::Display, net::SocketAddr, str::from_utf8};

use bytes::{Buf, Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::chain;

pub async fn actor(address: SocketAddr, chain: String) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&address).await?;

    loop {
        let chain = chain.clone();
        let (stream, client_addr) = listener.accept().await?;
        tokio::spawn(async move {
            log::debug!("TLS connection accepted: {}", &client_addr);
            if let Err(error) = proxy(stream, chain).await {
                log::debug!("an error occurred in TLS connection; error = {}", error);
            };
        });
    }
}

trait BufferExt {
    fn get_u24(&mut self) -> u32;
}

impl BufferExt for Bytes {
    fn get_u24(&mut self) -> u32 {
        let bytes: [u8; 4] = [0, self.get_u8(), self.get_u8(), self.get_u8()];
        u32::from_be_bytes(bytes)
    }
}

const CONTENT_TYPE_HANDSHAKE: u8 = 22;
const HANDSHAKE_TYPE_CLIENT_HELLO: u8 = 1;
const EXTENSION_SERVER_NAME: u16 = 0;
const EXTENSION_SERVER_NAME_TYPE_HOSTNAME: u8 = 0;

async fn proxy(mut stream: TcpStream, chain: String) -> anyhow::Result<()> {
    let content_type = stream.read_u8().await?;
    validate(CONTENT_TYPE_HANDSHAKE, content_type, "content type")?;
    let version = stream.read_u16().await?;
    let message_length = stream.read_u16().await?;
    let mut buffer = BytesMut::with_capacity(message_length.into());
    stream.read_buf(&mut buffer).await?;
    let mut buff_a = buffer.freeze();
    let mut buff_b = buff_a.clone();
    let handshake_type = buff_a.get_u8();
    validate(
        HANDSHAKE_TYPE_CLIENT_HELLO,
        handshake_type,
        "handshake type",
    )?;
    let hello_length = buff_a.get_u24();
    validate(
        (message_length - 4).into(),
        hello_length,
        "handshake message length",
    )?;
    buff_a.advance(2); // skip handshake version
    buff_a.advance(32); // skip random
    let session_id_length = buff_a.get_u8();
    buff_a.advance(session_id_length.into()); // skip session id
    let cipher_suits_length = buff_a.get_u16();
    buff_a.advance(cipher_suits_length.into());
    let compression_methods_length = buff_a.get_u8();
    buff_a.advance(compression_methods_length.into());
    let extension_methods_length: u16 = buff_a.get_u16();
    let mut remains = extension_methods_length;
    let server_name = loop {
        let extension_type = buff_a.get_u16();
        let extension_length: u16 = buff_a.get_u16();
        if extension_type == EXTENSION_SERVER_NAME {
            let server_name_list_length = buff_a.get_u16();
            validate(
                extension_length - 2,
                server_name_list_length,
                "extension server_name list length",
            )?;
            if server_name_list_length == 0 {
                continue;
            }
            let server_name_type = buff_a.get_u8();
            if server_name_type == EXTENSION_SERVER_NAME_TYPE_HOSTNAME {
                let server_name_length = buff_a.get_u16();
                validate(
                    server_name_list_length - 3,
                    server_name_length,
                    "extension server_name server name length",
                )?;
                let server_name_length: usize = server_name_length.into();
                let server_name = &buff_a.chunk()[0..server_name_length];
                break from_utf8(server_name).unwrap();
            }
        } else {
            buff_a.advance(extension_length.into());
        }
        log::debug!(
            "extension_length = {}, remains = {}",
            extension_length,
            remains
        );
        remains -= 4 + extension_length;
        if remains == 0 {
            return Err(anyhow::anyhow!("no server_name extension found; drop"));
        };
    };
    log::debug!("TLS connect to \"{}\"", server_name);
    let context = chain::Context {
        host: server_name.to_owned(),
        port: 443,
        address: format!("{}:{}", server_name, 443),
    };
    let mut proxy = chain::connect(context, chain.to_owned()).await?;
    proxy.write_u8(content_type).await?;
    proxy.write_u16(version).await?;
    proxy.write_u16(message_length).await?;
    proxy.write_all_buf(&mut buff_b).await?;
    proxy.flush().await?;
    tokio::io::copy_bidirectional(&mut stream, &mut proxy).await?;
    Ok(())
}

fn validate<T>(expect: T, actual: T, field_name: &str) -> anyhow::Result<()>
where
    T: Eq + Display,
{
    if expect == actual {
        Ok(())
    } else {
        let message = format!(
            "wrong TLS field \"{}\" value ({} expected, got {})",
            field_name, expect, actual
        );
        log::debug!("{}", &message);
        Err(anyhow::Error::msg(message))
    }
}
