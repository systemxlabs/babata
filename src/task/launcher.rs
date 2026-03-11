use crate::{
    BabataResult,
    task::{TaskRequest, manager::RunningTask},
};

#[derive(Debug)]
pub struct TaskLauncher {}

impl TaskLauncher {
    pub fn new() -> BabataResult<Self> {
        Ok(Self {})
    }

    pub fn launch(&self, request: &TaskRequest) -> BabataResult<RunningTask> {
        unimplemented!()
    }
}
