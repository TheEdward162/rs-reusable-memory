use std::{
	fmt,
	ops::{Bound, Range, RangeBounds}
};

use super::ReusableMemoryBorrow;

// Most of this code is copied from std Vec

pub struct BorrowDrainIter<'bor, 'mem, T: 'mem> {
	borrow: &'bor mut ReusableMemoryBorrow<'mem, T>,
	drain_range: Range<usize>,

	/// Start of the tail after the drained items
	tail_start: usize,
	/// Length of tail after the drained items
	tail_len: usize
}
impl<'bor, 'mem: 'bor, T: 'mem> BorrowDrainIter<'bor, 'mem, T> {
	pub(super) fn new(
		borrow: &'bor mut ReusableMemoryBorrow<'mem, T>, range: impl RangeBounds<usize>
	) -> Self {
		let len = borrow.len();
		let start = match range.start_bound() {
			Bound::Included(&n) => n,
			Bound::Excluded(&n) => n + 1,
			Bound::Unbounded => 0
		};
		let end = match range.end_bound() {
			Bound::Included(&n) => n + 1,
			Bound::Excluded(&n) => n,
			Bound::Unbounded => len
		};
		assert!(start <= end);
		assert!(end <= len);

		unsafe {
			// Safety in case Drain is leaked
			borrow.set_len(start);

			Self { borrow, drain_range: start .. end, tail_start: end, tail_len: len - end }
		}
	}
}
impl<T: fmt::Debug> fmt::Debug for BorrowDrainIter<'_, '_, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("BorrowDrainIter")
			.field(unsafe {
				&std::slice::from_raw_parts(
					self.borrow.as_ptr().add(self.drain_range.start),
					self.drain_range.end - self.drain_range.start
				)
			})
			.finish()
	}
}
impl<T> Iterator for BorrowDrainIter<'_, '_, T> {
	type Item = T;

	fn next(&mut self) -> Option<T> {
		self.drain_range
			.next()
			.map(|offset| unsafe { std::ptr::read(self.borrow.as_ptr().add(offset)) })
	}

	fn size_hint(&self) -> (usize, Option<usize>) { self.drain_range.size_hint() }
}
impl<T> DoubleEndedIterator for BorrowDrainIter<'_, '_, T> {
	fn next_back(&mut self) -> Option<T> {
		self.drain_range
			.next_back()
			.map(|offset| unsafe { std::ptr::read(self.borrow.as_ptr().add(offset)) })
	}
}
impl<T> ExactSizeIterator for BorrowDrainIter<'_, '_, T> {}
impl<T> Drop for BorrowDrainIter<'_, '_, T> {
	fn drop(&mut self) {
		// exhaust self first
		self.for_each(drop);

		if self.tail_len > 0 {
			unsafe {
				let start = self.borrow.len();
				let tail = self.tail_start;
				// There is some tail left and we need to memmove it
				if start != tail {
					let src = self.borrow.as_ptr().add(tail);
					let dst = self.borrow.as_mut_ptr().add(start);
					std::ptr::copy(src, dst, self.tail_len);
				}

				self.borrow.set_len(start + self.tail_len);
			}
		}
	}
}
