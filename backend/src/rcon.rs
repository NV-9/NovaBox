use anyhow::{bail, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const SERVERDATA_AUTH:        i32 = 3;
const SERVERDATA_EXECCOMMAND: i32 = 2;

pub struct RconClient {
    stream: TcpStream,
    request_id: i32,
}

impl RconClient {
    pub async fn connect(host: &str, port: u16, password: &str) -> Result<Self> {
        let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let mut client = Self { stream, request_id: 1 };
        client.authenticate(password).await?;
        Ok(client)
    }

    async fn authenticate(&mut self, password: &str) -> Result<()> {
        let auth_id = self.send_packet(SERVERDATA_AUTH, password).await?;

        loop {
            let (id, _ptype, _body) = self.read_packet().await?;
            if id == -1 {
                bail!("RCON authentication failed: wrong password");
            }
            if id == auth_id {
                break;
            }
        }

        Ok(())
    }

    pub async fn command(&mut self, cmd: &str) -> Result<String> {
        let id = self.request_id;
        self.request_id += 1;
        self.send_raw_packet(id, SERVERDATA_EXECCOMMAND, cmd).await?;

        loop {
            let (resp_id, _ptype, body) = self.read_packet().await?;
            if resp_id == -1 {
                bail!("RCON command failed: authentication dropped");
            }
            if resp_id == id {
                return Ok(body);
            }
        }
    }

    async fn send_packet(&mut self, ptype: i32, body: &str) -> Result<i32> {
        let id = self.request_id;
        self.request_id += 1;
        self.send_raw_packet(id, ptype, body).await?;
        Ok(id)
    }

    async fn send_raw_packet(&mut self, id: i32, ptype: i32, body: &str) -> Result<()> {
        let body_bytes = body.as_bytes();
        let length     = 10 + body_bytes.len() as i32;
        let mut packet = Vec::with_capacity(length as usize + 4);

        packet.extend_from_slice(&length.to_le_bytes());
        packet.extend_from_slice(&id.to_le_bytes());
        packet.extend_from_slice(&ptype.to_le_bytes());
        packet.extend_from_slice(body_bytes);
        packet.push(0);
        packet.push(0);

        self.stream.write_all(&packet).await?;
        Ok(())
    }

    async fn read_packet(&mut self) -> Result<(i32, i32, String)> {
        let length   = self.stream.read_i32_le().await?;
        let id       = self.stream.read_i32_le().await?;
        let ptype    = self.stream.read_i32_le().await?;
        let body_len = (length - 10).max(0) as usize;
        let mut body_bytes = vec![0u8; body_len];
        if body_len > 0 {
            self.stream.read_exact(&mut body_bytes).await?;
        }
        self.stream.read_u8().await?;
        self.stream.read_u8().await?;

        let body = String::from_utf8_lossy(&body_bytes).to_string();
        Ok((id, ptype, body))
    }
}
