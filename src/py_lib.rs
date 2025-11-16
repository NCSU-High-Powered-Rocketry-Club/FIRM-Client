#![cfg(feature = "python")]

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

/// Python module entry point for `firm_client`.
///
/// Registers the `PyFIRMParser` and `FIRMPacket` classes with the module.
#[pymodule]
fn firm_client(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFIRMParser>()?;
    m.add_class::<FIRMPacket>()?;
    Ok(())
}
