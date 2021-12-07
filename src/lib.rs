pub mod cqe;
pub mod sqe;

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    io,
    os::unix::prelude::RawFd,
    ptr::addr_of_mut,
};

use uring_sys::*;

use bitflags::bitflags;

use jemallocator::Jemalloc;

use anyhow::anyhow;

use self::{cqe::Cqe, sqe::Sqe};

#[global_allocator]
pub static GLOBAL: Jemalloc = Jemalloc;

// See `io_uring_setup(2)` for explianation.
bitflags! {
    pub struct SetupFlag: u32 {
        const IO_POLL = 0b0000001;
        const SQ_POLL = 0b0000010;
        const SQ_AFF = 0b0000100;
        const CQ_SIZE = 0b0001000;
        const CLAMP = 0b0010000;
        const ATTACH_WQ = 0b0100000;
        const RING_DISABLED = 0b1000000;
    }
}

// See `io_uring_setup(2)` for explianation.
bitflags! {
    pub struct RingFeature: u32 {
        const SINGLE_MMAP = 0b00000000001;
        const NO_DROP = 0b00000000010;
        const SUBMIT_STABLE = 0b00000000100;
        const RW_CUR_POS = 0b00000001000;
        const CUR_PERSONALITY = 0b00000010000;
        const FAST_POLL = 0b00000100000;
        const POLL32 = 0b00001000000;
        const SQ_POLL_NON_FIXED = 0b00010000000;
        const EXT_ARG = 0b00100000000;
        const NATIVE_WORKERS = 0b01000000000;
        const RSRC_TAGS = 0b10000000000;
    }
}

pub struct Rring {
    _inner: *mut io_uring,
    _layout: Layout,
}

impl Rring {
    pub fn new(entries: u32, flags: SetupFlag) -> io::Result<Self> {
        unsafe {
            let layout: Layout = Layout::new::<io_uring>();
            let inner: *mut io_uring = alloc_zeroed(layout).cast();
            let ret = io_uring_queue_init(entries, inner, flags.bits());
            if ret < 0 {
                let errno = -ret;
                Err(io::Error::from_raw_os_error(errno))
            } else {
                Ok(Self {
                    _inner: inner,
                    _layout: layout,
                })
            }
        }
    }
    pub fn with_param(entries: u32, param: RringParams) -> io::Result<Self> {
        unsafe {
            let layout: Layout = Layout::new::<io_uring>();
            let inner: *mut io_uring = alloc_zeroed(layout).cast();
            let mut param = param.to_raw();
            let ret = io_uring_queue_init_params(entries, inner, &mut param);
            if ret < 0 {
                let errno = -ret;
                Err(io::Error::from_raw_os_error(errno))
            } else {
                Ok(Self {
                    _inner: inner,
                    _layout: layout,
                })
            }
        }
    }
    pub fn submit(&self) -> i32 {
        unsafe { io_uring_submit(self._inner) }
    }
    pub fn get_sqe(&self) -> anyhow::Result<Sqe> {
        unsafe {
            let raw = io_uring_get_sqe(self._inner);
            if raw.is_null() {
                Err(anyhow!("SQ is currently full."))
            } else {
                Ok(Sqe::from_raw(raw))
            }
        }
    }
    pub fn wait(&self) -> anyhow::Result<Cqe> {
        let mut cqe: *mut io_uring_cqe = std::ptr::null_mut();
        let ptr: *mut *mut io_uring_cqe = addr_of_mut!(cqe);
        let retval = unsafe { io_uring_wait_cqe(self._inner, ptr) };
        if retval != 0 {
            let eno = -retval;
            Err(anyhow::Error::from(io::Error::from_raw_os_error(eno)))
        } else {
            Ok(Cqe::from_raw(cqe))
        }
    }
    pub fn seen(&self, cqe: Cqe) {
        unsafe {
            io_uring_cqe_seen(self._inner, cqe._inner);
        }
    }
    pub fn exit(&mut self) {
        let ptr = self._inner;
        unsafe {
            io_uring_queue_exit(ptr);
        }
    }
}

impl Drop for Rring {
    fn drop(&mut self) {
        self.exit();
        unsafe {
            dealloc(self._inner.cast(), self._layout);
        }
    }
}

pub struct RringParams {
    flags: SetupFlag,
    features: RingFeature,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
}

impl RringParams {
    pub fn new(flags: SetupFlag, features: RingFeature) -> Self {
        Self {
            flags,
            features,
            sq_thread_cpu: 0,
            sq_thread_idle: 0,
        }
    }
    pub fn set_sq_thread_cpu(&mut self, val: u32) {
        if (self.flags & SetupFlag::SQ_POLL).bits() != 0 {
            self.sq_thread_cpu = val;
        }
    }
    pub fn set_sq_thread_idle(&mut self, val: u32) {
        if (self.flags & SetupFlag::SQ_POLL).bits() != 0 {
            self.sq_thread_idle = val;
        }
    }
    pub unsafe fn to_raw(&self) -> io_uring_params {
        let mut param: io_uring_params = std::mem::zeroed();
        param.flags = self.flags.bits();
        param.features = self.features.bits();
        param.sq_thread_cpu = self.sq_thread_cpu;
        param.sq_thread_idle = self.sq_thread_idle;
        param
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Read,
    Write,
    Readv,
    Writev,
    Fsync,
    Close,
    Openat,
    Send,
    Recv,
    Accept,
}

// using u128 so it's compatible with UUID, Ulid, etc.
#[derive(Debug, Clone, Copy)]
pub struct Identifier(pub u128);

pub struct UserData<T> {
    op: Operation,
    id: Identifier,
    srcfd: RawFd,
    data: Option<Box<T>>,
}

impl<T> UserData<T> {
    pub fn new(op: Operation, id: Identifier, srcfd: RawFd) -> Self {
        Self {
            op,
            id,
            srcfd,
            data: None,
        }
    }
    pub fn with_data(op: Operation, id: Identifier, srcfd: RawFd, data: T) -> Self {
        Self {
            op,
            id,
            srcfd,
            data: Some(Box::new(data)),
        }
    }
    pub fn set_data(&mut self, data: T) {
        self.data = Some(Box::new(data));
    }
    pub fn op(&self) -> Operation {
        self.op
    }
    pub fn id(&self) -> Identifier {
        self.id
    }
    pub fn srcfd(&self) -> RawFd {
        self.srcfd
    }
    pub fn data(&self) -> Option<&T> {
        self.data.as_deref()
    }
}
