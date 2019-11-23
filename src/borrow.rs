use std::{iter::ExactSizeIterator, marker::PhantomData, mem, num::NonZeroUsize, ptr};

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
	pub(crate) fn new(memory: ptr::NonNull<T>, capacity: NonZeroUsize) -> Self {
		ReusableMemoryBorrow { memory, len: 0, capacity, boo: PhantomData }
	}

	/// Returns number of `T`s currently stored.
	pub fn len(&self) -> usize { self.len }

	/// Returns number of `T`s that can be stored.
	pub fn capacity(&self) -> NonZeroUsize { self.capacity }

	/// Returns a pointer to the data.
	pub fn as_ptr(&self) -> *const T { self.memory.as_ptr() as *const T }

	/// Returns a slice view of the data.
	pub fn as_slice(&self) -> &[T] {
		unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len()) }
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

	/// Pushes new values from `ExactSizeIterator`.
	///
	/// Returns Err if there is not enough capacity.
	pub fn push_from_exact_iter<E: ExactSizeIterator<Item = T>>(
		&mut self, iter: E
	) -> Result<(), ReusableMemoryBorrowError> {
		if self.len + iter.len() > self.capacity.get() {
			return Err(ReusableMemoryBorrowError::NotEnoughCapacity(self.capacity))
		}

		for elem in iter {
			self.push(elem).unwrap()
		}

		Ok(())
	}
}
impl<'mem, T> Drop for ReusableMemoryBorrow<'mem, T> {
	fn drop(&mut self) { self.clear(); }
}
impl<'mem, T: std::fmt::Debug> std::fmt::Debug for ReusableMemoryBorrow<'mem, T> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "[{}/{}] {:?}", self.len, self.capacity, self.as_slice())
	}
}
