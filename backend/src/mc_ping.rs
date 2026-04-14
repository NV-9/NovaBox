use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Default)]
pub struct McStatus {
    pub online_players: i64,
}

pub async fn ping(host: &str, port: u16) -> McStatus {
    match tokio::time::timeout(
        tokio::time::Duration::from_secs(5),
        do_ping(host, port),
    )
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            tracing::debug!("mc_ping {}:{} failed: {e}", host, port);
            McStatus::default()
        }
        Err(_) => {
            tracing::debug!("mc_ping {}:{} timed out", host, port);
            McStatus::default()
        }
    }
}

async fn do_ping(host: &str, port: u16) -> Result<McStatus> {
    let mut stream = TcpStream::connect((host, port)).await?;

    let mut handshake_data = Vec::new();
    write_varint(&mut handshake_data, 0x00);
    write_varint(&mut handshake_data, 0);
    write_string(&mut handshake_data, host);
    handshake_data.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake_data, 1);

    send_packet(&mut stream, &handshake_data).await?;

    let mut status_req = Vec::new();
    write_varint(&mut status_req, 0x00);
    send_packet(&mut stream, &status_req).await?;

    let _length   = read_varint(&mut stream).await?;
    let packet_id = read_varint(&mut stream).await?;
    if packet_id != 0x00 {
        return Err(anyhow!("Unexpected packet id 0x{:02x}", packet_id));
    }
    let json_len = read_varint(&mut stream).await? as usize;
    let mut json_bytes = vec![0u8; json_len];
    stream.read_exact(&mut json_bytes).await?;

    let json: serde_json::Value = serde_json::from_slice(&json_bytes)?;
    let online = json["players"]["online"].as_i64().unwrap_or(0);

    Ok(McStatus { online_players: online })
}

async fn send_packet(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    let mut buf = Vec::new();
    write_varint(&mut buf, data.len() as i32);
    buf.extend_from_slice(data);
    stream.write_all(&buf).await?;
    Ok(())
}

fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        if value & !0x7F == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push((value & 0x7F | 0x80) as u8);
        value = ((value as u32) >> 7) as i32;
    }
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_varint(buf, bytes.len() as i32);
    buf.extend_from_slice(bytes);
}

async fn read_varint(stream: &mut TcpStream) -> Result<i32> {
    let mut result = 0i32;
    let mut shift  = 0u32;
    loop {
        let mut b = [0u8; 1];
        stream.read_exact(&mut b).await?;
        result |= ((b[0] & 0x7F) as i32) << shift;
        if b[0] & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 35 {
            return Err(anyhow!("VarInt too wide"));
        }
    }
}
