use std::io::Result;
use std::{future::Future, sync::Arc};

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub trait Streamable: ToBytes + FromBytes + Send + Sync {
    fn write_to<T>(self, stream: &mut T) -> impl Future<Output = Result<()>> + Send
    where
        T: AsyncWriteExt + Unpin + Send + 'static,
    {
        async { stream.write_all(&self.to_bytes()).await }
    }

    fn read_from<T>(stream: &mut T) -> impl Future<Output = Result<Self>> + Send
    where
        T: AsyncReadExt + Unpin + Send + 'static,
    {
        async { Self::from_bytes(stream).await }
    }
}

pub trait ToBytes {
    fn to_bytes(self) -> Bytes;
}

pub trait FromBytes: Sized {
    fn from_bytes<T>(stream: &mut T) -> impl Future<Output = Result<Self>> + Send;
}

pub trait Provider<T> {
    fn fetch(&mut self) -> impl Future<Output = Option<T>> + Send;
}

pub trait Client<L, R, LS, RS>: Sized + Send + Sync + 'static
where
    L: Provider<LS> + Send,
    R: Provider<RS> + Send,
    LS: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
    RS: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    fn start(
        self,
        mut local_provider: L,
        mut remote_provider: R,
    ) -> impl Future<Output = ()> + Send {
        let inner = Arc::new(self);

        async move {
            while let Some(local) = local_provider.fetch().await {
                if let Some(remote) = remote_provider.fetch().await {
                    let inner = inner.clone();
                    tokio::spawn(async move { inner.handler(local, remote).await });
                }
            }
        }
    }

    fn handler(&self, local: LS, remote: RS) -> impl Future<Output = Result<()>> + Send;
}

pub trait Server<C, CS>: Sized + Send + Sync + 'static
where
    C: Provider<CS> + Send,
    CS: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    fn start(self, mut accept: C) -> impl Future<Output = ()> + Send {
        let inner = Arc::new(self);

        async move {
            while let Some(stream) = accept.fetch().await {
                let inner = inner.clone();
                tokio::spawn(async move { inner.handler(stream).await });
            }
        }
    }

    fn handler(&self, stream: CS) -> impl Future<Output = Result<()>> + Send;
}
