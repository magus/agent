use super::identity::Identity;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig, StreamOwned};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

pub(super) fn server_config(
    identity: &Identity,
    certificate_list: Vec<Vec<u8>>,
) -> io::Result<Arc<ServerConfig>> {
    let mut roots = RootCertStore::empty();
    roots
        .add(identity.certificate())
        .map_err(io::Error::other)?;
    for certificate in certificate_list {
        roots
            .add(CertificateDer::from(certificate))
            .map_err(io::Error::other)?;
    }

    // Anonymous TLS is only for invitation requests. Tool routes require an approved certificate.
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        Arc::new(rustls::crypto::ring::default_provider()),
    )
    .allow_unauthenticated()
    .build()
    .map_err(io::Error::other)?;
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(io::Error::other)?
            .with_client_cert_verifier(verifier)
            .with_single_cert(vec![identity.certificate()], identity.key())
            .map_err(io::Error::other)?;
    Ok(Arc::new(config))
}

pub(super) fn connect(
    address: SocketAddr,
    certificate: Vec<u8>,
    identity: Option<&Identity>,
) -> io::Result<StreamOwned<ClientConnection, TcpStream>> {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(certificate))
        .map_err(io::Error::other)?;
    let builder =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(io::Error::other)?
            .with_root_certificates(roots);
    let config = match identity {
        Some(identity) => builder
            .with_client_auth_cert(vec![identity.certificate()], identity.key())
            .map_err(io::Error::other)?,
        None => builder.with_no_client_auth(),
    };
    let connection = ClientConnection::new(
        Arc::new(config),
        ServerName::try_from("agent.local").unwrap(),
    )
    .map_err(io::Error::other)?;
    let socket = TcpStream::connect_timeout(&address, Duration::from_secs(10))?;
    socket.set_read_timeout(Some(Duration::from_secs(30)))?;
    socket.set_write_timeout(Some(Duration::from_secs(30)))?;
    Ok(StreamOwned::new(connection, socket))
}

pub(super) fn send<T: Serialize>(stream: &mut impl Write, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(io::Error::other("Device message exceeds 64 MiB"));
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()
}

pub(super) fn receive<T: DeserializeOwned>(stream: &mut impl Read) -> io::Result<T> {
    receive_limited(stream, 64 * 1024 * 1024)
}

pub(super) fn receive_limited<T: DeserializeOwned>(
    stream: &mut impl Read,
    limit: usize,
) -> io::Result<T> {
    let mut size = [0; 4];
    stream.read_exact(&mut size)?;
    let size = u32::from_be_bytes(size) as usize;
    if size > limit {
        return Err(io::Error::other("Device message exceeds the size limit"));
    }
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}
