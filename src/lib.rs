//! To reuse memory, it needs to be allocated first:
//! ```
//! let mut memory: ReusableMemory<u8> = ReusableMemory::new().unwrap(); 
//! ```
//! *`new` will return an `Err` if the generic type passed to `ReusableMemory` is a zero sized type.*
//!
//! The memory can then be borrowed as a different type:
//! ```
//! let mut borrowed_memory = memory.borrow_mut_as::<usize>(NonZeroUsize::new(3).unwrap()).unwrap();
//! ```
//! *`borrow_mut_as` will return an `Err` if the generic type passed is a zero sized type.*
//!
//! Now `borrowed_memory` holds a pointer to enough memory to store 3 properly-aligned `usize`s inside the memory allocated in `memory`.
//! ```
//! borrowed_memory.push(1).unwrap();
//! borrowed_memory.push(2).unwrap();
//! borrowed_memory.push(std::usize::MAX).unwrap();
//! ```
//! *`push` will return an `Err` if the pushed value would not fit into the capacity of the borrowed memory.*
//!
//! The borrowed memory is automatically returned when the object is dropped, and the pushed values are dropped as well.

mod base;
mod borrow;

pub use base::*;
pub use borrow::*;

#[cfg(test)]
mod tests {
	use std::num::NonZeroUsize;

	use super::*;

	/// Tests borrow of `u8` from base of `u8`.
	#[test]
	fn same_type() {
		let mut rm: ReusableMemory<u16> = ReusableMemory::new().unwrap();
		{
			let mut borrow = rm.borrow_mut_as::<u16>(NonZeroUsize::new(3).unwrap()).unwrap();
			borrow.push(1).unwrap();
			borrow.push(std::u16::MAX).unwrap();

			assert_eq!(
				borrow.as_ptr().align_offset(std::mem::align_of::<u16>()), 0
			);
			assert_eq!(
				borrow.len(), 2
			);
		}
	}

	/// Tests borrow of `i16` from base of `u16`.
	#[test]
	fn same_align_type() {
		let mut rm: ReusableMemory<u16> = ReusableMemory::new().unwrap();
		{
			let mut borrow = rm.borrow_mut_as::<i16>(NonZeroUsize::new(3).unwrap()).unwrap();
			borrow.push(1).unwrap();
			borrow.push(std::i16::MAX).unwrap();

			assert_eq!(borrow.as_ptr().align_offset(std::mem::align_of::<i16>()), 0);
			assert_eq!(
				borrow.len(), 2
			);
		}
	}

	/// Tests borrow of `usize` from base of `u8`.
	///
	/// This fails on Miri because it cannot align the pointers (yet?)
	#[test]
	fn different_align() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let mut borrow = rm.borrow_mut_as::<usize>(NonZeroUsize::new(3).unwrap()).unwrap();
			borrow.push(1).unwrap();
			borrow.push(std::usize::MAX).unwrap();

			assert_eq!(borrow.as_ptr().align_offset(std::mem::align_of::<usize>()), 0);
			assert_eq!(
				borrow.len(), 2
			);
		}
	}

	/// Tests that borrow can push from ExactSizeIterator.
	#[test]
	fn iter() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let mut borrow = rm.borrow_mut_as::<u8>(NonZeroUsize::new(3).unwrap()).unwrap();
			let iter = vec![1, std::u8::MAX].into_iter();

			borrow.push_from_exact_iter(iter).unwrap();
			assert_eq!(
				borrow.as_ptr().align_offset(std::mem::align_of::<u8>()), 0
			);
			assert_eq!(
				borrow.len(), 2
			);
		}
	}

	/// Tests that pushing from an iterator beyond capacity returns an error.
	#[test]
	fn iter_over_capacity() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let capacity = NonZeroUsize::new(1).unwrap();
			let mut borrow = rm.borrow_mut_as::<u8>(capacity).unwrap();
			let iter = vec![1, std::u8::MAX].into_iter();

			match borrow.push_from_exact_iter(iter) {
				Err(ReusableMemoryBorrowError::NotEnoughCapacity(c)) if c == capacity => (),
				_ => panic!("Expected Err(ReusableMemoryBorrowError::NotEnoughCapacity)")
			}
		}
	}

	/// Tests that values are dropped on clear.
	#[test]
	fn clear() {
		static mut DROP_COUNTER: usize = 0;
		struct DropCounter {
			_value: u8
		}
		impl DropCounter {
			pub fn new(value: u8) -> Self {
				unsafe {
					DROP_COUNTER += 1;
				}

				DropCounter { _value: value }
			}
		}
		impl Drop for DropCounter {
			fn drop(&mut self) {
				unsafe {
					DROP_COUNTER -= 1;
				}
			}
		}

		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let mut borrow =
				rm.borrow_mut_as::<DropCounter>(NonZeroUsize::new(2).unwrap()).unwrap();

			borrow.push(DropCounter::new(1)).unwrap();
			borrow.push(DropCounter::new(std::u8::MAX)).unwrap();

			unsafe {
				assert_eq!(DROP_COUNTER, 2);
			}

			borrow.clear();

			unsafe {
				assert_eq!(DROP_COUNTER, 0);
			}
		}
	}

	/// Tests that values are dropped on `Drop`.
	#[test]
	fn drop() {
		static mut DROP_COUNTER: usize = 0;
		struct DropCounter {
			_value: u8
		}
		impl DropCounter {
			pub fn new(value: u8) -> Self {
				unsafe {
					DROP_COUNTER += 1;
				}

				DropCounter { _value: value }
			}
		}
		impl Drop for DropCounter {
			fn drop(&mut self) {
				unsafe {
					DROP_COUNTER -= 1;
				}
			}
		}

		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let mut borrow =
				rm.borrow_mut_as::<DropCounter>(NonZeroUsize::new(2).unwrap()).unwrap();

			borrow.push(DropCounter::new(1)).unwrap();
			borrow.push(DropCounter::new(std::u8::MAX)).unwrap();

			unsafe {
				assert_eq!(DROP_COUNTER, 2);
			}
		}

		unsafe {
			assert_eq!(DROP_COUNTER, 0);
		}
	}

	/// Tests that pushing beyond capacity returns an error.
	#[test]
	fn not_enough_capacity() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let capacity = NonZeroUsize::new(1).unwrap();
			let mut borrow = rm.borrow_mut_as::<u8>(capacity).unwrap();
			borrow.push(1).unwrap();

			match borrow.push(1) {
				Err(ReusableMemoryBorrowError::NotEnoughCapacity(c)) if c == capacity => (),
				_ => panic!("Expected Err(ReusableMemoryBorrowError::NotEnoughCapacity)")
			}
		}
	}

	/// Tests that creating base with zero sized type returns an error.
	#[test]
	fn zero_sized_base() {
		let rm: Result<ReusableMemory<()>, ReusableMemoryError> = ReusableMemory::new();
		match rm {
			Err(ReusableMemoryError::ZeroSizedB) => (),
			_ => panic!("Expected Err(ReusableMemoryError::ZeroSizedB)")
		}
	}

	/// Tests that creating borrow with zero sized type return an error.
	#[test]
	fn zero_sized_t() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let borrow = rm.borrow_mut_as::<()>(NonZeroUsize::new(1).unwrap());
			match borrow {
				Err(ReusableMemoryError::ZeroSizedT) => (),
				_ => panic!("Expected Err(ReusableMemoryError::ZeroSizedT)")
			}
		}
	}
}
