use std::{
    ffi::{CStr, OsStr},
    io::{IoSlice, IoSliceMut},
    os::unix::prelude::*,
};

use crate::UserData;
use libc::{epoll_event, iovec, msghdr, sockaddr, statx};
use uring_sys::*;

pub struct Sqe {
    _inner: *mut io_uring_sqe,
}

impl Sqe {
    pub(crate) fn from_raw(raw: *mut io_uring_sqe) -> Self {
        Self { _inner: raw }
    }
    pub fn set_user_data<T>(&self, user_data: UserData<T>) {
        let ptr: *mut UserData<T> = Box::into_raw(Box::new(user_data));
        unsafe {
            io_uring_sqe_set_data(self._inner, ptr.cast());
        }
    }
    pub fn read(&self, src: RawFd, buf: &mut [u8], nbytes: u32, offset: i64) {
        unsafe {
            io_uring_prep_read(self._inner, src, buf.as_mut_ptr().cast(), nbytes, offset);
        }
    }
    pub fn write(&self, src: RawFd, buf: &[u8], nbytes: u32, offset: i64) {
        unsafe {
            io_uring_prep_write(self._inner, src, buf.as_ptr().cast(), nbytes, offset);
        }
    }
    pub fn readv(&self, src: RawFd, bufs: &mut [IoSliceMut], offset: i64) {
        let iovecs: Vec<iovec> = bufs
            .iter_mut()
            .map(|b| iovec {
                iov_len: b.len(),
                iov_base: b.as_mut_ptr().cast(),
            })
            .collect();
        unsafe {
            io_uring_prep_readv(
                self._inner,
                src,
                iovecs.as_ptr(),
                iovecs.len() as u32,
                offset,
            );
        }
    }
    pub fn writev(&self, src: RawFd, bufs: &[IoSlice], offset: i64) {
        let iovecs: Vec<iovec> = bufs
            .iter()
            .map(|b| {
                let ptr: *const u8 = b.as_ptr();
                iovec {
                    iov_len: b.len(),
                    iov_base: (ptr as *mut u8).cast(),
                }
            })
            .collect();
        unsafe {
            io_uring_prep_writev(
                self._inner,
                src,
                iovecs.as_ptr(),
                iovecs.len() as u32,
                offset,
            );
        }
    }
    pub fn fsync(&self, src: RawFd, fsync_flags: u32) {
        unsafe {
            io_uring_prep_fsync(self._inner, src.as_raw_fd(), fsync_flags);
        }
    }
    pub fn close(&self, src: RawFd) {
        unsafe {
            io_uring_prep_close(self._inner, src);
        }
    }
    pub fn openat(&self, dir: RawFd, path: &OsStr, flags: i32, mode: u32) {
        unsafe {
            let bytes = path.as_bytes();
            let cpath = CStr::from_bytes_with_nul_unchecked(bytes);
            io_uring_prep_openat(self._inner, dir, cpath.as_ptr(), flags, mode);
        }
    }
    pub fn statx(&self, dir: RawFd, path: &OsStr, flags: i32, mask: u32, buf: *mut statx) {
        unsafe {
            let bytes = path.as_bytes();
            let cpath = CStr::from_bytes_with_nul_unchecked(bytes);
            io_uring_prep_statx(self._inner, dir, cpath.as_ptr(), flags, mask, buf);
        }
    }
    pub fn fadvice(&self, src: RawFd, offset: i64, len: i64, advice: i32) {
        unsafe {
            io_uring_prep_fadvise(self._inner, src, offset, len, advice);
        }
    }
    pub fn madvice(&self, addr: &mut [u8], len: i64, advice: i32) {
        unsafe {
            io_uring_prep_madvise(self._inner, addr.as_mut_ptr().cast(), len, advice);
        }
    }
    pub fn splice(
        &self,
        in_fd: RawFd,
        in_offset: i64,
        out_fd: RawFd,
        out_offset: i64,
        n: u32,
        flags: u32,
    ) {
        unsafe {
            io_uring_prep_splice(self._inner, in_fd, in_offset, out_fd, out_offset, n, flags);
        }
    }
    pub fn recvmsg(&self, src: RawFd, msg: *mut msghdr, flags: u32) {
        unsafe {
            io_uring_prep_recvmsg(self._inner, src, msg, flags);
        }
    }
    pub fn sendmsg(&self, src: RawFd, msg: *mut msghdr, flags: u32) {
        unsafe {
            io_uring_prep_sendmsg(self._inner, src, msg, flags);
        }
    }
    pub fn recv(&self, socket: RawFd, buf: &mut [u8], len: usize, flags: i32) {
        unsafe {
            io_uring_prep_recv(self._inner, socket, buf.as_mut_ptr().cast(), len, flags);
        }
    }
    pub fn send(&self, socket: RawFd, buf: &[u8], len: usize, flags: i32) {
        unsafe {
            io_uring_prep_send(self._inner, socket, buf.as_ptr().cast(), len, flags);
        }
    }
    pub fn accept(&self, src: RawFd, addr: *mut sockaddr, addrlen: &mut u32, flags: i32) {
        unsafe {
            io_uring_prep_accept(self._inner, src, addr, addrlen, flags);
        }
    }
    pub fn connect(&self, src: RawFd, addr: *mut sockaddr, addrlen: u32) {
        unsafe {
            io_uring_prep_connect(self._inner, src, addr, addrlen);
        }
    }
    pub fn epoll_ctl(&self, epfd: RawFd, src: RawFd, op: i32, ev: *mut epoll_event) {
        unsafe {
            io_uring_prep_epoll_ctl(self._inner, epfd, src, op, ev);
        }
    }
    pub fn poll_add(&self, src: RawFd, poll_mask: i16) {
        unsafe {
            io_uring_prep_poll_add(self._inner, src, poll_mask);
        }
    }
    pub fn poll_remove<T>(&self, user_data: *mut T) {
        unsafe {
            io_uring_prep_poll_remove(self._inner, user_data.cast());
        }
    }
}
