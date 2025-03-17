use nix::libc;
use std::ffi::CString;
use which::which;

pub fn execv(args: &Vec<String>) -> libc::c_int {
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
        .clone()
        .into_iter()
        .map(|s| CString::new(s).expect("Failed to create CString"))
        .collect();

    argv_owned[0] = CString::new(argv0).expect("Failed to create CString");

    let mut argv: Vec<*const libc::c_char> = argv_owned.iter().map(|s| s.as_ptr()).collect();

    argv.push(std::ptr::null());

    let prog = argc.as_ptr();
    let argv = argv.as_ptr();

    unsafe { libc::execv(prog, argv) }
}
