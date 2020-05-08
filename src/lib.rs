

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash,Hasher};
    
    #[test]
    fn it_works() {
	let mut har = HeapArray::allocate(3);
	har[0] = 1u8;
	har[1] = 2u8;
	har[2] = 3u8;

	let har2 = har.clone();

	assert_eq!(har, har2);

	println!("{}", har);
	assert_eq!(format!("{}", har), format!("{}", har2));
	assert_eq!(har[1..], [2,3]);

	let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
	let mut hasher2 = std::collections::hash_map::DefaultHasher::new();

	har.hash(&mut hasher1);
	har2.hash(&mut hasher2);

	assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn macro_works()
    {
	let ha = heap![5u8; 10];
	assert_eq!(ha.len(), 10);
	assert_eq!(ha[1], 5);

	let mut ha2 = heap![u8; ha.len()];
	ha2.fill_bytes(5);

	assert_eq!(ha, ha2);

	let ha3 = heap![5,5,5,5,5,5,5,5,5,5];
	assert_eq!(ha3[2], 5u8);
	assert_eq!(ha3, ha2);

	assert_eq!(ha2.into_vec(), ha3.into_vec());
    }

    #[test]
    fn to_box()
    {
	let mut ha = heap![u8; 10];
	{
	    ha[1] = 22;
	    let mut bx = ha.into_box();
	    bx[0] = 10;

	    assert_eq!(bx.len(), 10);
	    assert_eq!(bx[0], 10);
	    assert_eq!(bx[1], 22);
	}
    }
}

use std::ops::{Index,IndexMut};
use std::slice::SliceIndex;

#[derive(Debug)]
/// An simple array on the heap, manages by `calloc()` and `free()`s on Drop from `libc`.
///
/// Essentially a `Vec<T>` that cannot change size.
pub struct HeapArray<T> {
    ptr: *mut T,
    count: usize,
}
fn copy_slice<T: Clone>(dst: &mut [T], src: &[T])
{
    for (x, y) in dst.iter_mut().zip(src) {
	*x = y.clone()
    }
}

#[macro_export]
/// Count arg list.
macro_rules! count_args {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + $crate::count_args!($($xs)*));
}

#[macro_export]
/// Create a heap array, using the same syntax as array definitions or vec![].
///
/// # Example
/// ```rust
/// let array = heaparray::heap![u8; 10]; // Creates a 10 element HeapArray<u8>
/// let array2 = heaparray::heap![5; 10]; // Creates a 10 element HeapArray where each element is 5
/// assert_eq!(5i32, array2[9]);
/// let array3 = heaparray::heap![1,2,3]; // Create a 3 element HeapArray where the elements are [1, 2, 3]
/// ```
macro_rules! heap {
    ($type:ty; $number:expr) => {
	{
	    $crate::HeapArray::<$type>::allocate($number)
	}
    };
    ($value:expr; $number:expr) => {
	{
	    let mut ha = $crate::HeapArray::allocate($number);
	    for x in 0..$number {
		ha[x] = $value;
	    }
	    ha
	}
    };
    ($($n:expr),*) => {
	{
	    let mut ha = $crate::HeapArray::allocate($crate::count_args!($($n)*));
	    {
		let mut fp = 0;
		$(
		    fp+=1;
		    ha[fp-1] = $n;
		)*
	    }
	    ha
	}
    }
}

impl<T> HeapArray<T> {
    /// Size of `T`
    pub const fn element_size() -> usize
    {
	std::mem::size_of::<T>()
    }

    /// Allocate a new `HeapArray<T>` with `size` elements.
    pub fn allocate(size: usize) -> Self {
	assert!(size>0);
	unsafe {
	    Self {
		count: size,
		ptr: libc::calloc(size, std::mem::size_of::<T>()) as *mut T,
	    }
	}
    }

    /// Return `HeapArray<T>` as an iterator.
    pub fn iter<'a>(&'a self) -> Iter<'a, T>
    {
	self.into_iter()
    }

    /// As a mutable slice of all bytes in the array.
    pub fn as_bytes_mut(&self) -> &mut [u8]
    {
	unsafe {
	    std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.count*std::mem::size_of::<T>())
	}
    }
    
    /// As a slice of all bytes in the array.
    pub fn as_bytes(&self) -> &[u8]
    {
	unsafe {
	    std::slice::from_raw_parts(self.ptr as *const u8, self.count*std::mem::size_of::<T>())
	}
    }

    /// A `null` pointer as `HeapArray<T>` that will not free on drop. (unsafe)
    pub unsafe fn null() -> Self {
	Self {
	    ptr: 0 as *mut T,
	    count: 0
	}
    }

    /// Consumes the `HeapArray<T>` and returns a boxed slice that now owns the data.
    pub fn into_box(mut self) -> Box<[T]>
    {
	unsafe {
	    let bx = Box::from_raw(self.as_mut() as *mut [T]);
	    std::mem::forget(self);
	    bx
	}
    }

    /// Is this instance referencing a `null` pointer?
    pub fn is_null(&self) -> bool {
	self.ptr == 0 as *mut T
    }

    /// Reallocate this `HeapArray<T>` to a new size.
    pub fn reallocate(&mut self, size: usize) -> &mut Self {
	assert!(size>0);
	unsafe {
	    self.ptr = libc::realloc(self.ptr as *mut core::ffi::c_void, size) as *mut T;
	}
	self.count=size;
	self
    }

    /// Compare the equality of the memory block this `HeapArray<T>` and another point to.
    pub fn mem_eq(&self, other: &Self) -> bool
    {
	unsafe {
	    self.count == other.count &&
		(self.ptr == other.ptr || //this should never be the case, but it could be if unsafe fns are used
		 libc::memcmp(self.ptr as *mut core::ffi::c_void, other.ptr as *mut core::ffi::c_void, self.count)==0)
	}
    }

    /// Memory block as a const pointer.
    pub fn as_ptr(&self) -> *const T
    { 
	self.ptr as *const T
    }

    /// Memory block as a mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut T
    {
	self.ptr
    }

    /// Fill this `HeapArray<T>` with a value.
    pub fn fill(&mut self, value: &T)
    where T: Clone
    {
	if std::mem::size_of::<T>() == std::mem::size_of::<u8>() {
	    unsafe { //single byte, use memset.
		let tm: u8 = std::mem::transmute_copy(value);
		libc::memset(self.ptr as *mut core::ffi::c_void, tm as i32, self.count);
	    }
	} else {
	    for i in 0..self.count {
		self[i] = value.clone()
	    }
	}
    }

    /// Create a new `HeapArray<T>` from a `Vec<T>`.
    pub fn from_vec(vec: Vec<T>) -> Self {
	let mut this = Self::allocate(vec.len());
	let mut i=0;
	for x in vec.into_iter() {
	    this[i] = x;
	    i+=1;
	}
	this
    }

    /// Create a new `HeapArray<T>` from a raw pointer and size.
    pub unsafe fn from_raw(ptr: *mut T, size: usize) -> Self {
	Self {
	    ptr: ptr,
	    count: size,
	}
    }

    /// As a slice of `T`
    pub fn as_slice(&self) -> &[T]
    {
	unsafe {
	    std::slice::from_raw_parts(self.ptr, self.count)
	}
    }

    /// As a mutable slice of `T`
    pub fn as_mut(&mut self) -> &mut [T]
    {
	unsafe {
	    std::slice::from_raw_parts_mut(self.ptr, self.count)
	}	
    }

    /// Fill the memory block this `HeapArray<T>` points to with a single byte.
    pub fn fill_bytes(&mut self, value: u8)
    {
	unsafe {
	    libc::memset(self.ptr as *mut core::ffi::c_void, value as i32, self.count);
	}	
    }

    /// Clone this `HeapArray<T>` into a `Vec<T>`
    pub fn to_vec(&self) -> Vec<T>
    where T: Clone
    {
	let mut v = Vec::with_capacity(self.count);
	v.extend_from_slice(self.as_slice());
	v
    }

    /// Consumes the `HeapArray<T>`, and returns a `Vec<T>` containing all the elements.
    pub fn into_vec(mut self) -> Vec<T>
    {
	let mut v = Vec::with_capacity(self.count);
	for i in 0..self.count {
	    let x = unsafe { std::mem::replace(&mut self[i], std::mem::MaybeUninit::zeroed().assume_init())};
	    v.push(x);
	}
	v
    }

    /// Return the amount of elements this `HeapArray<T>` can hold.
    pub fn len(&self) -> usize {
	self.count
    }
    /// Returns the full size of the allocated memory block.
    pub fn len_bytes(&self) -> usize {
	Self::element_size() * self.count
    }

    /// Clone the memory block into a new `HeapArray<T>`
    pub fn clone_mem(&self) -> Self
    {
	let mut this = Self::allocate(self.count);
	unsafe {
	    libc::memcpy(this.as_mut_ptr() as *mut core::ffi::c_void, self.as_ptr() as *mut core::ffi::c_void, self.count*std::mem::size_of::<T>());
	}
	this
    }
}

impl<T> Drop for HeapArray<T> {
    fn drop(&mut self)
    {
	if !self.is_null() {
	    unsafe {
		libc::free(self.ptr as *mut core::ffi::c_void);
	    }
	}
    }
    
}

impl<T> AsRef<[T]> for HeapArray<T>
{
    fn as_ref(&self) -> &[T]
    {
	unsafe {
	    std::slice::from_raw_parts(self.ptr, self.count)
	}
    }
}

impl<T> AsMut<[T]> for HeapArray<T>
{
    fn as_mut(&mut self) -> &mut [T]
    {
	unsafe {
	    std::slice::from_raw_parts_mut(self.ptr, self.count)
	}
    }
}

impl<T> std::borrow::Borrow<[T]> for HeapArray<T>
{
    fn borrow(&self) -> &[T]
    {
	unsafe {
	    std::slice::from_raw_parts(self.ptr, self.count)
	}
    }
}

impl<T> std::borrow::BorrowMut<[T]> for HeapArray<T>
{
    fn borrow_mut(&mut self) -> &mut [T]
    {
	unsafe {
	    std::slice::from_raw_parts_mut(self.ptr, self.count)
	}
    }
}

impl<T, I> Index<I> for HeapArray<T>
where I: SliceIndex<[T]>
{
    type Output = <I as SliceIndex<[T]>>::Output;
    fn index(&self, index: I) -> &Self::Output
    {
	&self.as_slice()[index]
    }
}


impl<T, I> IndexMut<I> for HeapArray<T>
where I: SliceIndex<[T]>
{
    fn index_mut(&mut self, index: I) -> &mut <Self as Index<I>>::Output
    {
	&mut self.as_mut()[index]
    }
}

/// Iterator for `HeapArray<T>`
pub struct Iter<'a, T>(&'a HeapArray<T>, usize);

impl<'a, T> Iter<'a, T> {
    fn new(ha: &'a HeapArray<T>) -> Self {
	Self(ha, 0)
    }
}

impl<'a, T> Iterator for Iter<'a, T>
{
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item>
    {
	if self.1 < self.0.len() {
	    let result = &self.0[self.1]; 
	    self.1 += 1;
	    Some(result)
	} else {
	    None
	}
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T>
{
    fn next_back(&mut self) -> Option<<Self as Iterator>::Item>
    {
	if self.1 > 0 {
	    self.1 -= 1;
	    Some(&self.0[self.1-1])
	} else {
	    None
	}
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T>
{
    fn len(&self) -> usize {
	self.0.len()
    }
}

impl<'a, T> IntoIterator for &'a HeapArray<T>
{
    type Item = &'a T;
    type IntoIter = Iter<'a , T>;

    fn into_iter(self) -> Iter<'a, T>
    {
	Iter::new(self)
    }
}
impl<T> Clone for HeapArray<T>
where T: Clone
{
    fn clone(&self) -> Self {
	let mut this = Self::allocate(self.count);
	copy_slice(this.as_mut(), self.as_slice());
	this
    }
}

impl<T> std::cmp::Eq for HeapArray<T>
where T: std::cmp::Eq{}
impl<T> std::cmp::PartialEq for HeapArray<T>
where T: std::cmp::PartialEq
{
    fn eq(&self, other: &Self) -> bool
    {
	self.count == other.count &&
	    (self.ptr == other.ptr
	     || {
		 for (x, y) in self.iter().zip(other.iter()) {
		     if x != y { return false; }
		 }
		 true
	     })
    }
}
fn to_hex_string(bytes: &[u8]) -> String {
    let strs: Vec<String> = bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    strs.join("")
}

impl<T> std::fmt::Display for HeapArray<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
	
	write!(f, "[{}x{} ({}): {}]", self.count, Self::element_size(), Self::element_size()*self.count, to_hex_string(self.as_bytes()))
    }
}

impl<T> std::hash::Hash for HeapArray<T>
where T: std::hash::Hash
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
	self.count.hash(state);
	self.as_slice().hash(state);
    }
}
