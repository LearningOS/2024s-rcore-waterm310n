//! Process management syscalls
use core::mem::size_of;

use crate::{
    config::MAX_SYSCALL_NUM, mm::{translated_byte_buffer, MapPermission}, task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_task_info, mmap_helper, suspend_current_and_run_next, TaskStatus
    }, timer::{get_time_ms, get_time_us}
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // ts地址是一个用户态的虚拟地址，所以需要找到它的物理地址，才能进行修改。
    let token = current_user_token(); //获取当前的用户token
    // println!("ts origin virtual address {:p} ",ts);
    let buffers = translated_byte_buffer(token, ts as *const u8, size_of::<TimeVal>());
    // println!("ts physical address {:p}",buffers[0].as_ptr());
    let ts:*mut TimeVal = buffers[0].as_ptr() as *mut TimeVal; //希望这个结构体不会被跨页
    // println!("ts real physical address {:p}",ts);
    let us = get_time_us(); //获取时间
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let token = current_user_token(); //获取当前的用户token
    let buffers = translated_byte_buffer(token, ti as *const u8, size_of::<TimeVal>());
    let ti:*mut TaskInfo = buffers[0].as_ptr() as *mut TaskInfo;
    unsafe {
        let (status,syscall_nums,first_run_time)=  get_task_info();
        (*ti).status = status;
        for i in 0..MAX_SYSCALL_NUM {
            (*ti).syscall_times[i] = syscall_nums[i];
        }
        (*ti).time = get_time_ms()-first_run_time;
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if port & !0x7 != 0 || port & 0x7 == 0 { // 不满足port定义，错误
        return -1
    }
    let start_va = start.into();
    let end_va = (start+len).into();
    let mut permission = MapPermission::U;
    if port & 0x1 == 1 {
        permission |= MapPermission::R;
    }
    if (port>>1) & 0x1 == 1 {
        permission |= MapPermission::W;
    }
    if (port>>2) & 0x1 == 1{
        permission |= MapPermission::X;
    }
    println!(" {:?},{:?}",start_va,end_va);
    if mmap_helper(start_va,end_va,permission){
        return 0;
    }else{
        return -1;
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
