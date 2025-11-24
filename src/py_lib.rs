#![cfg(feature = "python")]

use crate::command_sender::FirmCommand;
use crate::parser::{FIRMPacket, SerialParser};
use pyo3::prelude::*;

/// Python-facing wrapper around the FIRM serial parser.
///
/// Exposed to Python as the `PyFIRMParser` class.
#[pyclass]
pub struct PyFIRMParser {
    /// Internal Rust streaming parser.
    inner: SerialParser,
}

#[pymethods]
impl PyFIRMParser {
    /// Creates a new `PyFIRMParser` instance for use in Python.
    ///
    /// Returns
    /// -------
    /// PyFIRMParser
    ///     A parser with an empty internal buffer and no queued packets.
    #[new]
    fn new() -> Self {
        PyFIRMParser {
            inner: SerialParser::new(),
        }
    }

    /// Feeds raw bytes into the parser.
    ///
    /// Parameters
    /// ----------
    /// data : bytes
    ///     Raw bytes from the FIRM serial stream.
    fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

    /// Returns the next parsed packet, if one is available.
    ///
    /// Returns
    /// -------
    /// FIRMPacket or None
    ///     The next parsed packet, or ``None`` if no packets are queued.
    fn get_packet(&mut self) -> Option<FIRMPacket> {
        self.inner.get_packet()
    }
}

/// Helper class to construct FIRM commands.
#[pyclass]
pub struct FirmCommandBuilder;

#[pymethods]
impl FirmCommandBuilder {
    /// Creates a Ping command.
    ///
    /// Returns
    /// -------
    /// bytes
    ///     The serialized command bytes.
    #[staticmethod]
    fn ping() -> Vec<u8> {
        FirmCommand::Ping.to_bytes()
    }

    /// Creates a Reset command.
    ///
    /// Returns
    /// -------
    /// bytes
    ///     The serialized command bytes.
    #[staticmethod]
    fn reset() -> Vec<u8> {
        FirmCommand::Reset.to_bytes()
    }

    /// Creates a SetRate command.
    ///
    /// Parameters
    /// ----------
    /// rate_hz : int
    ///     The desired rate in Hertz.
    ///
    /// Returns
    /// -------
    /// bytes
    ///     The serialized command bytes.
    #[staticmethod]
    fn set_rate(rate_hz: u32) -> Vec<u8> {
        FirmCommand::SetRate(rate_hz).to_bytes()
    }
}

/// Python module entry point for `firm_client`.
///
/// Registers the `PyFIRMParser` and `FIRMPacket` classes with the module.
#[pymodule]
fn _firm_client(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFIRMParser>()?;
    m.add_class::<FIRMPacket>()?;
    m.add_class::<FirmCommandBuilder>()?;
    Ok(())
}
