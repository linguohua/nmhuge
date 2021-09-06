use ::libc::{*};
use log::{error, info};
use std::io::{Read, Error, ErrorKind};

// const MPOL_DEFAULT:c_int = 0;
const MPOL_BIND:c_int = 2;
const  MPOL_USED:c_int = MPOL_BIND;
const GB: size_t = 1024*1024*1024;

#[link(name = "numa")]
extern "C" {
    pub fn set_mempolicy(mode: c_int, nmask: *const c_ulong, maxnode: c_ulong) -> c_long;
/*
void *mmap(void *addr, size_t length, int prot, int flags,
                  int fd, off_t offset);
 */
    pub fn mmap(
        addr: *mut c_void,
        length: size_t,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: c_longlong,
    ) -> *mut c_void;

/*
       long mbind(void *addr, unsigned long len, int mode,
                  const unsigned long *nodemask, unsigned long maxnode,
                  unsigned int flags);
 */

    pub fn mbind(
        addr: *mut c_void,
        len: c_ulong,
        mode: c_int,
        nodemask: *const c_ulong,
        maxnode: c_ulong,
        flags: c_uint,
    ) -> c_long;
}

fn main() -> std::io::Result<()> {
    // unsafe {
    //     let nmask = 0x01;
    //     let lr = set_mempolicy(MPOL_USED, &nmask, 32);
    //     if lr != 0 {
    //         error!("set_mempolicy failed:{}, nmask:{:#b}, error:{:?}",lr, nmask, std::io::Error::last_os_error());
    //     } else {
    //         info!("set_mempolicy OK, nmask:{:#b}, bind:{}",nmask, MPOL_USED);
    //     }
    // }

    unsafe {
        let gb = 8;
        let protect_flags = PROT_READ|PROT_WRITE;
        let flags = MAP_PRIVATE | MAP_ANONYMOUS|MAP_HUGE_1GB;
        let start_addr:c_ulonglong = 32 << 40; // 32TB
        let start_ptr = start_addr as *mut c_ulonglong as *mut c_void;
        let addr = mmap(start_ptr, gb*GB,
            protect_flags, flags, -1, 0);
        if addr as c_ulonglong == 0 {
            error!("mmap failed:{}", Error::last_os_error());
            return Err(Error::new(ErrorKind::Other, "mmap failed"));
        }

        info!("mmap ok, got: {}", gb);
        let nmask = 0x01;
        let mode = MPOL_USED;
        let bflags = 0;
        let result = mbind(addr, (gb*GB) as c_ulong, mode,
        &nmask, 32, bflags);
        if result != 0 {
            error!("mbind failed:{}", Error::last_os_error());
            return Err(Error::new(ErrorKind::Other, "mbind failed"));
        }
        info!("mbind ok");

        let mut ptr = addr as *mut u32;
        for i in 1..gb {
            *ptr = 0x1;
            ptr = ptr.add(i * GB / std::mem::size_of::<u32>());
        }

        info!("write to {} GB ok", gb);
    }

    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    Ok(())
}
