mod c_str;

pub use c_str::{CStr, CString};

mod memchr {
    pub fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
        unsafe {
            let s = haystack.as_ptr() as *const _;
            let result = ::sys::memchr(s, needle.into(), haystack.len());
            if result.is_null() {
                None
            } else {
                Some(result as usize - s as usize)
            }
        }
    }
}

mod sys {
    use std::os::raw::{c_char, c_int, c_void};
    extern {
        pub fn strlen(ptr: *const c_char) -> usize;
        pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    }
}
