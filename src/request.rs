use std::io::{Error, Result};
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};

use bytes::{BufMut, Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{Resolver, Streamable, ToBytes};

// ===== Request =====
#[rustfmt::skip]
mod consts_request_type {
    pub const REQUEST_TYPE_TCP_CONNECT: u8 = 0x01;
}

/// ## Bytes
/// ```text
///          +------+------+----------+------+
///          | RTYP | ATYP |   ADDR   | PORT |
///          +------+------+----------+------+
///          |  1   |  1   | Variable |  2   |
///          +------+------+----------+------+
/// ```
///
#[derive(Debug, Clone)]
pub enum Request {
    TCPConnect(Address),
}

impl ToBytes for Request {
    fn to_bytes(self) -> Bytes {
        let mut bytes = BytesMut::new();

        match self {
            Self::TCPConnect(value) => {
                bytes.put_u8(consts_request_type::REQUEST_TYPE_TCP_CONNECT);
                bytes.extend(value.to_bytes());
            }
        };

        bytes.freeze()
    }
}

impl Streamable for Request {
    async fn write<T>(self, stream: &mut T) -> Result<()>
    where
        T: AsyncWriteExt + Unpin + Send + 'static,
    {
        stream.write_all(&self.to_bytes()).await
    }

    async fn read<T>(stream: &mut T) -> Result<Self>
    where
        T: AsyncReadExt + Unpin + Send + 'static,
    {
        let request_type = stream.read_u8().await?;

        let request = match request_type {
            consts_request_type::REQUEST_TYPE_TCP_CONNECT => {
                Request::TCPConnect(Address::read(stream).await?)
            }

            _ => {
                return Err(Error::other(format!(
                    "unsupported request type {}",
                    request_type
                )))
            }
        };

        Ok(request)
    }
}

// ===== Address =====
#[rustfmt::skip]
mod consts_address_type {
    pub const ADDRESS_TYPE_DOMAIN: u8 = 0x01;
    pub const ADDRESS_TYPE_IPV4:   u8 = 0x02;
    pub const ADDRESS_TYPE_IPV6:   u8 = 0x03;
}

/// ## Bytes
/// ```text
///          +------+----------+------+
///          | ATYP |   ADDR   | PORT |
///          +------+----------+------+
///          |  1   | Variable |  2   |
///          +------+----------+------+
/// ```
///
#[derive(Debug, Clone)]
pub enum Address {
    Domain(String, u16),
    IPv4(SocketAddrV4),
    IPv6(SocketAddrV6),
}

impl Address {
    pub async fn to_socket_address<R>(self, resolver: &R) -> Result<SocketAddr>
    where
        R: Resolver,
    {
        let socket_address = match self {
            Self::Domain(domain, port) => resolver.lookup(&domain, port).await?.into(),
            Self::IPv4(addr) => addr.into(),
            Self::IPv6(addr) => addr.into(),
        };

        Ok(socket_address)
    }
}

impl ToBytes for Address {
    fn to_bytes(self) -> Bytes {
        let mut bytes = BytesMut::new();

        match self {
            Self::Domain(domain, port) => {
                let domain_bytes = domain.as_bytes();
                bytes.put_u8(consts_address_type::ADDRESS_TYPE_DOMAIN);
                bytes.put_u8(domain_bytes.len() as u8);
                bytes.extend_from_slice(domain_bytes);
                bytes.extend_from_slice(&port.to_be_bytes());
            }
            Self::IPv4(addr) => {
                bytes.put_u8(consts_address_type::ADDRESS_TYPE_IPV4);
                bytes.extend_from_slice(&addr.ip().octets());
                bytes.extend_from_slice(&addr.port().to_be_bytes());
            }
            Self::IPv6(addr) => {
                bytes.put_u8(consts_address_type::ADDRESS_TYPE_IPV6);
                bytes.extend_from_slice(&addr.ip().octets());
                bytes.extend_from_slice(&addr.port().to_be_bytes());
            }
        }

        bytes.freeze()
    }
}

impl Streamable for Address {
    async fn write<T>(self, stream: &mut T) -> Result<()>
    where
        T: AsyncWriteExt + Unpin + Send + 'static,
    {
        stream.write_all(&self.to_bytes()).await
    }

    async fn read<T>(stream: &mut T) -> Result<Self>
    where
        T: AsyncReadExt + Unpin + Send,
    {
        use std::net::{Ipv4Addr, Ipv6Addr};

        let address_type = stream.read_u8().await?;

        let result = match address_type {
            consts_address_type::ADDRESS_TYPE_DOMAIN => {
                let domain_len = stream.read_u8().await? as usize;

                let mut buffer = vec![0u8; domain_len + 2];
                stream.read_exact(&mut buffer).await?;

                let domain = std::str::from_utf8(&buffer[0..domain_len])
                    .map_err(|_| Error::other("invalid domain name"))?;

                let port = ((buffer[domain_len] as u16) << 8) | (buffer[domain_len + 1] as u16);

                Address::Domain(domain.to_string(), port)
            }

            consts_address_type::ADDRESS_TYPE_IPV4 => {
                let mut buffer = [0u8; 4 + 2];
                stream.read_exact(&mut buffer).await?;

                let ip = Ipv4Addr::new(buffer[0], buffer[1], buffer[2], buffer[3]);
                let port = ((buffer[4] as u16) << 8) | (buffer[5] as u16);

                Address::IPv4(SocketAddrV4::new(ip, port))
            }

            consts_address_type::ADDRESS_TYPE_IPV6 => {
                let mut buffer = [0u8; 16 + 2];
                stream.read_exact(&mut buffer).await?;

                let ip = Ipv6Addr::new(
                    (buffer[0] as u16) << 8 | buffer[1] as u16,
                    (buffer[2] as u16) << 8 | buffer[3] as u16,
                    (buffer[4] as u16) << 8 | buffer[5] as u16,
                    (buffer[6] as u16) << 8 | buffer[7] as u16,
                    (buffer[8] as u16) << 8 | buffer[9] as u16,
                    (buffer[10] as u16) << 8 | buffer[11] as u16,
                    (buffer[12] as u16) << 8 | buffer[13] as u16,
                    (buffer[14] as u16) << 8 | buffer[15] as u16,
                );
                let port = ((buffer[16] as u16) << 8) | (buffer[17] as u16);

                Address::IPv6(SocketAddrV6::new(ip, port, 0, 0))
            }

            _ => {
                return Err(Error::other(format!(
                    "unsupported request address type {}",
                    address_type
                )))
            }
        };

        Ok(result)
    }
}
