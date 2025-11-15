#![cfg(feature = "python")]

use crate::parser::{FIRMPacket, SerialParser};
use pyo3::prelude::*;

/// Python-facing wrapper around the FIRM serial parser.
#[pyclass]
pub struct PyFirmParser {
    inner: SerialParser,
}

#[pymethods]
impl PyFirmParser {
    /// Create a new `PyFirmParser`.
    #[new]
    fn new() -> Self {
        PyFirmParser {
            inner: SerialParser::new(),
        }
    }

    /// Feed raw bytes (from Python `bytes`) into the parser.
    fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

    /// Return the next parsed packet as a Python object, or `None`.
    ///
    /// This converts the Rust `FIRMPacket` (which is annotated with
    /// `#[pyo3::pyclass]` when the `python` feature is enabled) into a
    /// Python object using the current GIL.
    fn get_packet(&mut self, py: Python<'_>) -> Option<PyObject> {
        match self.inner.get_packet() {
            Some(pkt) => Py::new(py, pkt).ok().map(|p| p.into_py(py)),
            None => None,
        }
    }
}

/// Python module entry point for `firm_client`.
#[pymodule]
fn firm_client(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFirmParser>()?;
    // Expose FIRMPacket type to Python as well so callers can inspect/construct if needed.
    m.add_class::<FIRMPacket>()?;
    Ok(())
}
