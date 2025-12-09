use pyo3::prelude::*;
use firm_rust::FirmClient as RustFirmClient;
use firm_core::parser::FIRMPacket;

#[pyclass(unsendable)]
struct FirmClient {
    inner: RustFirmClient,
}

#[pymethods]
impl FirmClient {
    #[new]
    #[pyo3(signature = (port_name, baud_rate=115200))]
    fn new(port_name: &str, baud_rate: u32) -> Self {
        let client = RustFirmClient::new(port_name, baud_rate);
        FirmClient { inner: client }
    }

    fn start(&mut self) -> PyResult<()> {
        self.inner.start()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
    }

    fn stop(&mut self) {
        self.inner.stop();
    }

    fn get_packets(&self) -> PyResult<Vec<FIRMPacket>> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }
        Ok(self.inner.get_packets())
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    fn __enter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Self>> {
        slf.borrow_mut().start()?;
        Ok(slf)
    }

    fn __exit__(
        slf: Bound<'_, Self>,
        _exc_type: Option<Bound<'_, PyAny>>,
        _exc_value: Option<Bound<'_, PyAny>>,
        _traceback: Option<Bound<'_, PyAny>>,
    ) {
        slf.borrow_mut().stop();
    }
}

#[pymodule(gil_used = false)]
fn firm_client(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FirmClient>()?;
    m.add_class::<FIRMPacket>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = FirmClient::new("test_port", 115200);
        assert!(!client.is_running());
    }
}
