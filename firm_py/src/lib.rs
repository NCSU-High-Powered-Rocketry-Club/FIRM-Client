use pyo3::prelude::*;
use firm_rust::FirmClient as RustFirmClient;
use firm_core::parser::FIRMPacket;

#[pyclass(unsendable)]
struct FirmClient {
    inner: RustFirmClient,
    timeout: f64
}

#[pymethods]
impl FirmClient {
    #[new]
    #[pyo3(signature = (port_name, baud_rate=2_000_000, timeout=0.1))]
    fn new(port_name: &str, baud_rate: Option<u32>, timeout: Option<f64>) -> PyResult<Self> {
        let baudrate = baud_rate.unwrap_or(2_000_000);
        let timeout_val = timeout.unwrap_or(0.1);
        let client = RustFirmClient::new(port_name, baudrate, timeout_val)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(FirmClient { inner: client , timeout: timeout_val })
    }

    fn start(&mut self) -> PyResult<()> {
        self.inner.start();
        Ok(())
    }

    fn stop(&mut self) {
        self.inner.stop();
    }

    #[pyo3(signature = (block=false))]
    fn get_data_packets(&self, block: bool) -> PyResult<Vec<FIRMPacket>> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let timeout = if block {
            Some(std::time::Duration::from_secs_f64(self.timeout))
        } else {
            None
        };
        // Get all packets, and return early if there's an error
        let packets = self.inner.get_packets(timeout)
                        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(packets)
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
