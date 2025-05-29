use std::fmt;

#[derive(Debug)]
pub enum NoiseErrorKind {
    Handshake(demand_sv2_connection::Error),
    Noise(noise_sv2::Error),
}

#[derive(Debug)]
pub enum Error {
    BinarySv2(binary_sv2::Error),
    /// Errors on bad noise handshake.
    SV2Connection(demand_sv2_connection::Error),
    /// Errors from `framing_sv2` crate.
    FramingSv2(framing_sv2::Error),
    /// Errors on bad `TcpStream` connection.
    Io(std::io::Error),
    /// Errors on bad `String` to `int` conversion.
    RolesSv2Logic(roles_logic_sv2::errors::Error),
    Timeout,
    Unrecoverable,
    UnexpectedMessage,
    /// Errors from Noise protocol operations.
    Noise(NoiseErrorKind),
    /// Errors when sending messages through channels.
    Send(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        match self {
            BinarySv2(e) => write!(f, "Binary SV2 error: `{:?}`", e),
            SV2Connection(e) => write!(f, "Demand SV2 connectiom  error: `{:?}", e),
            FramingSv2(e) => write!(f, "Framing SV2 error: `{:?}`", e),
            Io(e) => write!(f, "I/O error: `{:?}", e),
            RolesSv2Logic(e) => write!(f, "Roles SV2 Logic Error: `{:?}`", e),
            Unrecoverable => write!(f, "Unrecoverable error"),
            UnexpectedMessage => write!(f, "Unexpected Message Type"),
            Timeout => write!(f, "Timeout Elapsed"),
            Noise(e) => match e {
                NoiseErrorKind::Handshake(d) => write!(f, "Noise Handshake error: `{:?}`", d),
                NoiseErrorKind::Noise(s) => write!(f, "Noise Protocol error: `{:?}`", s),
            },
            Send(s) => write!(f, "Send error: {}", s),
        }
    }
}

impl From<binary_sv2::Error> for Error {
    fn from(e: binary_sv2::Error) -> Self {
        Error::BinarySv2(e)
    }
}

impl From<demand_sv2_connection::Error> for Error {
    fn from(e: demand_sv2_connection::Error) -> Self {
        Error::SV2Connection(e)
    }
}

impl From<framing_sv2::Error> for Error {
    fn from(e: framing_sv2::Error) -> Self {
        Error::FramingSv2(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<roles_logic_sv2::errors::Error> for Error {
    fn from(e: roles_logic_sv2::errors::Error) -> Self {
        Error::RolesSv2Logic(e)
    }
}
