use std::future::Future;
use std::io::Result;
use std::net::SocketAddr;

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub mod client;
pub mod request;
pub mod response;
pub mod server;

pub trait Streamable: Sized + Send + Sync {
    fn read<T>(stream: &mut T) -> impl Future<Output = Result<Self>> + Send
    where
        T: AsyncReadExt + Unpin + Send + 'static;

    fn write<T>(self, stream: &mut T) -> impl Future<Output = Result<()>> + Send
    where
        T: AsyncWriteExt + Unpin + Send + 'static;
}

pub trait ToBytes {
    fn to_bytes(self) -> Bytes;
}

pub trait Provider<T> {
    fn fetch(&mut self) -> impl Future<Output = Option<T>> + Send;
}

pub trait Resolver: Send + Sync {
    fn lookup(&self, domain: &str, port: u16) -> impl Future<Output = Result<SocketAddr>> + Send;
}
