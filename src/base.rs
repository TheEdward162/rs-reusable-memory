use std::{num::NonZeroUsize, mem, ptr};

use super::ReusableMemoryBorrow;

#[derive(Debug, Copy, Clone)]
pub enum ReusableMemoryError {
	ZeroSizedB,
	ZeroSizedT,
	/// Pointer to `B` could not be aligned to a pointer to `T`.
	///
	/// This error should never happen, since the pointer to `B` is provided by a `Vec` allocation
	/// and should be properly aligned. A properly aligned pointer will always be alignable to other
	/// power-of-two aligns.
	CouldNotAlignPointer
}
impl std::fmt::Display for ReusableMemoryError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ReusableMemoryError::ZeroSizedB => write!(f, "Type B (base type) must not be zero sized."),
			ReusableMemoryError::ZeroSizedT => write!(f, "Type T (borrowed type) must not be zero sized."),
			ReusableMemoryError::CouldNotAlignPointer => write!(f, "Could not align pointer to be to a pointer to T.")
		}
	}
}
impl std::error::Error for ReusableMemoryError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		None
	}
}

/// Reusable memory struct.
///
/// This struct keeps previously allocated memory and can mutably reborrow it as a different type on demand.
///
/// The generic type `B` can be used to control the alignment of the base memory, but it must not be zero sized.
/// Using a zero sized `B` returns an error in constructor.
#[derive(Debug, Clone)]
pub struct ReusableMemory<B = u8> {
	vec: Vec<B>
}
impl<B> ReusableMemory<B> {
	pub fn new() -> Result<Self, ReusableMemoryError> {
		Self::with_capacity(0)
	}

	/// Counted in the capacity of `B`.
	pub fn with_capacity(len: usize) -> Result<Self, ReusableMemoryError> {
		if mem::size_of::<B>() == 0 {
			return Err(
				ReusableMemoryError::ZeroSizedB
			)
		}

		Ok(
			ReusableMemory {
				vec: Vec::with_capacity(len)
			}
		)
	}

	/// Borrows the reusable memory as a different type.
	///
	/// This borrow is properly aligned and has at least the requested capacity.
	///
	/// Returns an error if `size_of::<T>() == 0`.
	/// Also returns an error when the pointer could not be aligned properly for `T`.
	pub fn borrow_mut_as<'mem, T>(&'mem mut self, capacity: NonZeroUsize) -> Result<ReusableMemoryBorrow<'mem, T>, ReusableMemoryError> {
		if mem::size_of::<T>() == 0 {
			return Err(
				ReusableMemoryError::ZeroSizedT
			)
		}
		
		// Needed length in bytes.
	    let needed_length = mem::size_of::<T>() * capacity.get() + mem::align_of::<T>();
		// Needed length divided by the size of `B`.
		let divided_length = needed_length / mem::size_of::<B>();

		// In case size of `T` is not divisible by the size of `B`, we need to take the least whole integer
		// greater than `size_of(T) / size_of(B)`.
		let reserved_length = if needed_length % mem::size_of::<B>() != 0 {
			divided_length + 1
		} else {
			divided_length
		};

		// Reserve so at least `capacity` of `T`s fit, plus possible align offset.
		self.vec.reserve(
	        reserved_length
	    );

		let memory_ptr = self.vec.as_mut_ptr();
		// Compute the offset we need from the vec pointer to have the proper alignment.
		let align_offset = memory_ptr.align_offset(
			mem::align_of::<T>()
		);
		if align_offset == std::usize::MAX {
			return Err(
				ReusableMemoryError::CouldNotAlignPointer
			)
		}

		let memory_ptr = unsafe {
			ptr::NonNull::new_unchecked(
				memory_ptr.add(align_offset) as *mut T
			)
		};

		Ok(
			ReusableMemoryBorrow::new(
				memory_ptr,
				capacity
			)
		)
	}
}