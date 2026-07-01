use crate::{
    cpu::{self, get_current_task},
    elf,
    ipc::{IPCMessage, IPCState, alloc_con, conn_mut, find_conn_to, read_ipc, recv_any, write_ipc},
    irq,
    msr::{rdmsr, wrmsr},
    pmm,
    scheduler::{self, Task, TaskState, cleanup_and_exit_task, find_task_by_pid, yield_now},
    serial, vmm,
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
        wrmsr(0xC0000081u32, (0x0008u64 << 32) | (0x0013u64 << 48));

        // lstar
        wrmsr(0xC0000082u32, syscall_entry as u64);

        // sfmask
        wrmsr(0xC0000084u32, 1u64 << 9);
    }
}

unsafe fn route_ipc_peer(me: *mut Task, my_ipcd: i32) -> Option<(*mut Task, i32)> {
    unsafe {
        let peer_pid = conn_mut(me, my_ipcd)?.peer_pid;
        let target = find_task_by_pid(peer_pid);
        if target.is_null() {
            return None;
        }
        let tgt_ipcd = match find_conn_to(target, (*me).pid) {
            Some(ipcd) => ipcd,
            None => alloc_con(target, (*me).pid),
        };
        if tgt_ipcd < 0 {
            return None;
        }
        return Some((target, tgt_ipcd));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(nr: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    match nr {
        0 => {
            // exit
            unsafe {
                cleanup_and_exit_task(cpu::get_current_task());
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

            let stack_pages: u64 = 64; // 256 kilo byteroonies should be enuff
            let mut i: u64 = 0;
            while i < stack_pages {
                let sp = pmm::alloc_pages(0) as u64;
                vmm::map_page(pml4, USER_STACK + i * 0x1000, sp, 0x07);
                i += 1;
            }
            let child_pid = scheduler::spawn_user_task(
                entry.unwrap(),
                USER_STACK + stack_pages * 0x1000,
                pml4 as u64,
                1,
                arg3 as u8,
            );
            return child_pid as u64;
        },
        6 => {
            // ipc write
            // arg1 is my ipcd
            // arg2 is the message
            // arg3 is the len of the message
            unsafe {
                let me = get_current_task();
                if me.is_null() {
                    return 1;
                }

                let Some((target, tgt_ipcd)) = route_ipc_peer(me, arg1 as i32) else {
                    return 1;
                };

                return write_ipc(target, tgt_ipcd, arg2 as *const u8, arg3 as i32) as u64;
            }
        }
        7 => {
            // ipc read
            // arg1 is out param
            // arg2 is my ipcd
            unsafe {
                let task = cpu::get_current_task();
                if task.is_null() {
                    return 1;
                }

                if conn_mut(task, arg2 as i32).is_none() {
                    return 1;
                }

                let msg = read_ipc(task, arg2 as i32);

                let out = arg1 as *mut IPCMessage;
                *out = *msg;

                return 0;
            }
        }
        8 => {
            // print for debugging - goes thru serial
            let bytes = unsafe { core::slice::from_raw_parts(arg1 as *const u8, arg2 as usize) };

            serial::lock();
            for byte in bytes {
                serial::write_byte(*byte);
            }
            serial::unlock();

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
            // arg1 = irq number, arg2 = service_id
            irq::register(arg1 as usize, arg2 as u32);
            return 0;
        }
        11 => unsafe {
            // call - basicalyl ipc write AND then read to wait for tha repsonse
            // arg1 is my ipcd
            // arg2 is the message
            // arg3 is the len of the message
            // arg4 is the out param for the message back

            let me = get_current_task();
            if me.is_null() {
                return 1;
            }
            let ipcd = arg1 as i32;

            match conn_mut(me, ipcd) {
                Some(con) => con.state = IPCState::AwaitingReply,
                None => return 1,
            }

            let Some((target, tgt_ipcd)) = route_ipc_peer(me, ipcd) else {
                if let Some(con) = conn_mut(me, ipcd) {
                    con.state = IPCState::Open;
                }
                return 1;
            };

            if write_ipc(target, tgt_ipcd, arg2 as *const u8, arg3 as i32) != 0 {
                if let Some(con) = conn_mut(me, ipcd) {
                    con.state = IPCState::Open;
                }
                return 1;
            }

            loop {
                let con = conn_mut(me, ipcd).unwrap();
                if con.has_msg {
                    break;
                }
                scheduler::block_current();
            }

            let out = arg4 as *mut IPCMessage;
            let con = conn_mut(me, ipcd).unwrap();
            *out = crate::ipc::IPC_POOL[con.ipc_pool_idx as usize];
            con.has_msg = false;
            con.state = IPCState::Open;
            return 0;
        },
        12 => unsafe {
            // ipc reply
            // arg1 message
            // arg2 is the len of the message
            // arg3 is my ipcd
            let me = get_current_task();
            if me.is_null() {
                return 1;
            }
            let Some((peer, peer_ipcd)) = route_ipc_peer(me, arg3 as i32) else {
                return 1;
            };
            return write_ipc(peer, peer_ipcd, arg1 as *const u8, arg2 as i32) as u64;
        },
        13 => unsafe {
            // ipc register a connection
            // arg 1 is the peer pid
            // returns the ipcd (ipc descriptor / poor mans fd)
            let me = get_current_task();
            if me.is_null() {
                return u64::MAX;
            }
            let ipcd = match find_conn_to(me, arg1 as i32) {
                Some(i) => i,
                None => alloc_con(me, arg1 as i32),
            };
            return ipcd as u64;
        },
        14 => unsafe {
            // recv_any
            // makes it so servers with more than 1 client can just like wake up when something
            // happens and doesnt wait on a singel cleint
            // arg1 is an out param for the ipcmessage
            // returns the ipcd that responded
            let task = cpu::get_current_task();
            if task.is_null() {
                return 1;
            }
            let mut msg_out = IPCMessage::default();
            let ipcd = recv_any(task, core::ptr::addr_of_mut!(msg_out)) as u64;
            let out = arg1 as *mut IPCMessage;
            *out = msg_out;
            return ipcd;
        },
        15 => unsafe {
            // get self pid
            return (*cpu::get_current_task()).pid as u64;
        },
        16 => unsafe {
            // block: server has no work & arg1 = service_id
            let svc = arg1 as u32;
            let pid = (*cpu::get_current_task()).pid;
            let core = unsafe { cpu::id() };
            crate::ipc::register_server(svc, pid, core);
            if crate::ipc::inbox_has_req(svc) {
                return 0;
            }
            scheduler::block_current();
            return 0;
        },
        17 => {
            // notify wake the server for arg1 = service_id
            crate::ipc::wake_server(arg1 as u32);
            return 0;
        }
        18 => {
            scheduler::set_direct_wake(arg1 != 0);
            return 0;
        }
        _ => u64::MAX,
    }
}
