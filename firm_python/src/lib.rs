use firm_core::firm_packets::{
    DeviceConfig, DeviceInfo, DeviceProtocol, FIRMData,
};
use firm_rust::FIRMClient as RustFirmClient;
use pyo3::prelude::*;

#[pyclass(unsendable)]
struct FIRMClient {
    inner: RustFirmClient,
    timeout: f64,
}

#[pymethods]
impl FIRMClient {
    #[new]
    #[pyo3(signature = (port_name, baud_rate=2_000_000, timeout=0.1))]
    fn new(port_name: &str, baud_rate: Option<u32>, timeout: Option<f64>) -> PyResult<Self> {
        let baudrate = baud_rate.unwrap_or(2_000_000);
        let timeout_val = timeout.unwrap_or(0.1);
        let client = RustFirmClient::new(port_name, baudrate, timeout_val)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(FIRMClient {
            inner: client,
            timeout: timeout_val,
        })
    }

    fn start(&mut self) -> PyResult<()> {
        self.inner.start();
        Ok(())
    }

    fn stop(&mut self) {
        self.inner.stop();
    }

    #[pyo3(signature = (log_path, realtime=true, speed=1.0, chunk_size=8192, start_timeout_seconds=5.0))]
    fn stream_mock_log_file(
        &mut self,
        log_path: &str,
        realtime: bool,
        speed: f64,
        chunk_size: usize,
        start_timeout_seconds: f64,
    ) -> PyResult<usize> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let sent = self
            .inner
            .stream_mock_log_file(
                log_path,
                std::time::Duration::from_secs_f64(start_timeout_seconds),
                realtime,
                speed,
                chunk_size,
            )
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

        Ok(sent)
    }

    #[pyo3(signature = (block=false))]
    fn get_data_packets(&mut self, block: bool) -> PyResult<Vec<FIRMData>> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let timeout = if block {
            Some(std::time::Duration::from_secs_f64(self.timeout))
        } else {
            None
        };
        // Get all packets, and return early if there's an error
        let packets = self
            .inner
            .get_data_packets(timeout)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(packets)
    }

    #[pyo3(signature = (timeout_seconds=5.0))]
    fn get_device_info(&mut self, timeout_seconds: f64) -> PyResult<Option<DeviceInfo>> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let info = self
            .inner
            .get_device_info(std::time::Duration::from_secs_f64(timeout_seconds))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(info)
    }

    #[pyo3(signature = (timeout_seconds=5.0))]
    fn get_device_config(&mut self, timeout_seconds: f64) -> PyResult<Option<DeviceConfig>> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let cfg = self
            .inner
            .get_device_config(std::time::Duration::from_secs_f64(timeout_seconds))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(cfg)
    }

    #[pyo3(signature = (name, frequency, protocol, timeout_seconds=5.0))]
    fn set_device_config(
        &mut self,
        name: String,
        frequency: u16,
        protocol: DeviceProtocol,
        timeout_seconds: f64,
    ) -> PyResult<bool> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let res = self
            .inner
            .set_device_config(
                name,
                frequency,
                protocol,
                std::time::Duration::from_secs_f64(timeout_seconds),
            )
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

        Ok(res.unwrap_or(false))
    }

    #[pyo3(signature = (timeout_seconds=5.0))]
    fn cancel(&mut self, timeout_seconds: f64) -> PyResult<bool> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let res = self
            .inner
            .cancel(std::time::Duration::from_secs_f64(timeout_seconds))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(res.unwrap_or(false))
    }

    fn reboot(&mut self) -> PyResult<()> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        self.inner
            .reboot()
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(())
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
    m.add_class::<FIRMClient>()?;
    m.add_class::<FIRMData>()?;
    m.add_class::<DeviceProtocol>()?;
    m.add_class::<DeviceInfo>()?;
    m.add_class::<DeviceConfig>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
