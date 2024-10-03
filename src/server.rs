use std::{marker::PhantomData, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};

use crate::{Provider, Resolver};

pub struct ServerHandlerContextBuilder<RE> {
    resolver: Option<RE>,
}

impl<RE> ServerHandlerContextBuilder<RE> {
    pub fn new() -> Self {
        Self { resolver: None }
    }

    pub fn with_resolver(mut self, resolver: RE) -> Self {
        self.resolver = Some(resolver);
        self
    }

    pub fn build(self) -> ServerHandlerContext<RE> {
        ServerHandlerContext {
            resolver: self.resolver.expect("Resolver must be provided"),
        }
    }
}

pub struct ServerHandlerContext<RE> {
    resolver: RE,
}

pub struct ServerBuilder<R, RE, RS> {
    accept: Option<R>,
    context: Option<Arc<ServerHandlerContext<RE>>>,
    _accept_stream: PhantomData<RS>,
}

impl<R, RE, RS> ServerBuilder<R, RE, RS>
where
    R: Provider<RS>,
    RE: Resolver + 'static,
    RS: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    pub fn new() -> Self {
        Self {
            accept: None,
            context: None,
            _accept_stream: PhantomData,
        }
    }

    pub fn with_accept(mut self, accept: R) -> Self {
        self.accept = Some(accept);
        self
    }

    pub fn with_context(mut self, context: ServerHandlerContext<RE>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn build(self) -> Server<R, RE, RS> {
        Server {
            accept: self.accept.expect("Accept must be provided"),
            context: self.context.expect("Context must be provided"),
            _accept_stream: PhantomData,
        }
    }
}

pub struct Server<R, RE, RS> {
    accept: R,
    context: Arc<ServerHandlerContext<RE>>,
    _accept_stream: PhantomData<RS>,
}

impl<R, RS, RE> Server<R, RE, RS>
where
    R: Provider<RS>,
    RE: Resolver + 'static,
    RS: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    pub async fn start(&mut self) {
        while let Some(stream) = self.accept.fetch().await {
            let context = self.context.clone();
            tokio::spawn(async move { Self::handler(stream, &context).await });
        }
    }

    async fn handler(mut stream: RS, context: &ServerHandlerContext<RE>) -> Result<()> {
        use tokio::io::copy_bidirectional;
        use tokio::net::TcpStream;

        use crate::request::Request;
        use crate::response::Response;
        use crate::Streamable;

        let request = Request::read(&mut stream).await?;

        match request {
            Request::TCPConnect(addr) => {
                let address = addr.to_socket_address(&context.resolver).await?;
                let mut connect = TcpStream::connect(address).await?;

                Response::Succeed.write(&mut stream).await?;

                copy_bidirectional(&mut stream, &mut connect).await?
            }
        };

        Ok(())
    }
}
