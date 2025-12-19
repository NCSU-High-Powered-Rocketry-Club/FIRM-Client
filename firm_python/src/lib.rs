use pyo3::prelude::*;
use firm_rust::FIRMClient as RustFirmClient;
use firm_core::data_parser::FIRMPacket;
use firm_core::command_sender::FIRMCommand;

#[pyclass(unsendable)]
struct FIRMClient {
    inner: RustFirmClient,
    timeout: f64
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
        Ok(FIRMClient { inner: client , timeout: timeout_val })
    }

    fn start(&mut self) -> PyResult<()> {
        self.inner.start();
        Ok(())
    }

    fn stop(&mut self) {
        self.inner.stop();
    }

    #[pyo3(signature = (block=false))]
    fn get_data_packets(&mut self, block: bool) -> PyResult<Vec<FIRMPacket>> {
        if let Some(err) = self.inner.check_error() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(err));
        }

        let timeout = if block {
            Some(std::time::Duration::from_secs_f64(self.timeout))
        } else {
            None
        };
        // Get all packets, and return early if there's an error
        let packets = self.inner.get_data_packets(timeout)
                        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(packets)
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    fn zero_out_pressure_altitude(&mut self) {
        self.inner.zero_out_pressure_altitude();
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
        FIRMCommand::Ping.to_bytes()
    }

    /// Creates a Reset command.
    ///
    /// Returns
    /// -------
    /// bytes
    ///     The serialized command bytes.
    #[staticmethod]
    fn reset() -> Vec<u8> {
        FIRMCommand::Reset.to_bytes()
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
        FIRMCommand::SetRate(rate_hz).to_bytes()
    }
}


#[pymodule(gil_used = false)]
fn firm_client(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FIRMClient>()?;
    m.add_class::<FIRMPacket>()?;
    m.add_class::<FirmCommandBuilder>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
