struct TlsMessage {
    content_type: u8,
    version: u16,
    length: u16,
    content: HandshakeMessage,
}

struct HandshakeMessage {
    handshake_type: HandshakeType,
    length: u32,
    version: u16,
    random: [u8; 32],
}

enum HandshakeType {
    ClientHello = 1,
}

const CONTENT_TYPE_HANDSHAKE: u8 = 22;

/*
async fn handle_tls_connection(stream: TcpStream) -> Result<(), std::io::Error> {
    let mut reader = BufReader::new(stream);
    let mut content_type: [u8; 1];
    reader.read_exact(&mut content_type).await?;
    assert!(content_type[0] == CONTENT_TYPE_HANDSHAKE);
}

async fn read_u8(reader: &mut BufReader<TcpStream>) -> Result<u8, std::io::Error> {
    let mut bytes: [u8; 1] = [0];
    reader.read_exact(&mut bytes).await?;
    Ok(bytes[0])
}

async fn read_u16(reader: &mut BufReader<TcpStream>) -> Result<u16, std::io::Error> {
    let mut bytes: [u8; 2] = [0, 0];
    reader.read_exact(&mut bytes).await?;
    Ok(u16::from_be_bytes(bytes))
}

async fn read_u24(reader: &mut BufReader<TcpStream>) -> Result<u32, std::io::Error> {
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    reader.read_exact(&mut bytes[1..]).await?;
    Ok(u32::from_be_bytes(bytes))
}

async fn read_u32(reader: &mut BufReader<TcpStream>) -> Result<u32, std::io::Error> {
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    reader.read_exact(&mut bytes).await?;
    Ok(u32::from_be_bytes(bytes))
}

async fn write_u8(writer: &mut BufWriter<TcpStream>, value: u8) -> Result<(), std::io::Error> {
    let bytes = value.to_be_bytes();
    writer.write_all(&bytes).await
}

async fn write_u16(writer: &mut BufWriter<TcpStream>, value: u16) -> Result<(), std::io::Error> {
    let bytes = value.to_be_bytes();
    writer.write_all(&bytes).await
}

async fn write_u24(writer: &mut BufWriter<TcpStream>, value: u32) -> Result<(), std::io::Error> {
    let bytes = value.to_be_bytes();
    writer.write_all(&bytes[1..]).await
}

async fn write_u32(writer: &mut BufWriter<TcpStream>, value: u32) -> Result<(), std::io::Error> {
    let bytes = value.to_be_bytes();
    writer.write_all(&bytes).await
}
*/
