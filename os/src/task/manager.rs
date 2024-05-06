//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // stride调度,brute-force,无畏性能,乐(
        // 如果不想暴力，我觉得大概率是把当前这个双向队列修改成堆来用。
        if self.ready_queue.len() == 0 {
            return None
        }
        let (mut min_index,mut min_stride) = (0,self.ready_queue[0].inner_exclusive_access().stride);
        for (index,task) in self.ready_queue.iter().enumerate().skip(1) {
            if min_stride >= task.inner_exclusive_access().stride {
                min_index = index;
                min_stride = task.inner_exclusive_access().stride;
            }
        }
        // 感觉可以直接在这里加上PASS值？直接偷懒了？正常应该要在实际调用的地方更新吧？
        let mut inner = self.ready_queue[min_index].inner_exclusive_access();
        inner.stride += BIG_STRIDE/inner.prio;
        drop(inner);
        self.ready_queue.remove(min_index)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
