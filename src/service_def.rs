use crate::conf::ServiceConf;
use crate::exec_args::ExecArgs;

#[derive(Clone, Debug)]
pub struct ServiceDef {
    pub name: String,
    pub conf: ServiceConf,
    pub args: ExecArgs,
}

impl ServiceDef {
    pub fn new(conf: &ServiceConf) -> Self {
        Self {
            name: conf.name.clone(),
            conf: conf.clone(),
            args: ExecArgs::new(conf),
        }
    }
}
