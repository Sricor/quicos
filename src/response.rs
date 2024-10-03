use std::io::Result;

use bytes::{BufMut, Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{Streamable, ToBytes};

#[rustfmt::skip]
mod consts_response_type {
    pub const SUCCEED: u8 = 0x01;
    pub const NO_ACCEPTABLE_REQUEST: u8 = 0xFF;
}

pub enum Response {
    Succeed,
    NoAcceptableMethod,
}

impl ToBytes for Response {
    fn to_bytes(self) -> Bytes {
        let mut bytes = BytesMut::new();

        match self {
            Self::Succeed => {
                bytes.put_u8(consts_response_type::SUCCEED);
            }
            Self::NoAcceptableMethod => bytes.put_u8(consts_response_type::NO_ACCEPTABLE_REQUEST),
        };

        bytes.freeze()
    }
}

impl Streamable for Response {
    async fn write<T>(self, stream: &mut T) -> Result<()>
    where
        T: AsyncWriteExt + Unpin + Send,
    {
        stream.write_all(&self.to_bytes()).await
    }

    async fn read<T>(stream: &mut T) -> Result<Self>
    where
        T: AsyncReadExt + Unpin + Send,
    {
        let response_type = stream.read_u8().await?;

        let response = match response_type {
            consts_response_type::SUCCEED => Self::Succeed,
            _ => Self::NoAcceptableMethod,
        };

        Ok(response)
    }
}
