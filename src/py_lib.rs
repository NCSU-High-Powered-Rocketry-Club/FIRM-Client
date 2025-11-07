#[cfg(feature = "python")]
use crate::parser::SerialParser;
#[cfg(feature = "python")]
use crate::parser::FIRMPacket;
#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pyclass]
pub struct PyFirmParser {
    inner: SerialParser,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyFirmParser {
    /// Creates a new FIRM parser for Python.
    /// 
    /// # Arguments
    /// 
    /// - *None* - Initializes an empty internal parser.
    /// 
    /// # Returns
    /// 
    /// - `PyFirmParser` - A new parser ready to accept bytes.
    #[new]
    fn new() -> Self {
        PyFirmParser {
            inner: SerialParser::new(),
        }
    }

    /// Feeds raw bytes from Python into the parser.
    /// 
    /// # Arguments
    /// 
    /// - `data` (`&[u8]`) - Raw bytes from the FIRM serial stream.
    /// 
    /// # Returns
    /// 
    /// - `()` - Parsed packets are stored internally for `get_packet`.
    fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

    /// Returns the next parsed packet as a Python object or `None`.
    /// 
    /// # Arguments
    /// 
    /// - *None* - Reads from the internal packet queue.
    /// 
    /// # Returns
    /// 
    /// - `Option<FIRMPacket>` - The next packet or `None` if none are available.
    fn get_packet(&mut self) -> Option<FIRMPacket> {
        self.inner.get_packet()
    }
}

/// Python module entry point for `firm_client`.
/// 
/// # Arguments
/// 
/// - `_py` (`Python`) - Python interpreter handle.
/// - `m` (`&PyModule`) - Module to register classes and functions on.
/// 
/// # Returns
/// 
/// - `PyResult<()>` - Ok if module initialization succeeded.
#[cfg(feature = "python")]
#[pymodule]
fn firm_client(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyFirmParser>()?;
    Ok(())
}
