use crate::conf::Service;
use nix::{
    errno::Errno,
    libc,
    unistd::{fork, ForkResult, Pid},
};

pub fn start_service(service: &Service) -> Result<Pid, Errno> {
    println!("Starting service: {}", service.name);
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            println!("I'm the parent! Child has PID {}", child);
            Ok(child)
        }
        Ok(ForkResult::Child) => {
            let prog = std::ffi::CString::new(service.cmd.clone()).unwrap();

            // Extract the program name from the path (e.g., "sleep" from "/path/to/sleep")
            let prog_name = std::path::Path::new(&service.cmd)
                .file_name()
                .expect("Failed to get program name")
                .to_str()
                .expect("Program name not valid UTF-8");

            let prog_name_c = std::ffi::CString::new(prog_name).unwrap();

            let mut c_strings: Vec<*const i8> = Vec::with_capacity(service.args.len() + 2);

            // argv[0] should be the program name, not the full path!
            c_strings.push(prog_name_c.as_ptr());

            // Convert each string in the Vec<String> into a CString (C-style string)
            for s in service.args.clone() {
                // CString automatically null-terminates the string
                let c_str = std::ffi::CString::new(s).expect("CString::new failed");
                // Push the pointer to the C-string into the vector
                c_strings.push(c_str.as_ptr());
                // Ensure the CString is kept alive for as long as necessary
                std::mem::forget(c_str); // Prevent it from being deallocated
            }

            // Null-terminate the array
            c_strings.push(std::ptr::null());

            // Create a raw pointer to the Vec<*const i8>
            let argv: *const *const i8 = c_strings.as_ptr();

            match unsafe { libc::execv(prog.as_ptr(), argv) } {
                -1 => {
                    eprintln!("execv errored");
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("execv returned Ok()... This should never happen");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => Err(e),
    }
}
