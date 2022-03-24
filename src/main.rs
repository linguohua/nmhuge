use ::libc::{*};
use log::{error, info};
use std::io::{Error, ErrorKind, Result};
use simple_logger::SimpleLogger;

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

    pub fn mlock(
        addr: *mut c_void,
        len: c_ulong,
    ) -> c_long;

/*
        int munmap(void *addr, size_t length);
 */
    pub fn munmap(
        addr: *mut c_void,
        len: c_ulong,
    ) -> c_long;


/*
        int memfd_create(const char *name, unsigned int flags);
 */
    pub fn memfd_create(name: *const c_char, flags: c_uint) -> c_int;

/*
        int ftruncate(int fd, off_t length);
 */
    pub fn ftruncate(fd: c_int, length: c_longlong) -> c_int;
}

pub const MFD_HUGE_1GB:c_uint = 0x78000000;
pub const MFD_CLOEXEC:c_uint = 0x0001;

// MFD_HUGETLB|MFD_HUGE_1GB
fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();

    let gb: usize = 4;
    let start_addr:c_ulonglong = 32 << 40; // 32TB

    // create memory file fd
    let cstr = std::ffi::CString::new("hello").unwrap();
    let flags: c_uint = MFD_CLOEXEC|MFD_HUGETLB|MFD_HUGE_1GB;

    let fd = unsafe {memfd_create(cstr.as_ptr(), flags)};
    if fd < 0 {
        let e = format!("memfd_create failed:{}", Error::last_os_error());
        error!("{}", e);
        return Err(Error::new(ErrorKind::Other, e));
    }

    let ir = unsafe { ftruncate(fd, (gb*GB) as i64)};
    if ir < 0 {
        let e = format!("ftruncate failed:{}", Error::last_os_error());
        error!("{}", e);
        return Err(Error::new(ErrorKind::Other, e));
    }

    unsafe {
        let start_ptr = (start_addr as u64)
                as *mut c_ulonglong as *mut c_void;

        let protect_flags = PROT_READ|PROT_WRITE;
        let flags = MAP_PRIVATE | MAP_ANONYMOUS;

        let addr = mmap(start_ptr, gb*GB,
            protect_flags, flags, fd, 0);
        if addr as c_ulonglong == 0 {
            let e = format!("mmap failed:{}", Error::last_os_error());
            error!("{}", e);
            return Err(Error::new(ErrorKind::Other, e));
        }

        // write to address
        let ptr = addr as c_ulonglong as *mut u64;
        std::ptr::write_bytes(ptr, 0, 1);
    }

    info!("test ok!");
    Ok(())
}

pub unsafe fn map_1gb(start_addr: * mut c_void, numa_node: u32) -> Result<*mut c_void> {
    let gb = 1;
    let protect_flags = PROT_READ|PROT_WRITE;
    let flags = MAP_PRIVATE | MAP_ANONYMOUS|MAP_HUGE_1GB|MAP_HUGETLB;

    let start_ptr = start_addr as *mut c_ulonglong as *mut c_void;
    let addr = mmap(start_ptr, gb*GB,
        protect_flags, flags, -1, 0);
    if addr as c_ulonglong == 0 {
        let e = format!("mmap failed:{}", Error::last_os_error());
        return Err(Error::new(ErrorKind::Other, e));
    }

    let nmask = 1 << numa_node;
    let mode = MPOL_USED;
    let bflags = 0;
    let result = mbind(addr, (gb*GB) as c_ulong, mode,
    &nmask, 32, bflags);
    if result != 0 {
		munmap(addr, (gb*GB) as c_ulong);
        let e = format!("mbind failed:{}", Error::last_os_error());
        return Err(Error::new(ErrorKind::Other, e));
    }

    let result = mlock(addr, (gb*GB) as c_ulong);
    if result != 0 {
		munmap(addr, (gb*GB) as c_ulong);
        let e = format!("mlock failed:{}", Error::last_os_error());
        return Err(Error::new(ErrorKind::Other, e));
    }

    Ok(addr)
}

// fn main() -> std::io::Result<()> {
//     SimpleLogger::new().init().unwrap();
//      /*unsafe {
//          let nmask = 0x01;
//          let lr = set_mempolicy(MPOL_USED, &nmask, 32);
//          if lr != 0 {
//              error!("set_mempolicy failed:{}, nmask:{:#b}, error:{:?}",lr, nmask, std::io::Error::last_os_error());
//          } else {
//              info!("set_mempolicy OK, nmask:{:#b}, bind:{}",nmask, MPOL_USED);
//          }
//      }*/

//     unsafe {
//         let start_addr:c_ulonglong = 32 << 40; // 32TB
//         let pages = 10;
//         let mut got = 0;
//         for i in 0..pages {
//             let start_ptr = (start_addr + (i * GB) as u64)
//                     as *mut c_ulonglong as *mut c_void;
//             match map_1gb(start_ptr , 0) {
//                 Ok(addr) => {
//                     let gb = 1;
//                     let ptr = addr as c_ulonglong as *mut u64;
//                     // for _i in 1..gb {
//                     //     *ptr = 0x1;
//                     //     ptr = ptr.add(GB / std::mem::size_of::<u32>());
//                     // }
//                     std::ptr::write_bytes(ptr, 0, 1);

//                     info!("write to {} GB ok", gb);
//                     got = got + 1;
//                 },
//                 Err(e) => {
//                     error!("map_1gb failed:{}", e);
//                     break;
//                 },
//             }
//         }

//         while got < pages {
//             let start_ptr = (start_addr + (got * GB) as u64)
//                     as *mut c_ulonglong as *mut c_void;
//             match map_1gb(start_ptr , 1) {
//                 Ok(addr) => {
//                     let gb = 1;
//                     let ptr = addr as c_ulonglong as *mut u64;
//                     // for _i in 1..gb {
//                     //     *ptr = 0x1;
//                     //     ptr = ptr.add(GB / std::mem::size_of::<u32>());
//                     // }
//                     std::ptr::write_bytes(ptr, 0, 1);

//                     info!("write to {} GB ok", gb);
//                     got = got + 1;
//                 },
//                 Err(e) => {
//                     error!("map_1gb failed:{}", e);
//                     break;
//                 },
//             }
//         }
//     let mut buffer = String::new();
//     std::io::stdin().read_line(&mut buffer)?;

//     }

//     //let mut buffer = String::new();
//     //std::io::stdin().read_line(&mut buffer)?;

//     Ok(())
// }
