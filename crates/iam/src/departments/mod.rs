mod error;
mod model;
mod request;
mod service;
pub use error::DeptError;
pub use model::*;
pub use request::*;
pub use service::{DepartmentService, build_dept_tree};
