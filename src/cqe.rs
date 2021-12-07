use std::io;
use uring_sys::*;

use super::{Operation, UserData};

pub struct Cqe {
    pub(crate) _inner: *mut io_uring_cqe,
}
#[derive(Debug)]
pub struct OperationError {
    op: Operation,
    err: io::Error,
}

impl OperationError {
    fn op_err(op: Operation, err_code: i32) -> Self {
        Self {
            op,
            err: io::Error::from_raw_os_error(err_code),
        }
    }
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error when performing IO operation {:?}: {}",
            self.op,
            self.err.to_string()
        )
    }
}

impl std::error::Error for OperationError {}

impl Cqe {
    pub(crate) fn from_raw(raw: *mut io_uring_cqe) -> Self {
        Self { _inner: raw }
    }
    pub fn get_data<T>(&self) -> Result<Option<Box<UserData<T>>>, OperationError> {
        let op_result = self.get_result();
        let ptr = unsafe { io_uring_cqe_get_data(self._inner) };
        if ptr.is_null() {
            Ok(None)
        } else {
            let data_ptr: *mut UserData<T> = ptr.cast();
            if op_result < 0 {
                let op = unsafe { (*data_ptr).op() };
                let err = OperationError::op_err(op, -op_result);
                Err(err)
            } else {
                let boxed_data = unsafe { Box::from_raw(data_ptr) };
                Ok(Some(boxed_data))
            }
        }
    }
    pub fn get_result(&self) -> i32 {
        unsafe { (*self._inner).res }
    }
}
