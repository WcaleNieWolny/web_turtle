use std::ops::Deref;
use std::ptr::NonNull;
use std::alloc::{alloc, Layout, handle_alloc_error};

pub struct StaticVec<T, const S: usize> {
    ptr: NonNull<T>,
}

impl<T, const S: usize> StaticVec<T, S> {
    pub fn new() -> Self {
        let layout = Layout::array::<T>(S).unwrap();
        let ptr = unsafe {
            //TODO: MEMSET
            alloc(layout)
        };
        let ptr = match NonNull::new(ptr as *mut T) {
            Some(p) => p,
            None => handle_alloc_error(layout),
        };

        Self {
            ptr
        }
    }
}

impl<T, const S: usize> Deref for StaticVec<T, S> {
    type Target = [T; S];

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.ptr.as_ptr() as *const [T; S];
            &*ptr
        }
    }
}

unsafe impl<T: Send, const S: usize> Send for StaticVec<T, S> {}
unsafe impl<T: Sync, const S: usize> Sync for StaticVec<T, S> {}
