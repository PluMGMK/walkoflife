/*!
  Functions for manipulating the memory of another program on Linux
  (with a particular view to Rayman 2).
  */

extern crate nix;

use nix::{unistd::Pid,sys::uio::{process_vm_readv,process_vm_writev,IoVec,RemoteIoVec},Result};
use std::mem::size_of;

/// Read `n` primitives (i.e. objects implementing `Copy`) from the memory of a process given by
/// `pid`, starting from a location given by `offset`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
///
/// ## Returns:
/// * Return type is a [`nix::Result`](../../nix/type.Result.html), reflecting the success or
/// failure of the underlying operation(s).
/// * On success, returns a `Vec<T>` containing the data read, with `len()` equal to `n`.
pub fn read_prims<T:Copy>(pid: Pid, offset: usize, n: usize) -> Result<Vec<T>> {
    let bytes_per_prim = size_of::<T>();
    let mut ret: Vec<T> = Vec::with_capacity(n);

    let byteslice = unsafe{std::slice::from_raw_parts_mut(ret.as_mut_ptr().cast::<u8>(), n * bytes_per_prim)};
    let iovec = IoVec::from_mut_slice(byteslice);
    let iovec_rem = RemoteIoVec{base: offset, len: n * bytes_per_prim};

    let bytes_copied = process_vm_readv(pid, &[iovec], &[iovec_rem])?;
    unsafe {
        ret.set_len(bytes_copied / bytes_per_prim);
    }
    Ok(ret)
}

/// Read a UTF-8 string from the memory of a process given by `pid`, starting from the location
/// given by `offset`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
///
/// ## Returns:
/// * Return type is a [`nix::Result`](../../nix/type.Result.html), reflecting the success or
/// failure of the underlying operation(s).
/// * On success, returns a `String` at most `n` bytes long. It can be shorter if a null terminator
/// or invalid character is found.
pub fn read_string(pid: Pid, offset: usize, n: usize) -> Result<String> {
    let bytes = read_prims::<u8>(pid, offset, n)?;
    // Truncate at null terminator
    let trunc = match bytes.iter().position(|&x| x==0) {
        Some(idx) => bytes[0..idx].to_vec(),
        None => bytes,
    };
    match String::from_utf8(trunc) {
        Ok(string) => Ok(string),
        Err(err) => Ok(String::from_utf8(read_prims::<u8>(pid, offset, err.utf8_error().valid_up_to()).unwrap()).unwrap()),
    }
}

/// Look up a pointer in the memory of the process given by `pid`, by following a "path".
///
/// ## Details:
/// * `base` is a pointer which gives the beginning of the "path", i.e. this function will
/// start by reading a 32-bit integer from the memory location given by `base`.
/// * If any `offsets` are specified, each one indicates another node on the "path". So after
/// reading the integer at `base`, it will add `offsets.unwrap()[0]` to that integer, and then use
/// it as another pointer to read the next integer.
///     * Then it will add `offsets.unwrap()[1]` to _that_ integer, and use it as the next pointer,
/// and so on.
/// * Note that the `offsets` can all be zero, to follow a simple path (in which each pointer
/// simply points to the next one).
/// * The return value is the final integer read.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
///
/// ## Returns:
/// * Return type is a [`nix::Result`](../../nix/type.Result.html), reflecting the success or
/// failure of the underlying operation(s).
/// * On success, returns a `usize` corresponding to the desired pointer.
pub fn get_pointer_path(pid: Pid, base: usize, offsets: Option<&Vec<usize>>) -> Result<usize> {
    let mut cur_address = base;

    // Rayman 2 is 100% 32-bit, so we need to cast a u32 to a usize.
    cur_address = read_prims::<u32>(pid, cur_address, 1)?[0] as usize;

    if let Some(offs) = offsets {
        for offset in offs.iter() {
            cur_address = read_prims::<u32>(pid, cur_address + offset, 1)?[0] as usize;
        }
    }

    Ok(cur_address)
}

/// Write an array (technically a vector) of primitives (i.e. objects implementing `Copy`) to 
/// the memory of a process given by `pid`, starting from a location given by `offset`.
///
/// ## Requirements:
/// * We need to have permissions to debug `pid` (e.g. with `CAP_SYS_PTRACE`).
///
/// ## Returns:
/// * Return type is a [`nix::Result`](../../nix/type.Result.html), reflecting the success or
/// failure of the underlying operation(s).
/// * On success, returns `Ok(())`.
pub fn write_prims<T:Copy>(pid: Pid, offset: usize, data: &Vec<T>) -> Result<()> {
    let bytes_per_prim = size_of::<T>();
    let n = data.len();

    let byteslice = unsafe{std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), n * bytes_per_prim)};
    let iovec = IoVec::from_slice(byteslice);
    let iovec_rem = RemoteIoVec{base: offset, len: n * bytes_per_prim};

    let _ = process_vm_writev(pid, &[iovec], &[iovec_rem])?;
    Ok(())
}

#[cfg(test)]
mod byte_tests {
    use super::*;
    use nix::{sys::{ptrace,wait::{waitpid,WaitStatus},signal::{raise,Signal::SIGTRAP}},unistd::{fork,ForkResult},libc::SYS_write};

    #[test]
    fn can_read_strings() {
        match fork().expect("Fork failed") {
            ForkResult::Parent { child, .. } => {
                let mut foundwrite = false;
                loop {
                    match waitpid(child, None) {
                        Ok(WaitStatus::Exited(_,_)) => {
                            assert!(foundwrite, "Child never wrote anything - you need to run this test with --nocapture");
                            break;
                        }
                        _ => {
                            let child_regs = ptrace::getregs(child)
                                .expect("Unable to get child registers");
                            if child_regs.orig_rax as i64 == SYS_write {
                                foundwrite = true;
                                assert_eq!(
                                    read_string(child, child_regs.rsi as usize, child_regs.rdx as usize).unwrap(),
                                    "Hello, world!\n"
                                    );
                            }
                            ptrace::syscall(child)
                                .expect("Unable to get syscall from child");
                        }
                    }
                }
            },
            ForkResult::Child => {
                ptrace::traceme()
                    .expect("Child unable to get traced");
                raise(SIGTRAP).expect("Unable to raise trap");
                println!("Hello, world!");
            },
        }
    }
}
