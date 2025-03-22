//! A helper utility for calling exec out of a String slice.
//!
//! In order to call the exec family of syscalls one needs to provide values
//! with the type char `*const *const c_char` which are structured as
//! a list of null-terminated strings which is, itself, null-terminated.
use nix::libc;
use std::ffi::CString;
use which::which;

pub fn execve(args: &[String], envp: &[String]) -> libc::c_int {
    let self_path = which(&args[0]).unwrap();

    let argc = CString::new(
        self_path
            .to_str()
            .expect("Path provided can't be turned into a string"),
    )
    .expect("Can't convert to CString");

    let argv0 = std::path::Path::new(&self_path)
        .file_name()
        .expect("Failed to get program name")
        .to_str()
        .expect("Program name not valid UTF-8");

    // Convert args into CStrings
    let mut argv_owned: Vec<CString> = args
        .to_owned()
        .clone()
        .into_iter()
        .map(|s| CString::new(s).expect("Failed to create CString"))
        .collect();

    let envp_owned: Vec<CString> = envp
        .to_owned()
        .clone()
        .into_iter()
        .map(|s| CString::new(s).expect("Failed to create CString"))
        .collect();

    let mut envp: Vec<*const libc::c_char> = envp_owned.iter().map(|s| s.as_ptr()).collect();
    envp.push(std::ptr::null());

    argv_owned[0] = CString::new(argv0).expect("Failed to create CString");

    let mut argv: Vec<*const libc::c_char> = argv_owned.iter().map(|s| s.as_ptr()).collect();

    argv.push(std::ptr::null());

    let prog = argc.as_ptr();
    let argv = argv.as_ptr();
    let envp = envp.as_ptr();

    unsafe { libc::execve(prog, argv, envp) }
}
