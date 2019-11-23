mod base;
mod borrow;

pub use base::*;
pub use borrow::*;

#[cfg(test)]
mod tests {
	use std::num::NonZeroUsize;

	use super::*;

	#[test]
	fn push_twice_usize() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let mut borrow = rm.borrow_mut_as::<usize>(
				NonZeroUsize::new(3).unwrap()
			).unwrap();
			borrow.push(1).unwrap();
			borrow.push(18446744073709551615).unwrap();

			// eprintln!("{:?}", borrow);

			assert_eq!(
				borrow.as_ptr().align_offset(std::mem::align_of::<usize>()), 0
			);
		}
	}

	#[test]
	fn clear() {
		static mut DROP_COUNTER: usize = 0;
		struct DropCounter {
			_value: usize
		}
		impl DropCounter {
			pub fn new(value: usize) -> Self {
				unsafe {
					DROP_COUNTER += 1;
				}

				DropCounter {
					_value: value
				}
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
			let mut borrow = rm.borrow_mut_as::<DropCounter>(
				NonZeroUsize::new(2).unwrap()
			).unwrap();

			borrow.push(
				DropCounter::new(1)
			).unwrap();
			borrow.push(
				DropCounter::new(18446744073709551615)
			).unwrap();

			unsafe {
				assert_eq!(DROP_COUNTER, 2);
			}

			borrow.clear();

			unsafe {
				assert_eq!(DROP_COUNTER, 0);
			}
		}
	}

	#[test]
	fn not_enough_capacity() {
		let mut rm: ReusableMemory<u8> = ReusableMemory::new().unwrap();
		{
			let capacity = NonZeroUsize::new(1).unwrap();
			let mut borrow = rm.borrow_mut_as::<usize>(
				capacity
			).unwrap();
			borrow.push(1).unwrap();

			match borrow.push(1) {
				Err(ReusableMemoryBorrowError::NotEnoughCapacity(c)) if c == capacity => (),
				_ => panic!("Expected Err(ReusableMemoryBorrowError::NotEnoughCapacity)")
			}
		}
	}

	#[test]
	fn zero_sized_base() {
		let rm: Result<ReusableMemory<()>, ReusableMemoryError> = ReusableMemory::new();
		match rm {
			Err(ReusableMemoryError::ZeroSizedB) => (),
			_ => panic!("Expected Err(ReusableMemoryError::ZeroSizedB)")
		}
	}

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
