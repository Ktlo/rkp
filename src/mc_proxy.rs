use std::{fmt::Display, net::SocketAddr};

use tokio::{
    io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::chain::{self, Context};

pub async fn actor(address: SocketAddr, chain: String) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&address).await?;

    loop {
        let chain = chain.clone();
        let (stream, client_addr) = listener.accept().await?;
        tokio::spawn(async move {
            log::debug!("Minecraft connection accepted: {}", &client_addr);
            if let Err(error) = proxy(stream, chain).await {
                log::debug!(
                    "an error occurred in Minecraft connection; error = {}",
                    error
                );
            };
        });
    }
}

const SEGMENT_BITS: u32 = 0x7F;
const CONTINUE_BIT: u32 = 0x80;

#[async_trait::async_trait]
trait McAsyncReadExt: AsyncRead {
    async fn read_varint<'a>(&'a mut self) -> io::Result<u32>
    where
        Self: Unpin,
    {
        let mut value: u32 = 0;
        let mut position: usize = 0;
        loop {
            let byte: u32 = self.read_u8().await?.into();
            value = value | ((byte & SEGMENT_BITS) << position);
            if (byte & CONTINUE_BIT) == 0 {
                break Ok(value);
            };
            position += 7;
            if position >= 32 {
                break Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "VarInt is too big",
                ));
            }
        }
    }

    async fn read_varstring<'a>(&'a mut self) -> io::Result<String>
    where
        Self: Unpin,
    {
        let length: usize = self.read_varint().await?.try_into().unwrap();
        let mut buf = vec![0u8; length];
        self.read_exact(&mut buf).await?;
        let value = String::from_utf8(buf).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("not an utf8 string: {}", e),
            )
        })?;
        Ok(value)
    }
}

#[async_trait::async_trait]
trait McAsyncWriteExt: AsyncWrite {
    async fn write_varint<'a>(&'a mut self, mut value: u32) -> io::Result<()>
    where
        Self: Unpin,
    {
        let mut buf: [u8; 5] = [0, 0, 0, 0, 0];
        let mut position: usize = 0;
        let mut write_byte = |byte: u32| {
            buf[position] = byte.try_into().unwrap();
            position += 1;
        };
        loop {
            if (value & !SEGMENT_BITS) == 0 {
                write_byte(value);
                break;
            }
            write_byte((value & SEGMENT_BITS) | CONTINUE_BIT);
            value = value >> 7;
        }
        self.write_all(&buf[..position]).await
    }

    async fn write_varstring<'a>(&'a mut self, value: &str) -> io::Result<()>
    where
        Self: Unpin,
    {
        let bytes = value.as_bytes();
        self.write_varint(bytes.len().try_into().unwrap()).await?;
        self.write_all(bytes).await
    }
}

impl McAsyncReadExt for TcpStream {}
impl McAsyncWriteExt for TcpStream {}

async fn proxy(mut stream: TcpStream, chain: String) -> anyhow::Result<()> {
    let length = stream.read_varint().await?;
    let packet_id = stream.read_varint().await?;
    validate(0, packet_id, "packet ID")?;
    let version = stream.read_varint().await?;
    let original_host = stream.read_varstring().await?;
    let host = un_fml_address(&original_host);
    let port = stream.read_u16().await?;
    let address = format!("{}:{}", host, port);
    let next_state = stream.read_varint().await?;
    let context = Context {
        host: host.clone(),
        port: port,
        address: address,
    };
    let mut proxy = chain::connect(context, chain).await?;
    proxy.write_varint(length).await?;
    proxy.write_varint(packet_id).await?;
    proxy.write_varint(version).await?;
    proxy.write_varstring(&original_host).await?;
    proxy.write_u16(port).await?;
    proxy.write_varint(next_state).await?;
    tokio::io::copy_bidirectional(&mut stream, &mut proxy).await?;
    Ok(())
}

fn un_fml_address(address: &str) -> String {
    let null = address.find('\0');
    match null {
        Some(index) => address[..index].to_owned(),
        None => address.to_owned(),
    }
}

fn validate<T>(expect: T, actual: T, field_name: &str) -> anyhow::Result<()>
where
    T: Eq + Display,
{
    if expect == actual {
        Ok(())
    } else {
        let message = format!(
            "wrong Minecraft protocol field \"{}\" value ({} expected, got {})",
            field_name, expect, actual
        );
        log::debug!("{}", &message);
        Err(anyhow::Error::msg(message))
    }
}
