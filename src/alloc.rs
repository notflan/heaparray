struct HeapAlloc {
    ptr: *mut (),
    size: usize,
}

impl HeapAlloc {
    pub fn allocate(size: usize) -> Self {
	
    }
}

impl Drop for HeapAlloc {
    fn drop(&self)
    {
	libc::free(self.ptr);
    }
}
