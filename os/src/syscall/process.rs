//! Process management syscalls
//!
use alloc::sync::Arc;

use core::mem::size_of;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str,translated_byte_buffer,MapPermission}, 
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        mmap_helper, munmap_helper, suspend_current_and_run_next, TaskStatus,get_task_info
    },timer::{get_time_ms, get_time_us}
    
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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
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
        let (status,syscall_nums,first_run_time) =  get_task_info();
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
    if port & !0x7 != 0 || port & 0x7 == 0  { // 不满足port定义，错误
        return -1;
    }
    if start & (PAGE_SIZE-1) != 0 { // 参数没有对齐，错误
        return  -1;
    } 
    let start_va = start.into();
    let end_va = (start+len).into(); //向上取整
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
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if start & (PAGE_SIZE-1) != 0 { // 参数没有对齐，错误
        return  -1;
    } 
    let start_va = start.into();
    let end_va = (start+len).into();
    if munmap_helper(start_va, end_va){
        return 0
    }
    -1
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // 首先获取程序的elf数据,这一步直接copy exec的实现
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let new_task = task.spawn(all_data.as_slice());
        let new_pid = new_task.pid.0;
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        trap_cx.x[10] = 0;
        // add new task to scheduler
        add_task(new_task);
        new_pid as isize
    } else {
        // 无效的文件名，不存在elf数据，调用失败
        println!("invalid path name {}",path);
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // 参数检查,要求优先级设置钟prio>=2
    if prio < 2 {
        return -1;
    }
    let task = current_task().unwrap();
    task.change_prio(prio as usize);
    return prio;
}
