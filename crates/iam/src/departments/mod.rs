mod error;
mod model;
mod request;
mod service;

pub use error::DeptError;
pub use model::{Dept, DeptNode};
pub use request::{CreateDeptPayload, UpdateDeptPayload};
pub use service::DepartmentService;
