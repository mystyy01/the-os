use crate::{
    cpu::{self, get_current_task},
    elf,
    ipc::{IPCMessage, IPCState, read_ipc, write_ipc},
    irq,
    msr::{rdmsr, wrmsr},
    pmm::{self, PAGE_SIZE},
    scheduler::{self, TaskState, find_task_by_pid, yield_now},
    serial::{self, write_hex},
    vmm,
};

const USER_STACK: u64 = 0x10000000;

unsafe extern "C" {
    fn syscall_entry();
}

pub unsafe fn init() {
    unsafe {
        // efer
        let efer_num = 0xC0000080u32;
        let efer = rdmsr(efer_num);
        wrmsr(efer_num, efer | 1);

        // star
        wrmsr(0xC0000081u32, (0x0008u64 << 32) | (0x0010u64 << 48));

        // lstar
        wrmsr(0xC0000082u32, syscall_entry as u64);

        // sfmask
        wrmsr(0xC0000084u32, 1u64 << 9);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(nr: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    match nr {
        0 => {
            // exit
            unsafe {
                let curr_task = *(cpu::get_current_task());
                core::arch::asm!("mov cr3, {}", in(reg) cpu::get_kernel_cr3());
                vmm::free_table(curr_task.cr3, 4);
                if !curr_task.stack.is_null() {
                    pmm::free_pages(0, curr_task.stack as u64);
                }
                scheduler::kill_current_task();
            }
            return 0;
        }
        1 => {
            let addr = pmm::alloc_pages(arg1 as usize) as u64;
            return addr;
        }
        2 => {
            pmm::free_pages(arg2 as usize, arg1);
            return 0;
        }
        3 => unsafe {
            let task = *(cpu::get_current_task());
            vmm::map_page(task.cr3 as *mut u64, arg1, arg2, arg3);
            return 0;
        },
        4 => unsafe {
            let task = *(cpu::get_current_task());
            vmm::unmap_page(task.cr3 as *mut u64, arg1);
            return 0;
        },
        5 => unsafe {
            // spawn task
            let pml4 = vmm::create_address_space();
            let entry = elf::load(arg1 as *const u8, arg2 as usize, pml4);
            if entry == None {
                vmm::free_table(pml4 as u64, 4);
                return u64::MAX;
            }
            let stack_phys = pmm::alloc_pages(0) as u64;
            vmm::map_page(pml4, USER_STACK, stack_phys, 0x07);
            let child_pid =
                scheduler::spawn_user_task(entry.unwrap(), USER_STACK + 0x1000, pml4 as u64, 1);
            return child_pid as u64;
        },
        6 => {
            // ipc write
            // arg1 is the pid target
            // arg2 is the message
            // arg3 is the len of the message
            let task = find_task_by_pid(arg1 as i32);
            if task.is_null() {
                return 1;
            }

            write_ipc(task, arg2 as *const u8, arg3 as i32);

            return 0;
        }
        7 => {
            // ipc read
            // arg1 is out param
            unsafe {
                let task = cpu::get_current_task();
                if task.is_null() {
                    return 1;
                }

                let msg = read_ipc(task);

                let out = arg1 as *mut IPCMessage;
                *out = *msg;

                return 0;
            }
        }
        8 => {
            // print for debugging - goes thru serial
            let bytes = unsafe { core::slice::from_raw_parts(arg1 as *const u8, arg2 as usize) };

            for byte in bytes {
                serial::write_byte(*byte);
            }

            return 0;
        }
        9 => {
            // yield
            // i really shoulda done these comments for the other syscalls cuz i lowk forgot what
            // they do
            yield_now();
            return 0;
        }
        10 => {
            // register an irq reader
            // arg1 = irq number
            let pid = unsafe { (*cpu::get_current_task()).pid };
            irq::register(arg1 as usize, pid);
            return 0;
        }
        11 => unsafe {
            // call - basicalyl ipc write AND then read to wait for tha repsonse
            // arg1 is the pid target
            // arg2 is the message
            // arg3 is the len of the message
            // arg4 is the out param for the message back

            let target = find_task_by_pid(arg1 as i32);
            if target.is_null() {
                return 1;
            }
            let me = get_current_task();

            (*me).ipc_con.peer_pid = arg1 as i32;
            (*me).ipc_con.state = IPCState::AwaitingReply;

            write_ipc(target, arg2 as *const u8, arg3 as i32);

            while !(*me).ipc_con.has_msg {
                scheduler::block_current();
            }
            (*me).ipc_con.has_msg = false;
            (*me).ipc_con.state = IPCState::Open;

            let out = arg4 as *mut IPCMessage;
            *out = (*me).ipc_con.msg;
            return 0;
        },
        12 => unsafe {
            // ipc reply
            // arg1 message
            // arg2 is the len of the message
            let me = get_current_task();
            let peer = find_task_by_pid((*me).ipc_con.peer_pid);
            if peer.is_null() {
                return 1;
            }
            write_ipc(peer, arg1 as *const u8, arg2 as i32);
            return 0;
        },
        _ => u64::MAX,
    }
}
