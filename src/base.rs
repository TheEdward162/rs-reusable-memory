use std::{mem, num::NonZeroUsize, ptr};

use super::borrow::ReusableMemoryBorrow;

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
			ReusableMemoryError::ZeroSizedB => {
				write!(f, "Type B (base type) must not be zero sized.")
			}
			ReusableMemoryError::ZeroSizedT => {
				write!(f, "Type T (borrowed type) must not be zero sized.")
			}
			ReusableMemoryError::CouldNotAlignPointer => {
				write!(f, "Could not align pointer to be to a pointer to T.")
			}
		}
	}
}
impl std::error::Error for ReusableMemoryError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { None }
}

/// `align_up(base, align)` returns the smallest greater integer than `base` aligned to `align`.
///
/// More formally:
/// ```norun
/// f_d(x) =
///     x, if x mod d = 0
///     x + d - x mod d, otherwise
/// ```
/// simplifies to `x - 1 + d - (x - 1) mod d`
/// assuming `d = 2^N`, can also be written in code like: `(x - 1 + d) & !(d - 1)`
/// where `x = base` and `d = align`
///
/// Similar code to `std::alloc::Layout::padding_needed_for`, but without the `- base`
const fn align_up(base: usize, align: usize) -> usize {
	base.wrapping_add(align.wrapping_sub(1)) & !align.wrapping_sub(1)
}
macro_rules! impl_borrow_mut_X_as {
	(
		pub fn $capacity_name: ident;
		pub fn $name: ident<$($gen_name: ident),+>[$count: literal];
	) => {
		// pub fn $capacity_name<$($gen_name),+>(
		// 	&self, capacity: [NonZeroUsize; $count]
		// ) -> Result<[usize; $count + 1], ReusableMemoryError> {
		// 	let align_of: [usize; $count] = [$(mem::align_of::<$gen_name>()),+];

		// 	$(
		// 		if mem::size_of::<$gen_name>() == 0 {
		// 			return Err(ReusableMemoryError::ZeroSizedT)
		// 		}
		// 	)+

		// 	let needed_bytes = 0;
		// 	let counter = 0;

		// 	$(
		// 		// where the block for $gen_name starts, in bytes, and the index
		// 		#[allow(non_snake_case)]
		// 		let $gen_name: (usize, usize) = (align_up(needed_bytes, mem::align_of::<$gen_name>()), counter);
		// 		// where the block from $gen_name ends
		// 		let needed_bytes = $gen_name.0 + mem::size_of::<$gen_name>() * capacity[counter].get();

		// 		#[allow(unused_variables)]
		// 		let counter = counter + 1;
		// 	)+

		// 	// Add `align - 1` to `needed_bytes` if align of `T` is more than align of `B`.
		// 	let align_bump = if mem::align_of::<B>() >= mem::align_of::<T>() {
		// 		0
		// 	} else {
		// 		align_of[0] - 1
		// 	};
		// 	// Add `align_bump` afterwards so that $gen_name starts are correct
		// 	let needed_bytes = needed_bytes + align_bump;
		// 	let needed_length = (needed_bytes + mem::size_of::<B>() - 1) / mem::size_of::<B>();

		// 	Ok(needed_length)
		// }

		pub fn $name<'mem, $($gen_name),+>(
			&'mem mut self, capacity: [NonZeroUsize; $count]
		) -> Result<( $(ReusableMemoryBorrow<'mem, $gen_name>),+ ), ReusableMemoryError> {
			let align_of: [usize; $count] = [$(mem::align_of::<$gen_name>()),+];

			$(
				if mem::size_of::<$gen_name>() == 0 {
					return Err(ReusableMemoryError::ZeroSizedT)
				}
			)+

			let needed_bytes = 0;
			let counter = 0;

			$(
				// where the block for $gen_name starts, in bytes, and the index
				#[allow(non_snake_case)]
				let $gen_name: (usize, usize) = (align_up(needed_bytes, mem::align_of::<$gen_name>()), counter);
				// where the block from $gen_name ends
				let needed_bytes = $gen_name.0 + mem::size_of::<$gen_name>() * capacity[counter].get();

				#[allow(unused_variables)]
				let counter = counter + 1;
			)+

			// Add `align - 1` to `needed_bytes` if align of `T` is more than align of `B`.
			let align_bump = if mem::align_of::<B>() >= mem::align_of::<T>() {
				0
			} else {
				align_of[0] - 1
			};
			// Add `align_bump` afterwards so that $gen_name starts are correct
			let needed_bytes = needed_bytes + align_bump;
			let needed_length = (needed_bytes + mem::size_of::<B>() - 1) / mem::size_of::<B>();

			// Reserve the memory
			self.vec.reserve(needed_length);
			let memory_ptr = self.vec.as_mut_ptr();

			// Compute the offset we need from the vec pointer to have the proper alignment.
			let align_offset = memory_ptr.align_offset(align_of[0]);
			if align_offset == std::usize::MAX {
				return Err(ReusableMemoryError::CouldNotAlignPointer)
			}

			Ok(
				unsafe {
					(
						$(
							ReusableMemoryBorrow::from_raw_parts(
								ptr::NonNull::new_unchecked(
									(memory_ptr.add(align_offset) as *mut u8).add($gen_name.0) as *mut $gen_name
								),
								capacity[$gen_name.1]
							)
						),+
					)
				}
			)
		}
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
	impl_borrow_mut_X_as!(
		pub fn needed_capacity_for_two;
		pub fn borrow_mut_two_as<T, U>[2];
	);

	impl_borrow_mut_X_as!(
		pub fn needed_capacity_for_three;
		pub fn borrow_mut_three_as<T, U, V>[3];
	);

	impl_borrow_mut_X_as!(
		pub fn needed_capacity_for_four;
		pub fn borrow_mut_four_as<T, U, V, W>[4];
	);

	impl_borrow_mut_X_as!(
		pub fn needed_capacity_for_five;
		pub fn borrow_mut_five_as<T, U, V, W, X>[5];
	);

	/// Creates new reusable memory without checking the size of `B`.
	///
	/// Can be used in const context.
	///
	/// ### Safety
	///
	/// * `std::mem::size_of::<B>()` must not be zero.
	pub const unsafe fn new_unchecked() -> Self { ReusableMemory { vec: Vec::new() } }

	pub fn new() -> Result<Self, ReusableMemoryError> { Self::with_capacity(0) }

	/// Counted in the capacity of `B`.
	pub fn with_capacity(len: usize) -> Result<Self, ReusableMemoryError> {
		if mem::size_of::<B>() == 0 {
			return Err(ReusableMemoryError::ZeroSizedB)
		}

		Ok(ReusableMemory { vec: Vec::with_capacity(len) })
	}

	pub fn needed_capacity_for<T>(
		&self, count: NonZeroUsize
	) -> Result<usize, ReusableMemoryError> {
		if mem::size_of::<T>() == 0 {
			return Err(ReusableMemoryError::ZeroSizedT)
		}

		// Add `align - 1` to `needed_bytes` if align of `T` is more than align of `B`.
		let align_bump =
			if mem::align_of::<B>() >= mem::align_of::<T>() { 0 } else { mem::align_of::<T>() - 1 };

		// Needed length in bytes.
		let needed_length = {
			let needed_bytes = mem::size_of::<T>() * count.get() + align_bump;

			// Needed length divided by the size of `B`, or the number of `B`s needed rounded up.
			(needed_bytes + mem::size_of::<B>() - 1) / mem::size_of::<B>()
		};

		Ok(needed_length)
	}

	/// Borrows the reusable memory as a different type.
	///
	/// This borrow is properly aligned and has at least the requested capacity.
	///
	/// Returns an error if `size_of::<T>() == 0`.
	/// Also returns an error when the pointer could not be aligned properly for `T`.
	pub fn borrow_mut_as<'mem, T>(
		&'mem mut self, capacity: NonZeroUsize
	) -> Result<ReusableMemoryBorrow<'mem, T>, ReusableMemoryError> {
		let needed_length = self.needed_capacity_for::<T>(capacity)?;

		// Reserve so at least `capacity` of `T`s fit, plus possible align offset.
		self.vec.reserve(needed_length);
		let memory_ptr = self.vec.as_mut_ptr();

		// Compute the offset we need from the vec pointer to have the proper alignment.
		let align_offset = memory_ptr.align_offset(mem::align_of::<T>());
		if align_offset == std::usize::MAX {
			return Err(ReusableMemoryError::CouldNotAlignPointer)
		}

		Ok(unsafe {
			ReusableMemoryBorrow::from_raw_parts(
				ptr::NonNull::new_unchecked(memory_ptr.add(align_offset) as *mut T),
				capacity
			)
		})
	}
}
