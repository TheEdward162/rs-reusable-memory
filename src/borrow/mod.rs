use std::{
	borrow::{Borrow, BorrowMut},
	marker::PhantomData,
	mem,
	num::NonZeroUsize,
	ops::{Deref, DerefMut, RangeBounds},
	ptr
};

pub mod drain;
mod manual_specialization;

pub use drain::BorrowDrainIter;

#[derive(Debug, Copy, Clone)]
pub enum ReusableMemoryBorrowError {
	NotEnoughCapacity(NonZeroUsize)
}
impl std::fmt::Display for ReusableMemoryBorrowError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ReusableMemoryBorrowError::NotEnoughCapacity(capacity) => {
				write!(f, "Not enough capacity ({}) to push another element.", capacity)
			}
		}
	}
}
impl std::error::Error for ReusableMemoryBorrowError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { None }
}

/// Borrow of the reusable memory.
///
/// This struct borrows a properly aligned subset of the memory owned by `ReusableMemory`.
///
/// This structs semantically acts as `&'mem mut [T]` for variance, `Send` and `Sync` purposes.
pub struct ReusableMemoryBorrow<'mem, T> {
	// This could be `*mut T`, but we know it can't be null.
	memory: ptr::NonNull<T>,
	len: usize,
	capacity: NonZeroUsize,

	// We don't own the memory, we just mutably borrowed it.
	boo: PhantomData<&'mem mut [T]>
}
impl<'mem, T> ReusableMemoryBorrow<'mem, T> {
	/// Constructs memory borrow from raw parts.
	///
	/// ### Safety
	///
	/// * memory must be a valid pointer into `capacity * size_of::<T>()` bytes of memory.
	pub unsafe fn from_raw_parts(memory: ptr::NonNull<T>, capacity: NonZeroUsize) -> Self {
		ReusableMemoryBorrow { memory, len: 0, capacity, boo: PhantomData }
	}

	/// Returns number of `T`s currently stored.
	pub const fn len(&self) -> usize { self.len }

	/// Returns number of `T`s currently stored.
	pub unsafe fn set_len(&mut self, len: usize) { self.len = len; }

	/// Returns number of `T`s that can be stored.
	pub const fn capacity(&self) -> NonZeroUsize { self.capacity }

	/// Returns a const pointer to the data.
	pub const fn as_ptr(&self) -> *const T { self.memory.as_ptr() as *const _ }

	/// Returns a mut pointer to the data.
	pub const fn as_mut_ptr(&self) -> *mut T { self.memory.as_ptr() }

	/// Returns a slice view of the data.
	pub fn as_slice(&self) -> &[T] {
		unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
	}

	/// Returns a mut slice view of the data.
	pub fn as_mut_slice(&mut self) -> &mut [T] {
		unsafe { std::slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len()) }
	}

	/// Drops all pushed values and sets the length to 0.
	pub fn clear(&mut self) {
		if mem::needs_drop::<T>() {
			unsafe {
				let mut ptr = self.memory.as_ptr().add(self.len);
				let current_len = self.len;
				// Panic safety, rather leak than double-drop.
				// Vec uses internal `SetLenOnDrop` but this is okay too.
				self.len = 0;

				for _ in 0 .. current_len {
					ptr = ptr.offset(-1);
					ptr::drop_in_place(ptr);
				}
			}
		} else {
			self.len = 0;
		}
	}

	/// Pushes a new value.
	///
	/// Returns Err if there is not enough capacity.
	pub fn push(&mut self, value: T) -> Result<(), ReusableMemoryBorrowError> {
		if self.len == self.capacity.get() {
			return Err(ReusableMemoryBorrowError::NotEnoughCapacity(self.capacity))
		}

		unsafe {
			let dst = self.memory.as_ptr().add(self.len);
			ptr::write(dst, value);

			self.len += 1;
		}

		Ok(())
	}

	/// Pops from the end.
	///
	/// Returns `None` if `self.len() == 0`.
	pub fn pop(&mut self) -> Option<T> {
		if self.len() == 0 {
			return None
		}

		let value = unsafe {
			self.len -= 1;
			ptr::read(self.memory.as_ptr().add(self.len))
		};

		Some(value)
	}

	/// Creates a draining iterator that removes the specified range in the borrow and yields the removed items.
	///
	/// This functions exactly as `Vec::drain`.
	pub fn drain<'bor>(
		&'bor mut self, range: impl RangeBounds<usize>
	) -> BorrowDrainIter<'bor, 'mem, T> {
		BorrowDrainIter::new(self, range)
	}
}
impl<'mem, T> Deref for ReusableMemoryBorrow<'mem, T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target { self.as_slice() }
}
impl<'mem, T> DerefMut for ReusableMemoryBorrow<'mem, T> {
	fn deref_mut(&mut self) -> &mut Self::Target { self.as_mut_slice() }
}
impl<'mem, T> Borrow<[T]> for ReusableMemoryBorrow<'mem, T> {
	fn borrow(&self) -> &[T] { self.as_slice() }
}
impl<'mem, T> BorrowMut<[T]> for ReusableMemoryBorrow<'mem, T> {
	fn borrow_mut(&mut self) -> &mut [T] { self.as_mut_slice() }
}
impl<'mem, T> AsRef<[T]> for ReusableMemoryBorrow<'mem, T> {
	fn as_ref(&self) -> &[T] { self.as_slice() }
}
impl<'mem, T> AsMut<[T]> for ReusableMemoryBorrow<'mem, T> {
	fn as_mut(&mut self) -> &mut [T] { self.as_mut_slice() }
}
impl<'mem, T> Drop for ReusableMemoryBorrow<'mem, T> {
	fn drop(&mut self) { self.clear(); }
}
impl<'mem, T: std::fmt::Debug> std::fmt::Debug for ReusableMemoryBorrow<'mem, T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "[{}/{}] {:?}", self.len, self.capacity, self.as_slice())
	}
}
