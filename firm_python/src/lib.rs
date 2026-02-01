use firm_core::constants::packet::PacketHeader;
use firm_core::firm_packets::{DeviceConfig, DeviceInfo, DeviceProtocol, FIRMData};
use firm_core::framed_packet::FramedPacket;
use firm_rust::mock_serial::MockDeviceHandle as RustMockDeviceHandle;
use firm_rust::FIRMClient as RustFirmClient;
use pyo3::prelude::*;
use std::time::Duration;

#[inline]
fn py_io_err(msg: impl ToString) -> PyErr {
    pyo3::exceptions::PyIOError::new_err(msg.to_string())
}

#[inline]
fn map_io<T>(res: Result<T, impl ToString>) -> PyResult<T> {
    res.map_err(py_io_err)
}

#[pyclass(unsendable)]
struct FIRMClient {
    inner: RustFirmClient,
    /// Used only when `get_data_packets(block=true)` is called.
    timeout: f64,
}

#[pyclass(unsendable)]
struct MockDeviceHandle {
    inner: RustMockDeviceHandle,
}

#[pymethods]
impl FIRMClient {
    #[new]
    #[pyo3(signature = (port_name, baud_rate=2_000_000, timeout=0.1))]
    fn new(port_name: &str, baud_rate: Option<u32>, timeout: Option<f64>) -> PyResult<Self> {
        let baudrate = baud_rate.unwrap_or(2_000_000);
        let timeout_val = timeout.unwrap_or(0.1);

        let client = map_io(RustFirmClient::new(port_name, baudrate, timeout_val))?;
        Ok(Self {
            inner: client,
            timeout: timeout_val,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (timeout=0.1))]
    fn new_mock(timeout: f64) -> PyResult<(Self, MockDeviceHandle)> {
        let (client, device) = RustFirmClient::new_mock(timeout);
        Ok((
            Self {
                inner: client,
                timeout,
            },
            MockDeviceHandle { inner: device },
        ))
    }

    #[inline]
    fn ensure_ok(&self) -> PyResult<()> {
        if let Some(err) = self.inner.check_error() {
            return Err(py_io_err(err));
        }
        Ok(())
    }

    fn start(&mut self) -> PyResult<()> {
        self.inner.start();
        Ok(())
    }

    fn stop(&mut self) {
        self.inner.stop();
    }

    /// Stream an entire mock log file synchronously and return the number of bytes sent.
    #[pyo3(signature = (log_path, realtime=true, speed=1.0, chunk_size=8192, start_timeout_seconds=5.0))]
    fn stream_mock_log_file(
        &mut self,
        log_path: &str,
        realtime: bool,
        speed: f64,
        chunk_size: usize,
        start_timeout_seconds: f64,
    ) -> PyResult<usize> {
        self.ensure_ok()?;

        let sent = map_io(self.inner.stream_mock_log_file(
            log_path,
            Duration::from_secs_f64(start_timeout_seconds),
            realtime,
            speed,
            chunk_size,
        ))?;

        Ok(sent)
    }

    /// Start streaming a mock log file in the background.
    #[pyo3(signature = (log_path, realtime=true, speed=1.0, chunk_size=8192, start_timeout_seconds=5.0, cancel_on_finish=true))]
    fn start_mock_log_stream(
        &mut self,
        log_path: &str,
        realtime: bool,
        speed: f64,
        chunk_size: usize,
        start_timeout_seconds: f64,
        cancel_on_finish: bool,
    ) -> PyResult<()> {
        self.ensure_ok()?;

        map_io(self.inner.start_mock_log_stream(
            log_path.to_string(),
            Duration::from_secs_f64(start_timeout_seconds),
            realtime,
            speed,
            chunk_size,
            cancel_on_finish,
        ))?;

        Ok(())
    }

    #[pyo3(signature = (cancel_device=true))]
    fn stop_mock_log_stream(&mut self, cancel_device: bool) -> PyResult<()> {
        self.ensure_ok()?;
        map_io(self.inner.stop_mock_log_stream(cancel_device))?;
        Ok(())
    }

    /// Non-blocking join: returns `Ok(None)` if still streaming; `Ok(Some(bytes_sent))` if finished.
    fn poll_mock_log_stream(&mut self) -> PyResult<Option<usize>> {
        self.ensure_ok()?;
        let res = map_io(self.inner.try_join_mock_log_stream())?;
        Ok(res)
    }

    /// Blocking join.
    fn join_mock_log_stream(&mut self) -> PyResult<Option<usize>> {
        self.ensure_ok()?;
        let res = map_io(self.inner.join_mock_log_stream())?;
        Ok(res)
    }

    #[pyo3(signature = (block=false))]
    fn get_data_packets(&mut self, block: bool) -> PyResult<Vec<FIRMData>> {
        self.ensure_ok()?;

        let timeout = if block {
            Some(Duration::from_secs_f64(self.timeout))
        } else {
            None
        };

        let packets = map_io(self.inner.get_data_packets(timeout))?;
        Ok(packets)
    }

    #[pyo3(signature = (timeout_seconds=5.0))]
    fn get_device_info(&mut self, timeout_seconds: f64) -> PyResult<Option<DeviceInfo>> {
        self.ensure_ok()?;
        let info = map_io(self.inner.get_device_info(Duration::from_secs_f64(timeout_seconds)))?;
        Ok(info)
    }

    #[pyo3(signature = (timeout_seconds=5.0))]
    fn get_device_config(&mut self, timeout_seconds: f64) -> PyResult<Option<DeviceConfig>> {
        self.ensure_ok()?;
        let cfg = map_io(self.inner.get_device_config(Duration::from_secs_f64(timeout_seconds)))?;
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
        self.ensure_ok()?;

        let res = map_io(self.inner.set_device_config(
            name,
            frequency,
            protocol,
            Duration::from_secs_f64(timeout_seconds),
        ))?;

        Ok(res.unwrap_or(false))
    }

    #[pyo3(signature = (timeout_seconds=5.0))]
    fn cancel(&mut self, timeout_seconds: f64) -> PyResult<bool> {
        self.ensure_ok()?;

        let res = map_io(self.inner.cancel(Duration::from_secs_f64(timeout_seconds)))?;
        Ok(res.unwrap_or(false))
    }

    fn reboot(&mut self) -> PyResult<()> {
        self.ensure_ok()?;
        map_io(self.inner.reboot())?;
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

#[pymethods]
impl MockDeviceHandle {
    fn inject_response(&self, identifier: u16, payload: Vec<u8>) {
        let packet = FramedPacket::new(PacketHeader::Response, identifier, payload);
        self.inner.inject_framed_packet(packet);
    }

    #[pyo3(signature = (timeout_seconds))]
    fn wait_for_command_identifier(&self, timeout_seconds: f64) -> PyResult<Option<u16>> {
        map_io(self.inner.wait_for_command_identifier(Duration::from_secs_f64(
            timeout_seconds,
        )))
    }
}

#[pymodule(gil_used = false)]
fn firm_client(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FIRMClient>()?;
    m.add_class::<MockDeviceHandle>()?;
    m.add_class::<FIRMData>()?;
    m.add_class::<DeviceProtocol>()?;
    m.add_class::<DeviceInfo>()?;
    m.add_class::<DeviceConfig>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
