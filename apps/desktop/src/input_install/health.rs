use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use qinglink_protocol::{DEFAULT_PORT, HealthResponse};

const HEALTH_ADDRESS: &str = "127.0.0.1:3721";
const HEALTH_TIMEOUT: Duration = Duration::from_millis(500);
const MAX_HEALTH_RESPONSE: u64 = 16 * 1024;

pub(crate) fn service_network_ready() -> bool {
    let address: SocketAddr = match HEALTH_ADDRESS.parse() {
        Ok(value) => value,
        Err(_) => return false,
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&address, HEALTH_TIMEOUT) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(HEALTH_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HEALTH_TIMEOUT));
    if stream
        .write_all(b"GET /api/v2/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = Vec::new();
    if stream
        .take(MAX_HEALTH_RESPONSE)
        .read_to_end(&mut response)
        .is_err()
    {
        return false;
    }
    validate_response(&response)
}

fn validate_response(response: &[u8]) -> bool {
    let Ok(response) = std::str::from_utf8(response) else {
        return false;
    };
    let Some((headers, body)) = response.split_once("\r\n\r\n") else {
        return false;
    };
    let Some(status_line) = headers.lines().next() else {
        return false;
    };
    if status_line != "HTTP/1.1 200 OK" && status_line != "HTTP/1.0 200 OK" {
        return false;
    }
    let Ok(health) = serde_json::from_str::<HealthResponse>(body) else {
        return false;
    };
    health.version == 2
        && health.port == DEFAULT_PORT
        && !health.pc_id.trim().is_empty()
        && !health.name.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::validate_response;

    fn response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{body}").into_bytes()
    }

    #[test]
    fn accepts_only_qinglink_v2_health_identity() {
        assert!(validate_response(&response(
            r#"{"pc_id":"pc-1","name":"QingLink","version":2,"port":3721}"#
        )));
        assert!(!validate_response(&response(
            r#"{"pc_id":"pc-1","name":"QingLink","version":1,"port":3721}"#
        )));
        assert!(!validate_response(&response(r#"{"status":"ok"}"#)));
        assert!(!validate_response(b"HTTP/1.1 200 OK\r\n\r\nnot-json"));
    }
}
