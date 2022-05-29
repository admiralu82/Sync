use bytes::BufMut;
use lzzzz::lz4;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::types::BUFFER_SIZE;



pub async fn write_data(socket: &mut TcpStream, buf: &[u8]) -> Result<(), std::io::Error> {
    let mut out = Vec::with_capacity(buf.len());
    lz4::compress_to_vec(buf, &mut out, 1)?;

    socket.write_u32(buf.len() as u32).await?;
    socket.write_u32(out.len() as u32).await?;

    let buf_out = out.as_mut_slice();
    encode(buf_out);

    socket.write_all(buf_out).await?;
    socket.write_u8(crate::types::DEFAULT_OK).await?;
    
    Ok(())
}
pub async fn read_data(socket: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    let size_orig = socket.read_u32().await? as usize;
    let size_pack = socket.read_u32().await? as usize;

    let mut pos = 0;
    let mut out = Vec::<u8>::with_capacity(size_pack);
    let buf = &mut [0u8; BUFFER_SIZE];

    loop {
        let read;
        if pos + BUFFER_SIZE < size_pack {
            read = BUFFER_SIZE;
        } else {
            read = size_pack - pos;
        }
        socket.read_exact(&mut buf[..read]).await?;
        out.put_slice(&buf[..read]);

        pos += read;
        if pos == size_pack {
            break;
        }
    }

    if socket.read_u8().await? != crate::types::DEFAULT_OK {
        log::error!("Error read_data CRC");
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }

    encode(out.as_mut_slice());

    // decode
    let mut dec = Vec::<u8>::with_capacity(size_orig);
    dec.put_bytes(0, size_orig);
    lz4::decompress(out.as_slice(), dec.as_mut_slice())?;
    Ok(dec)
}

pub async fn read_data_in_string(socket: &mut TcpStream) -> Result<String, std::io::Error> {
    let f = read_data(socket).await?;
    let a = String::from_utf8(f);

    if let Err(err) = a {
        log::error!("Error FromUTF8_1: {}", err);
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let a = a.unwrap();
    Ok(a)
}

pub async fn write_string_max32(socket: &mut TcpStream, s: String) -> Result<(), std::io::Error> {

    if s.as_bytes().len() > 32 {
        log::error!("String max 32 bytes");
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }

    let mut s = s.clone();
    s.push_str("                                          ");
    socket.write_all(&s.as_bytes()[..32]).await?;

    Ok(())
}
pub async fn read_string_max32(socket: &mut TcpStream) -> Result<String, std::io::Error> {
    let buf_rcv: &mut [u8] = &mut [0u8; 32][..];

    socket.read_exact(buf_rcv).await?;

    let out = std::str::from_utf8(&buf_rcv);
    if let Err(err) = out {
        log::error!("Error FromUTF8_2: {}", err);
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let out = out.unwrap().trim().to_string();
    Ok(out)
}

pub async fn write_status_ok(socket: &mut TcpStream) -> Result<(), std::io::Error> {
    socket.write_u8(crate::types::DEFAULT_OK).await?;
    Ok(())
}
pub async fn write_status_err(socket: &mut TcpStream) -> Result<(), std::io::Error> {
    socket.write_u8(crate::types::DEFAULT_ERROR).await?;
    Ok(())
}
pub async fn read_status(socket: &mut TcpStream) -> Result<u8, std::io::Error> {
    let out = socket.read_u8().await?;
    if out == crate::types::DEFAULT_ERROR {
        log::error!("Error in status");
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    Ok(out)
}
pub async fn write_status(socket: &mut TcpStream, st: u8) -> Result<(), std::io::Error> {
    socket.write_u8(st).await?;
    Ok(())
}

pub async fn stream_send_file(
    socket: &mut TcpStream,
    mut file: tokio::fs::File,
) -> Result<(), std::io::Error> {
    let size = file.metadata().await?;
    let size = size.len() ;
    socket.write_u64(size).await?;

    // tokio::io::copy(&mut file, socket).await?;
    let mut pos = 0u64;
    let buf = &mut [0u8; BUFFER_SIZE];
    
    loop {
        let read;
        if pos + (BUFFER_SIZE as u64) < size {
            read = BUFFER_SIZE as u64;
        } else {
            read = size - pos;
        }
       
        let bufsize = read as usize;
        file.read_exact(&mut buf[..bufsize]).await?;

        encode(&mut buf[..bufsize]);
        
        socket.write_all(&buf[..bufsize]).await?;

        pos += bufsize as u64;
        if pos == size {
            break;
        }
    }

    socket.write_u8(crate::types::DEFAULT_OK).await?;
    Ok(())
}

fn encode(buf : &mut [u8]) {
    let key = crate::types::MAGIC_NUMBER.as_bytes();
    for i in 0..buf.len() {
        buf[i] ^= key[i%4];
    }
}

pub async fn server_recv_file(
    socket: &mut TcpStream,
    p: &std::path::PathBuf,
    s: u64,
) -> Result<(), std::io::Error> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(p)
        .await?;

    let size = socket.read_u64().await?;
    if size != s {
        log::error!(
            "Error in file length: {}<>{} for {:?}. Send old bytes....",
            size,
            s,
            &p,
        );
        // return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }

    let mut pos = 0u64;
    let buf = &mut [0u8; BUFFER_SIZE];

    loop {
        let read;
        if pos + (BUFFER_SIZE as u64) < size {
            read = BUFFER_SIZE as u64;
        } else {
            read = size - pos;
        }

        let buf_size = read as usize;
        socket.read_exact(&mut buf[..buf_size]).await?;

        encode(&mut buf[..buf_size]);

        file.write_all(&buf[..buf_size]).await?;

        pos += read;
        if pos == size {
            break;
        }
    }

    if socket.read_u8().await? != crate::types::DEFAULT_OK {
        log::error!("Error server_recv_file CRC. file={:?}",&p);
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }

    Ok(())
}

#[test]
fn test() {}
