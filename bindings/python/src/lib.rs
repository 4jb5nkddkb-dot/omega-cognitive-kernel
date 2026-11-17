use pyo3::prelude::*;

mod error;
mod types;
mod kernel;
mod audit;
mod checkpoint;

use kernel::Kernel;
use audit::AuditLog;
use checkpoint::CheckpointStore;

/// Python module for omega-cognitive-kernel
#[pymodule]
fn omega_cognitive_kernel_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Kernel>()?;
    m.add_class::<AuditLog>()?;
    m.add_class::<CheckpointStore>()?;
    Ok(())
}