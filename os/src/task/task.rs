//! Types related to task management

use crate::config::MAX_SYSCALL_NUM;

use super::TaskContext;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// 任务使用的系统调用及调用次数
    pub syscall_nums: [u32; MAX_SYSCALL_NUM],
    /// 任务是否运行过,
    pub has_been_run:bool,
    /// 任务第一次执行时的时间,单位ms
    pub first_run_time:usize,
    /// The task context
    pub task_cx: TaskContext,
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
