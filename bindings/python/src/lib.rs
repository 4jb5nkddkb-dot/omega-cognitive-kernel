use pyo3::prelude::*;

mod audit;
mod checkpoint;
mod error;
mod kernel;
mod types;

use audit::AuditLog;
use checkpoint::CheckpointStore;
use kernel::Kernel;

/// Python module for omega-cognitive-kernel
#[pymodule]
fn omega_cognitive_kernel_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Kernel>()?;
    m.add_class::<AuditLog>()?;
    m.add_class::<CheckpointStore>()?;
    Ok(())
}
