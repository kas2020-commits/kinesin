use crate::conf::ServiceConf;
use nix::libc;
use std::ffi::CString;
use which::which;

#[derive(Clone, Debug)]
pub struct ExecArgs {
    pub args: Vec<CString>,
    pub argc: CString,
}

impl ExecArgs {
    pub fn new(service: &ServiceConf) -> Self {
        let self_path = which(&service.cmd).unwrap();

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
        let mut args: Vec<CString> = service
            .args
            .clone()
            .into_iter()
            .map(|s| CString::new(s).expect("Failed to create CString"))
            .collect();

        args.insert(0, CString::new(argv0).expect("Failed to create CString"));

        Self { argc, args }
    }

    pub fn to_argv(&self) -> Vec<*const libc::c_char> {
        let mut argv: Vec<*const libc::c_char> = self.args.iter().map(|s| s.as_ptr()).collect();

        argv.push(std::ptr::null());
        argv
    }
}
