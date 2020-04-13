//! To reuse memory, it needs to be allocated first:
//! ```
//! use reusable_memory::ReusableMemory;
//!
//! let mut memory: ReusableMemory<u8> = ReusableMemory::new();
//! // The memory can then be borrowed as a different type:
//! let mut borrowed_memory = memory.borrow_mut_as::<usize>(std::num::NonZeroUsize::new(3).unwrap());
//!
//! // Now `borrowed_memory` holds a pointer to enough memory to store 3 properly-aligned `usize`s inside the memory allocated in `memory`.
//! borrowed_memory.push(1).unwrap();
//! borrowed_memory.push(2).unwrap();
//! borrowed_memory.push(std::usize::MAX).unwrap();
//! ```
//! *`push` will return an `Err` if the pushed value would not fit into the capacity of the borrowed memory.*
//!
//! The borrowed memory is automatically returned when the object is dropped, and the pushed values are dropped as well.

mod base;
pub mod borrow;

pub use base::*;

#[cfg(test)]
mod tests {
	use std::num::NonZeroUsize;

	use super::{borrow::*, *};

	/// Tests borrow of `u8` from base of `u8`.
	#[test]
	fn same_type() {
		let mut rm: ReusableMemory<u16> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<u16>(NonZeroUsize::new(3).unwrap());
			borrow.push(1).unwrap();
			borrow.push(std::u16::MAX).unwrap();

			assert_eq!(borrow.as_ptr().align_offset(std::mem::align_of::<u16>()), 0);
			assert_eq!(borrow.len(), 2);
		}
	}

	/// Tests borrow of `i16` from base of `u16`.
	#[test]
	fn same_align_type() {
		let mut rm: ReusableMemory<u16> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<i16>(NonZeroUsize::new(3).unwrap());
			borrow.push(1).unwrap();
			borrow.push(std::i16::MAX).unwrap();

			assert_eq!(borrow.as_ptr().align_offset(std::mem::align_of::<i16>()), 0);
			assert_eq!(borrow.len(), 2);
		}
	}

	/// Tests borrow of `usize` from base of `u8`.
	///
	/// This fails on Miri because it cannot align the pointers (yet?)
	#[test]
	fn different_align() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<usize>(NonZeroUsize::new(3).unwrap());
			borrow.push(1).unwrap();
			borrow.push(std::usize::MAX).unwrap();

			assert_eq!(borrow.as_ptr().align_offset(std::mem::align_of::<usize>()), 0);
			assert_eq!(borrow.len(), 2);
		}
	}

	#[test]
	fn borrow_two_same_type() {
		let mut rm: ReusableMemory<u16> = ReusableMemory::new();
		{
			let (mut borrow_a, mut borrow_b) = rm.borrow_mut_two_as::<u16, i16>([
				NonZeroUsize::new(6).unwrap(),
				NonZeroUsize::new(3).unwrap()
			]);

			borrow_a.push(1).unwrap();
			borrow_a.push(2).unwrap();
			borrow_a.push(std::u16::MAX).unwrap();

			borrow_b.push(-1).unwrap();
			borrow_b.push(-2).unwrap();
			borrow_b.push(std::i16::MIN).unwrap();

			assert_eq!(borrow_a.as_ptr().align_offset(std::mem::align_of::<u16>()), 0);
			assert_eq!(borrow_b.as_ptr().align_offset(std::mem::align_of::<i16>()), 0);

			assert_eq!(borrow_a.as_slice(), &[1, 2, std::u16::MAX]);
			assert_eq!(borrow_b.as_slice(), &[-1, -2, std::i16::MIN]);
		}
	}

	/// Tests borrow of `u64`,`u32` and `u16` from base of `u8`.
	///
	/// This fails on Miri because it cannot align the pointers (yet?)
	#[test]
	fn borrow_three_different_align() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let (mut borrow_u64, mut borrow_u32, mut borrow_u16) = rm
				.borrow_mut_three_as::<u64, u32, u16>([
					NonZeroUsize::new(1).unwrap(),
					NonZeroUsize::new(2).unwrap(),
					NonZeroUsize::new(4).unwrap()
				]);

			borrow_u64.push(1).unwrap();

			borrow_u32.push(1).unwrap();
			borrow_u32.push(2).unwrap();

			borrow_u16.push(1).unwrap();
			borrow_u16.push(2).unwrap();
			borrow_u16.push(3).unwrap();
			borrow_u16.push(4).unwrap();

			assert_eq!(borrow_u64.as_ptr().align_offset(std::mem::align_of::<u64>()), 0);
			assert_eq!(borrow_u32.as_ptr().align_offset(std::mem::align_of::<u32>()), 0);
			assert_eq!(borrow_u16.as_ptr().align_offset(std::mem::align_of::<u16>()), 0);

			assert_eq!(borrow_u64.as_slice(), &[1u64]);
			assert_eq!(borrow_u32.as_slice(), &[1u32, 2u32]);
			assert_eq!(borrow_u16.as_slice(), &[1u16, 2u16, 3u16, 4u16]);
		}
	}

	#[test]
	fn push_iter() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<u8>(NonZeroUsize::new(6).unwrap());
			let iter = (0 .. 5u8).into_iter();

			borrow.push_from_iter(iter).unwrap();
			assert_eq!(borrow.as_slice(), &[0, 1, 2, 3, 4]);
		}
	}

	#[test]
	fn push_iter_fill_up() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<u8>(NonZeroUsize::new(5).unwrap());
			let iter = (0 .. 5u8).into_iter();

			let mut iter = borrow.push_from_iter(iter).unwrap_err();
			assert_eq!(borrow.as_slice(), &[0, 1, 2, 3, 4]);
			assert_eq!(iter.next(), None);
		}
	}

	#[test]
	fn push_iter_fill_up_peekable() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<u8>(NonZeroUsize::new(5).unwrap());
			let iter = (0 .. 5u8).into_iter();

			borrow.push_from_iter_peeking(iter).unwrap();
			assert_eq!(borrow.as_slice(), &[0, 1, 2, 3, 4]);
		}
	}

	#[test]
	fn push_iter_over_capacity() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<u8>(NonZeroUsize::new(4).unwrap());
			let iter = (0 .. 5u8).into_iter();

			let mut iter = borrow.push_from_iter(iter).unwrap_err();
			assert_eq!(borrow.as_slice(), &[0, 1, 2, 3]);
			assert_eq!(iter.next(), Some(4));
		}
	}

	/// Tests that borrow can push from ExactSizeIterator.
	#[test]
	fn push_exact_iter() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<u8>(NonZeroUsize::new(3).unwrap());
			let iter = vec![1, std::u8::MAX].into_iter();

			borrow.push_from_exact_iter(iter).unwrap();
			assert_eq!(borrow.as_ptr().align_offset(std::mem::align_of::<u8>()), 0);
			assert_eq!(borrow.len(), 2);
		}
	}

	/// Tests that pushing from an iterator beyond capacity returns an error.
	#[test]
	fn push_exact_iter_over_capacity() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let capacity = NonZeroUsize::new(1).unwrap();
			let mut borrow = rm.borrow_mut_as::<u8>(capacity);
			let iter = vec![1, std::u8::MAX].into_iter();

			match borrow.push_from_exact_iter(iter.clone()) {
				Err(err_iter) if err_iter.as_slice() == iter.as_slice() => (),
				_ => panic!("Expected Err(iter)")
			}
		}
	}

	#[test]
	fn pop() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let capacity = NonZeroUsize::new(1).unwrap();
			let mut borrow = rm.borrow_mut_as::<u8>(capacity);

			borrow.push(1).unwrap();

			assert_eq!(borrow.pop(), Some(1));
			assert_eq!(borrow.pop(), None);
		}
	}

	#[test]
	fn drain() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<u8>(NonZeroUsize::new(5).unwrap());
			borrow.push_from_exact_iter(0 ..= 4).unwrap();

			{
				let mut drain = borrow.drain(1 ..= 3);
				assert_eq!(drain.next(), Some(1));
				assert_eq!(drain.next_back(), Some(3));
			}

			assert_eq!(borrow.as_slice(), &[0, 4]);
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

		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<DropCounter>(NonZeroUsize::new(2).unwrap());

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

		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let mut borrow = rm.borrow_mut_as::<DropCounter>(NonZeroUsize::new(2).unwrap());

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
		let mut rm: ReusableMemory<u8> = ReusableMemory::new();
		{
			let capacity = NonZeroUsize::new(1).unwrap();
			let mut borrow = rm.borrow_mut_as::<u8>(capacity);
			borrow.push(1).unwrap();

			match borrow.push(1) {
				Err(ReusableMemoryBorrowError::NotEnoughCapacity(c)) if c == capacity => (),
				_ => panic!("Expected Err(ReusableMemoryBorrowError::NotEnoughCapacity)")
			}
		}
	}
}
