use super::device_service::DeviceService;
use super::remote_request::{RemoteRequest, read_result, response};
use crate::devices::identity::device_id;
use crate::devices::{discovery, tailscale, transport};
use serde_json::{Value, json};
use std::io;
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

impl DeviceService {
    pub(in crate::devices) fn call(&self, host: &str, request: RemoteRequest) -> io::Result<Value> {
        if matches!(
            request,
            RemoteRequest::Pair { .. } | RemoteRequest::RequestAccess { .. }
        ) {
            return Err(io::Error::other("Use an invitation to pair"));
        }
        let peer = self.store.lock().unwrap().peer(host)?;
        let mut connection = transport::connect(
            peer.device.address,
            peer.device.certificate,
            Some(&self.identity),
        )?;
        transport::send(&mut connection, &request)?;
        read_result(&mut connection)
    }

    pub(super) fn listen(self: &Arc<Self>, address: SocketAddr) -> io::Result<()> {
        validate_address(address)?;
        let mut listening = self.listening.lock().unwrap();
        if let Some(current) = *listening {
            if current == address {
                return Ok(());
            }
            return Err(io::Error::other(
                "Restart Agent before changing its listening address",
            ));
        }
        let listener = TcpListener::bind(address)?;
        *listening = Some(listener.local_addr()?);
        let service = self.clone();
        let active_count = Arc::new(AtomicUsize::new(0));
        std::thread::Builder::new()
            .name("device-listener".into())
            .spawn(move || {
                for socket in listener.incoming().flatten() {
                    if active_count.fetch_add(1, Ordering::Relaxed) >= 8 {
                        active_count.fetch_sub(1, Ordering::Relaxed);
                        continue;
                    }
                    let service = service.clone();
                    let active_count = active_count.clone();
                    std::thread::spawn(move || {
                        let _ = service.serve(socket);
                        active_count.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            })?;
        Ok(())
    }

    fn serve(&self, mut socket: std::net::TcpStream) -> io::Result<()> {
        socket.set_read_timeout(Some(Duration::from_secs(10)))?;
        socket.set_write_timeout(Some(Duration::from_secs(10)))?;
        let mut first_byte = [0];
        socket.peek(&mut first_byte)?;
        if first_byte[0] == b'A' {
            return discovery::answer(&mut socket, &self.description()?);
        }
        let source_address = socket.peer_addr()?;
        let certificate_list = self
            .store
            .lock()
            .unwrap()
            .peer_list
            .iter()
            .filter(|peer| peer.may_control_us)
            .map(|peer| peer.device.certificate.clone())
            .collect();
        let config = transport::server_config(&self.identity, certificate_list)?;
        let connection = rustls::ServerConnection::new(config).map_err(io::Error::other)?;
        let mut stream = rustls::StreamOwned::new(connection, socket);
        let request: RemoteRequest = transport::receive_limited(&mut stream, 8 * 1024 * 1024)?;
        let peer_id = stream
            .conn
            .peer_certificates()
            .and_then(|list| list.first())
            .map(|certificate| device_id(certificate));
        let result = match request {
            RemoteRequest::RequestAccess { device } => (|| {
                if source_address.ip() != device.address.ip() {
                    return Err(io::Error::other(
                        "Pairing source does not match device address",
                    ));
                }
                tailscale::identify(source_address)?;
                let device = self.verify_device(device)?;
                self.accept_request(device)
            })(),
            RemoteRequest::Pair { ref device, .. }
                if source_address.ip() != device.address.ip() =>
            {
                Err(io::Error::other(
                    "Pairing source does not match device address",
                ))
            }
            request => self.dispatch(peer_id.as_deref(), request),
        };
        transport::send(&mut stream, &response(result))
    }

    pub(super) fn dispatch(
        &self,
        peer_id: Option<&str>,
        request: RemoteRequest,
    ) -> io::Result<Value> {
        if let RemoteRequest::Pair { secret, device } = request {
            validate_address(device.address)?;
            if device.name.is_empty() || device.name.len() > 100 || device.certificate.len() > 16384
            {
                return Err(io::Error::other("Invalid device identity"));
            }
            let mut roots = rustls::RootCertStore::empty();
            roots
                .add(rustls::pki_types::CertificateDer::from(
                    device.certificate.clone(),
                ))
                .map_err(|_| io::Error::other("Invalid device certificate"))?;
            let mut invitation_by_secret = self.invitation_by_secret.lock().unwrap();
            let invitation = invitation_by_secret
                .get_mut(&secret)
                .filter(|invitation| invitation.created.elapsed() < Duration::from_secs(600))
                .ok_or_else(|| io::Error::other("Invitation is invalid or expired"))?;
            let id = device.id();
            if invitation
                .claimed_by
                .as_ref()
                .is_some_and(|claimed| claimed != &id)
            {
                return Err(io::Error::other("Invitation already used"));
            }
            invitation.claimed_by = Some(id.clone());
            drop(invitation_by_secret);
            let device = self.verify_device(device)?;
            return self.accept_request(device);
        }
        if matches!(request, RemoteRequest::Describe) {
            return Ok(serde_json::to_value(self.description()?)?);
        }
        if let RemoteRequest::PairingStatus { id } = request {
            return Ok(json!({"status": self.pairing_status(&id)}));
        }

        let store = self.store.lock().unwrap();
        let peer_id = peer_id.ok_or_else(|| {
            io::Error::new(io::ErrorKind::PermissionDenied, "Device is not authorized")
        })?;
        if !store
            .peer_list
            .iter()
            .any(|peer| peer.may_control_us && peer.device.id() == peer_id)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Device is not authorized",
            ));
        }
        if let RemoteRequest::Service { request } = request {
            let peer = store
                .peer_list
                .iter()
                .find(|peer| peer.device.id() == peer_id)
                .unwrap();
            let sender = crate::protocol::Target {
                id: peer_id.into(),
                name: peer.device.name.clone(),
            };
            let handler = self
                .service_handler
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| {
                    io::Error::other("This device does not host the central service.")
                })?;
            // This authenticated request is admitted. Revocation rejects subsequent requests.
            drop(store);
            return handler(sender, request);
        }

        // Hold authorization through dispatch so revocation cannot race a new submission.
        match request {
            RemoteRequest::Service { .. } => unreachable!("service requests handled above"),
            RemoteRequest::Info => Ok(
                json!({"id": device_id(&self.identity.certificate), "name": store.name,
                "tool_list": self.tool_list(), "unavailable_tools": self.unavailable_tools()}),
            ),
            RemoteRequest::Submit { .. }
            | RemoteRequest::Status { .. }
            | RemoteRequest::Cancel { .. }
                if !self.execution_device =>
            {
                Err(io::Error::other(
                    "The central service is not an execution target.",
                ))
            }
            RemoteRequest::Submit { id, tool } => Ok(serde_json::to_value(
                self.tools.submit(peer_id, &id, tool)?,
            )?),
            RemoteRequest::Status { id } => {
                Ok(serde_json::to_value(self.tools.get(peer_id, &id)?)?)
            }
            RemoteRequest::Cancel { id } => {
                Ok(serde_json::to_value(self.tools.cancel(peer_id, &id)?)?)
            }
            RemoteRequest::Pair { .. }
            | RemoteRequest::Describe
            | RemoteRequest::PairingStatus { .. } => unreachable!(),
            RemoteRequest::RequestAccess { .. } => Err(io::Error::other(
                "Pairing requires a verified Tailscale connection",
            )),
        }
    }
}

pub(super) fn validate_address(address: SocketAddr) -> io::Result<()> {
    let allowed = match address.ip() {
        std::net::IpAddr::V4(ip) => {
            ip.is_loopback() || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        }
        std::net::IpAddr::V6(ip) => ip.is_loopback(),
    };
    if !allowed || address.port() == 0 {
        return Err(io::Error::other(
            "Use a Tailscale IPv4 address and a nonzero port",
        ));
    }
    Ok(())
}
