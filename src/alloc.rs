use core::ffi::c_void;
use std::mem::{transmute,replace, MaybeUninit};

#[cfg(test)]
mod tests
{
    use super::*;
    #[test]
    fn cbox()
    {
	let v = 10i32;
	let c = on_heap(v);
	assert_eq!(c.extract(), 10);
    }
}

pub struct CBox<T>(*mut T);

pub fn on_heap<T>(value: T) -> CBox<T>
{
    CBox::new(value)
}

pub fn off_heap<T>(ptr: *mut T) 
{
    unsafe {
	
	libc::free(ptr as *mut c_void);
    }
}

impl<T> CBox<T>
{
    pub const fn inner_size() -> usize {
	std::mem::size_of::<T>()
    }
    pub fn new(mut value: T) -> Self {
	unsafe {
	    let ptr = libc::malloc(Self::inner_size());
	    assert!(!ptr.is_null());
	    libc::memcpy(ptr, &mut value as *mut T as *mut c_void, Self::inner_size());
	    
	    Self(ptr as *mut T)
	}
    }

    pub fn leak(self) -> &'static mut T
    {
	Box::leak(self.to_box())
    }

    pub fn as_ptr(&self) -> *const T
    {
	self.0 as *const T
    }
    pub fn as_mut_ptr(&mut self) -> *mut T
    {
	self.0
    }
    pub fn extract(mut self) -> T
    {
	unsafe {
	    replace(self.as_mut(), MaybeUninit::zeroed().assume_init())
	}
    }
    pub fn as_value(&self) -> &T
    {
	unsafe {
	    transmute::<*mut T, &T>(self.0)
	}
    }
    pub fn as_mut(&mut self) -> &mut T
    {
	unsafe {
	    transmute::<*mut T, &mut T>(self.0)
	}
    }

    pub fn get_value(&self) -> T
    where T: Clone {
	self.as_value().clone()
    }

    pub fn to_box(self) -> Box<T>
    {
	unsafe {
	    Box::from_raw(self.0)
	}
    }
    pub fn from_box(bx: Box<T>) -> Self
    {
	let value = *bx;
	Self::new(value)
    }

    pub unsafe fn from_raw(raw: *mut T) -> Self
    {
	Self::new(*Box::from_raw(raw))
    }
}

impl<T> Drop for CBox<T>
{
    fn drop(&mut self)
    {
	unsafe {
	    if !self.0.is_null() {
		libc::free(self.0 as *mut c_void);
	    }
	}
    }
}
