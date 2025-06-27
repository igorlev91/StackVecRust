//  Description:
//!   Implements a non-reallocatable, but resizeable, [`Vec`]-like
//!   structure that lives in the stack.
//

use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Result as FResult};
use std::iter::FusedIterator;
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};


/***** HELPER MACROS *****/
/// Implements [`Index`] and [`IndexMut`] for a particular range.
macro_rules! index_range_impl {
    ($range:ty, $conv:expr) => {
        impl<const LEN: usize, T> Index<$range> for StackVec<LEN, T> {
            type Output = [T];

            #[inline]
            #[track_caller]
            fn index(&self, index: $range) -> &Self::Output {
                // Get a proper range out of this
                let (start, end): (usize, usize) = $conv(self.len, index);

                // Create a slice
                // SAFETY: Gotta prove two things here;
                // - We can safely assume that the [`MaybeUninit`]s are initialized, because of our assertion for `self.len` that the first `self.len` elements are always initialized, and we made sure that the `index` is within that space; and
                // - We can call `std::mem::transmute` because `T` and `MaybeUninit<T>` are guaranteed to have the same layout. Conditions on stuff like abusing niche values do not apply, since we don't inherently enum the MaybeUninit, and neither does slice do anything with it.
                unsafe { std::mem::transmute(&self.data[start..end]) }
            }
        }
        impl<const LEN: usize, T> IndexMut<$range> for StackVec<LEN, T> {
            #[inline]
            #[track_caller]
            fn index_mut(&mut self, index: $range) -> &mut Self::Output {
                // Get a proper range out of this
                let (start, end): (usize, usize) = $conv(self.len, index);

                // Create a slice
                // SAFETY: Gotta prove two things here;
                // - We can safely assume that the [`MaybeUninit`]s are initialized, because of our assertion for `self.len` that the first `self.len` elements are always initialized, and we made sure that the `index` is within that space; and
                // - We can call `std::mem::transmute` because `T` and `MaybeUninit<T>` are guaranteed to have the same layout. Conditions on stuff like abusing niche values do not apply, since we don't inherently enum the MaybeUninit, and neither does slice do anything with it.
                unsafe { std::mem::transmute(&mut self.data[start..end]) }
            }
        }
    };
}





/***** ITERATORS *****/
/// Iterates over a [`StackVec`] by ownership.
#[derive(Clone, Debug)]
pub struct IntoIter<const LEN: usize, T> {
    /// Some [`StackVec`] that we iterate over.
    vec: StackVec<LEN, T>,
    /// The current index of iteration.
    i:   usize,
    /// The current end of the iteration. Exclusive (so `0` means nothing).
    end: usize,
}

impl<const LEN: usize, T> Default for IntoIter<LEN, T> {
    /// Creates an empty iterator.
    #[inline]
    fn default() -> Self { Self { vec: StackVec::default(), i: 0, end: 0 } }
}
impl<const LEN: usize, T> Drop for IntoIter<LEN, T> {
    #[inline]
    fn drop(&mut self) {
        // Drop any remaining elements
        while self.i < self.end {
            // SAFETY: This is OK because of the `self.len` assertion and `i` is below that length (because `end` is below that length).
            unsafe { self.vec.data[self.i].assume_init_drop() };
            self.i += 1;
        }
    }
}

impl<const LEN: usize, T> Iterator for IntoIter<LEN, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.end {
            // Get the element
            let mut res: MaybeUninit<T> = MaybeUninit::uninit();
            std::mem::swap(&mut res, &mut self.vec.data[self.i]);
            self.i += 1;

            // SAFETY: This is OK because of the `self.len` assertion and `i` is below that length (because `end` is below that length).
            Some(unsafe { res.assume_init() })
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { (self.end - self.i, Some(self.end - self.i)) }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.end - self.i
    }
}
impl<const LEN: usize, T> DoubleEndedIterator for IntoIter<LEN, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end > 0 {
            // Get the element
            let mut res: MaybeUninit<T> = MaybeUninit::uninit();
            std::mem::swap(&mut res, &mut self.vec.data[self.end]);
            self.end -= 1;

            // SAFETY: This is OK because of the `self.len` assertion and `end` is below that length (given at construction).
            Some(unsafe { res.assume_init() })
        } else {
            None
        }
    }
}
impl<const LEN: usize, T> ExactSizeIterator for IntoIter<LEN, T> {
    #[inline]
    fn len(&self) -> usize { self.end - self.i }
}
impl<const LEN: usize, T> FusedIterator for IntoIter<LEN, T> {}





/***** LIBRARY *****/
/// Implements a non-reallocatable, but resizeable, [`Vec`]-like structure that lives in the stack.
///
/// This makes allocating it pretty cheap, and even implements [`Copy`]. Even better, basically all functions on it can be `const`.
///
/// **Editor's note**: Making `StackVec` [`Copy`] requires conditional [`Drop`] to be safe, which is not possible currently. Aww man that's sad.
pub struct StackVec<const LEN: usize, T> {
    /// The data array that we wrap.
    data: [MaybeUninit<T>; LEN],
    /// The current number of initialized elements.
    ///
    /// We implement the StackVec such that is upholds the following assertion: the first `len` elements of `data` are initialized.
    len:  usize,
}

impl<const LEN: usize, T> Default for StackVec<LEN, T> {
    #[inline]
    fn default() -> Self { Self::new() }
}
impl<const LEN: usize, T> StackVec<LEN, T> {
    /// Constructor for the StackVec that initializes it as empty.
    ///
    /// Note that, by design, StackVecs always have capacity `LEN`.
    ///
    /// # Returns
    /// A new StackVec with no elements in it.
    #[inline]
    pub const fn new() -> Self {
        Self {
            // SAFETY: We can do this because the "initialization" actually leaves us with uninitialized elements, still.
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len:  0,
        }
    }

    /// Gets an element from the StackVec by index.
    ///
    /// This is a "safe" version of reading an element in the StackVec. If you want a less forgiving check, see [`Self::index()`](StackVec::index()); or, if you are particularly brave, see [`Self::get_unchecked()`](StackVec::get_unchecked()).
    ///
    /// # Arguments
    /// - `idx`: The index of the element to return.
    ///
    /// # Returns
    /// A reference to the referred element, or else [`None`] if the `idx` is out-of-bounds.
    #[inline]
    pub const fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and that `idx` is surely within range of `self.len`.
            Some(unsafe { self.data[idx].assume_init_ref() })
        } else {
            None
        }
    }

    /// Gets an element from the StackVec by index with no handholding.
    ///
    /// This is the fastest version of reading an element in the StackVec, but you have to manually ensure that **your `idx` is within range of this StackVec**. Essentially, only do this if
    /// ```ignore
    /// idx < stack_vec.len()
    /// ```
    /// returns true (see [Self::len()](StackVec::len())).
    ///
    /// You can use [`Self::index()`](StackVec::index()) for a version that automatically performs this check. Alternatively, you can also use [`Self::get()`](StackVec::get()) if you want this check to be recoverable.
    ///
    /// # Arguments
    /// - `idx`: The index of the element to return.
    ///
    /// # Returns
    /// A reference to the referred element, or else [`None`] if the `idx` is out-of-bounds.
    #[inline]
    #[track_caller]
    pub const unsafe fn get_unchecked(&self, idx: usize) -> &T {
        // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and rely on the user to ensure that `idx < self.len`.
        self.data[idx].assume_init_ref()
    }

    /// Gets an element mutably from the StackVec by index.
    ///
    /// This is a "safe" version of reading/writing an element in the StackVec. If you want a less forgiving check, see [`Self::index_mut()`](StackVec::index_mut());
    /// or, if you are particularly brave, see [`Self::get_mut_unchecked()`](StackVec::get_mut_unchecked()).
    ///
    /// # Arguments
    /// - `idx`: The index of the element to return.
    ///
    /// # Returns
    /// A mutable reference to the referred element, or else [`None`] if the `idx` is out-of-bounds.
    #[inline]
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx < self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and that `idx` is surely within range of `self.len`.
            Some(unsafe { self.data[idx].assume_init_mut() })
        } else {
            None
        }
    }

    /// Gets an element mutably from the StackVec by index with no handholding.
    ///
    /// This is the fastest version of reading/writing an element in the StackVec, but you have to manually ensure that **your `idx` is within range of this StackVec**.
    /// Essentially, only do this if
    /// ```ignore
    /// idx < stack_vec.len()
    /// ```
    /// returns true (see [Self::len()](StackVec::len())).
    ///
    /// You can use [`Self::index_mut()`](StackVec::index_mut()) for a version that automatically performs this check. Alternatively,
    /// you can also use [`Self::get_mut()`](StackVec::get_mut()) if you want this check to be recoverable.
    ///
    /// # Arguments
    /// - `idx`: The index of the element to return.
    ///
    /// # Returns
    /// A mutable reference to the referred element, or else [`None`] if the `idx` is out-of-bounds.
    #[inline]
    #[track_caller]
    pub unsafe fn get_mut_unchecked(&mut self, idx: usize) -> &mut T {
        // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and rely on the user to ensure that `idx < self.len`.
        self.data[idx].assume_init_mut()
    }

    /// Returns this StackVec as a slice of `T`s.
    ///
    /// # Returns
    /// A [`&[T]`] that has the length of this StackVec. Equivalent to `&stack_vec[..]`.
    #[inline]
    pub fn as_slice(&self) -> &[T] { &self[..] }

    /// Returns this StackVec as a slice of `T`s.
    ///
    /// # Returns
    /// A [`&mut [T]`] that has the length of this StackVec. Equivalent to `&mut stack_vec[..]`.
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [T] { &mut self[..] }

    /// Removes an element from the StackVec.
    ///
    /// This version preserves the order of non-removed elements. This is at the cost of moving all those other elements one place closer.
    ///
    /// # Arguments
    /// - `idx`: The index of the element to return.
    ///
    /// # Returns
    /// The removed element, or else [`None`] if the `idx` is out-of-bounds. The vec is guaranteed to be untouched, in that case.
    #[inline]
    pub fn remove(&mut self, idx: usize) -> Option<T> {
        if idx < self.len {
            // Move all the elements one to the front. By swapping, we push the "actual" element to the back of the array
            for i in idx..self.len - 1 {
                // SAFETY: This will not break our `self.len` assertion, because both indices are guaranteed to be below `self.len`, keeping it intact.
                self.data.swap(i, i + 1);
            }

            // Get the element we're talking about, leaving the old value uninitialized
            let mut res: MaybeUninit<T> = MaybeUninit::uninit();
            std::mem::swap(&mut res, &mut self.data[self.len - 1]);
            self.len -= 1;

            // OK, return that
            // SAFETY: We assert throughout that all elements before `self.len` are initialized, which was the case above. We also assert that `idx` is within range.
            Some(unsafe { res.assume_init() })
        } else {
            // Nothing to be done
            None
        }
    }

    /// Removes an element from the StackVec, then moves the last element in-place of the removed one.
    ///
    /// This version does _not_ preserve the order of non-removed elements. However, this is more efficient, as it does not require us to move all elements in the array but only the last one.
    ///
    /// # Arguments
    /// - `idx`: The index of the element to return.
    ///
    /// # Returns
    /// The removed element, or else [`None`] if the `idx` is out-of-bounds. The vec is guaranteed to be untouched, in that case.
    #[inline]
    pub fn swap_remove(&mut self, idx: usize) -> Option<T> {
        if idx < self.len {
            // Swap the selected and the last element
            // The function itself takes care that this doesn't happen needlessly.
            // SAFETY: This will not break our `self.len` assertion, because both indices are guaranteed to be below `self.len`, keeping it intact.
            self.data.swap(idx, self.len - 1);

            // Get the element we're talking about, leaving the old value uninitialized
            let mut res: MaybeUninit<T> = MaybeUninit::uninit();
            std::mem::swap(&mut res, &mut self.data[self.len - 1]);
            self.len -= 1;

            // OK, return that
            // SAFETY: We assert throughout that all elements before `self.len` are initialized, which was the case above. We also assert that `idx` is within range.
            Some(unsafe { res.assume_init() })
        } else {
            // Nothing to be done
            None
        }
    }

    /// Removes the last element from the StackVec.
    ///
    /// # Returns
    /// An element if there was one, or else [`None`].
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            // Get the element we're talking about, leaving the old value uninitialized
            let mut res: MaybeUninit<T> = MaybeUninit::uninit();
            std::mem::swap(&mut res, &mut self.data[self.len - 1]);
            self.len -= 1;

            // OK, return that
            // SAFETY: We assert throughout that all elements before `self.len` are initialized, which was the case above. We also asserted there was at least one element.
            Some(unsafe { res.assume_init() })
        } else {
            None
        }
    }

    /// Removes _all_ elements from the StackVec, starting afresh.
    #[inline]
    pub fn clear(&mut self) {
        // Drop all elements in ourselves
        for i in 0..self.len {
            // SAFETY: OK because `i` is guaranteed to be below `self.len`, and we asserted the first `self.len` elements are ininitialized.
            unsafe {
                self.data[i].assume_init_drop();
            }
        }

        // Reset the length to reset the elements
        self.len = 0;
    }

    /// Pushes a new element to the end of the StackVec.
    ///
    /// # Arguments
    /// - `elem`: The new element (of type `T`) to push.
    ///
    /// # Panics
    /// This function can panic if the there isn't enough space in the Vec. You can prevent this by manually checking for space, i.e.,
    /// ```ignore
    /// if stack_vec.len() < stack_vec.capacity() {
    ///     // Never panics now
    ///     stack_vec.push(elem);
    /// }
    /// ```
    #[inline]
    #[track_caller]
    pub fn push(&mut self, elem: T) {
        // Assert there is enough space
        if self.len < LEN {
            self.data[self.len].write(elem);
            // SAFETY: This upholds our `self.len` assertion, because we just initialized the value that we promise will be initialized.
            self.len += 1;
        } else {
            panic!("Cannot push {}th element to StackVec of capacity {}", self.len + 1, LEN);
        }
    }

    /// Inserts a new element in the StackVec at a given location.
    ///
    /// The insert location must either replace an existing element, or be exactly after the last element. Anything else is considered out-of-bounds.
    ///
    /// The replaced element and all elements after it are pushed one space back to preserve array order.
    ///
    /// # Arguments
    /// - `idx`: The index to insert the new element in.
    /// - `elem`: The new element to insert.
    ///
    /// # Panic
    /// This function panics if the given `idx` is out-of-bounds by more than 1 (i.e., one place outside of the current length is OK, emulating a [`Self::push()`](StackVec::push())).
    ///
    /// Another panic case is if there is not enough capacity to store the extra element. You can prevent this by manually checking for space, i.e.,
    /// ```ignore
    /// if stack_vec.len() < stack_vec.capacity() {
    ///     // Never panics now
    ///     stack_vec.push(elem);
    /// }
    /// ```
    #[inline]
    #[track_caller]
    pub fn insert(&mut self, idx: usize, elem: T) {
        // Assert there is enough space
        if self.len < LEN {
            // Assert the index is within bounds
            if idx <= LEN {
                // Push all elements one further
                for i in (idx + 1..=self.len).rev() {
                    // SAFETY: This temporarily BREAKS our `self.len` assertion, because we push the uninitialized element at `self.len` forward to below the boundary.
                    //         This will, however, be remedied below.
                    self.data.swap(i, i - 1);
                }

                // Now insert the element
                // SAFETY: This restores our `self.len` assertion, because we initialize the only uninitialized element.
                self.data[idx].write(elem);
                // SAFETY: This is OK, because we swapped the uninitialized space at the end for the then-last element.
                self.len += 1;
            } else {
                panic!("Inserting at index {} is out-of-bounds for StackVec of length {}", idx, self.len);
            }
        } else {
            panic!("Cannot push {}th element to StackVec of capacity {}", self.len + 1, LEN);
        }
    }

    /// Extends this StackVec with any number of new elements.
    ///
    /// The elements are pushed to the end of the vec in-order as yielded by the iterator.
    ///
    /// # Arguments
    /// - `elems`: Something [iterable](IntoIterator) that generates the elements to append.
    ///
    /// # Panics
    /// This function can panic if one of the elements causes the StackVec to outgrow its capacity. Being stack-allocated, it cannot be resized.
    ///
    /// Note that this panic is raised lazily, i.e., if it occurs, any elements that may have fit will have been written.
    #[inline]
    #[track_caller]
    pub fn extend(&mut self, elems: impl IntoIterator<Item = T>) {
        // No need to reserve, we have all the capacity we ever get
        for elem in elems {
            self.push(elem);
        }
    }

    /// Returns an iterator over the internal `T`s.
    ///
    /// This is equivalent to calling:
    /// ```ignore
    /// stack_vec.as_slice().iter()
    /// ```
    ///
    /// # Returns
    /// A [`std::slice::Iter`] is returned, with all the double-endedness pleasure that comes along with it.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<T> { self.as_slice().iter() }

    /// Returns a mutable iterator over the internal `T`s.
    ///
    /// This is equivalent to calling:
    /// ```ignore
    /// stack_vec.as_slice_mut().iter_mut()
    /// ```
    ///
    /// # Returns
    /// A [`std::slice::IterMut`] is returned, with all the double-endedness pleasure that comes along with it.
    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> { self.as_slice_mut().iter_mut() }

    /// Returns a iterator-by-ownership over the internal `T`s.
    ///
    /// # Returns
    /// An [`IntoIter`] that owns the internal array and uses it to efficiently return elements.
    #[inline]
    pub fn into_iter(self) -> IntoIter<LEN, T> {
        let end: usize = self.len;
        IntoIter { vec: self, i: 0, end }
    }

    /// Returns the number of elements stored in the StackVec.
    #[inline]
    pub const fn len(&self) -> usize { self.len }

    /// Returns whether any elements are stored in the StackVec at all.
    ///
    /// # Returns
    /// True if there are 0 elements, false if there is at least 1.
    #[inline]
    pub const fn is_empty(&self) -> bool { self.len == 0 }

    /// Returns the number of elements this StackVec can store in total.
    ///
    /// Note that, when this number is exceeded, the StackVec does not re-allocate (like a [`Vec`]) but instead throws errors.
    #[inline]
    pub const fn capacity(&self) -> usize { LEN }
}

// Things we usually derive, but require some special attention
impl<const LEN: usize, T: Clone> Clone for StackVec<LEN, T> {
    #[inline]
    fn clone(&self) -> Self {
        // Clone only initialized elements
        // SAFETY: We can do this because the "initialization" actually leaves us with uninitialized elements, still.
        let mut data: [MaybeUninit<T>; LEN] = unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized.
            data[i] = MaybeUninit::new(unsafe { self.data[i].assume_init_ref() }.clone());
        }

        // OK, create Self with that
        Self { data, len: self.len }
    }
}
// NOTE: Can re-enable once/if [`Drop`] becomes conditional.
// impl<const LEN: usize, T: Copy> Copy for StackVec<LEN, T> {}
impl<const LEN: usize, T: Debug> Debug for StackVec<LEN, T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> FResult {
        let mut vec = f.debug_list();
        for i in 0..self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized.
            vec.entry(unsafe { self.data[i].assume_init_ref() });
        }
        vec.finish()
    }
}
// The drop ruining the [`Copy`] implementation :/
impl<const LEN: usize, T> Drop for StackVec<LEN, T> {
    #[inline]
    fn drop(&mut self) {
        for i in 0..self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized.
            unsafe { self.data[i].assume_init_drop() };
        }
    }
}
impl<const LEN: usize, T: Eq> Eq for StackVec<LEN, T> {}
impl<const LEN: usize, T: PartialEq> PartialEq for StackVec<LEN, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        for i in 0..self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized.
            let lhs: &T = unsafe { self.data[i].assume_init_ref() };
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and the fact that we asserted `self.len == other.len` (so the property extends to the other vec too).
            let rhs: &T = unsafe { other.data[i].assume_init_ref() };

            // Now we can compare
            if lhs != rhs {
                return false;
            }
        }
        true
    }

    #[inline]
    fn ne(&self, other: &Self) -> bool {
        if self.len != other.len {
            return true;
        }
        for i in 0..self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized.
            let lhs: &T = unsafe { self.data[i].assume_init_ref() };
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and the fact that we asserted `self.len == other.len` (so the property extends to the other vec too).
            let rhs: &T = unsafe { other.data[i].assume_init_ref() };

            // Now we can compare
            if lhs == rhs {
                return false;
            }
        }
        true
    }
}
impl<const LEN: usize, T: Ord> Ord for StackVec<LEN, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // SAFETY: We can `unwrap()` here because [`Ord`] requires that `T`'s [`PartialOrd`] implementation always returns [`Some`].
        self.partial_cmp(other)
            .expect("Broken promise from 'T'; T implementing 'Ord' requires that its 'PartialOrd' implementation always returns 'Some'")
    }
}
impl<const LEN: usize, T: PartialOrd> PartialOrd for StackVec<LEN, T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        for i in 0.. {
            // See if we're still into range
            if i >= self.len && i >= other.len {
                // They really are the same; stop here
                break;
            } else if i < self.len {
                // `self` is shorter than `other`, which tells us to consider `self` the lesser
                // See 'https://doc.rust-lang.org/std/cmp/trait.Ord.html#lexicographical-comparison'
                return Some(Ordering::Less);
            } else if i < other.len {
                // `self` is longer than `other`, which tells us to consider `self` the greater
                // See 'https://doc.rust-lang.org/std/cmp/trait.Ord.html#lexicographical-comparison'
                return Some(Ordering::Greater);
            }

            // Otherwise, the ordering depends on the elements
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and the fact that we asserted that `i` is shorted than `self.len`.
            let lhs: &T = unsafe { self.data[i].assume_init_ref() };
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and the fact that we asserted that `i` is shorted than `other.len`.
            let rhs: &T = unsafe { other.data[i].assume_init_ref() };

            // Compare them
            match lhs.partial_cmp(rhs) {
                Some(Ordering::Greater) => return Some(Ordering::Greater),
                Some(Ordering::Less) => return Some(Ordering::Less),
                None => return None,

                // Only if they are explicitly equal, we continue
                Some(Ordering::Equal) => continue,
            }
        }

        // If we got here, all elements & length are equal
        Some(Ordering::Equal)
    }
}

// Indexing
impl<const LEN: usize, T> Index<usize> for StackVec<LEN, T> {
    type Output = T;

    #[inline]
    #[track_caller]
    fn index(&self, index: usize) -> &Self::Output {
        if index < self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and that `idx` is surely within range of `self.len`.
            unsafe { self.data[index].assume_init_ref() }
        } else {
            panic!("Index {} is out-of-bounds for a StackVec of length {}", index, self.len);
        }
    }
}
impl<const LEN: usize, T> IndexMut<usize> for StackVec<LEN, T> {
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < self.len {
            // SAFETY: We use our assertion for `self.len` that the first `self.len` elements are always initialized, and that `idx` is surely within range of `self.len`.
            unsafe { self.data[index].assume_init_mut() }
        } else {
            panic!("Index {} is out-of-bounds for a StackVec of length {}", index, self.len);
        }
    }
}
index_range_impl!(Range<usize>, |len: usize, index: Range<usize>| {
    if index.start >= len {
        panic!("Range start {} (inclusive) is out-of-bounds for a StackVec of length {}", index.start, len);
    }
    if index.end > len {
        panic!("Range end {} (exclusive) is out-of-bounds for a StackVec of length {}", index.end, len);
    }
    (index.start, index.end)
});
index_range_impl!(RangeInclusive<usize>, |len: usize, index: RangeInclusive<usize>| {
    let (start, end): (usize, usize) = (*index.start(), *index.end());
    if start >= len {
        panic!("Range start {} (inclusive) is out-of-bounds for a StackVec of length {}", start, len);
    }
    if end >= len {
        panic!("Range end {} (inclusive) is out-of-bounds for a StackVec of length {}", end, len);
    }
    (start, end + 1)
});
index_range_impl!(RangeFrom<usize>, |len: usize, index: RangeFrom<usize>| {
    if index.start >= len {
        panic!("Range start {} (inclusive) is out-of-bounds for a StackVec of length {}", index.start, len);
    }
    (index.start, len)
});
index_range_impl!(RangeTo<usize>, |len: usize, index: RangeTo<usize>| {
    if index.end > len {
        panic!("Range end {} (exclusive) is out-of-bounds for a StackVec of length {}", index.end, len);
    }
    (0, index.end)
});
index_range_impl!(RangeToInclusive<usize>, |len: usize, index: RangeToInclusive<usize>| {
    if index.end >= len {
        panic!("Range end {} (inclusive) is out-of-bounds for a StackVec of length {}", index.end, len);
    }
    (0, index.end + 1)
});
index_range_impl!(RangeFull, |len: usize, _index: RangeFull| { (0, len) });

// Iteration
impl<const LEN: usize, T> IntoIterator for StackVec<LEN, T> {
    type IntoIter = IntoIter<LEN, T>;
    type Item = T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter { <Self>::into_iter(self) }
}
impl<'s, const LEN: usize, T> IntoIterator for &'s StackVec<LEN, T> {
    type IntoIter = std::slice::Iter<'s, T>;
    type Item = &'s T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter { self.iter() }
}
impl<'s, const LEN: usize, T> IntoIterator for &'s mut StackVec<LEN, T> {
    type IntoIter = std::slice::IterMut<'s, T>;
    type Item = &'s mut T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter { self.iter_mut() }
}

// From
impl<const LEN: usize, T> FromIterator<T> for StackVec<LEN, T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        // Create a new stack, extend, enjoy
        let mut stack: Self = Self::new();
        stack.extend(iter);
        stack
    }
}
impl<const LEN: usize, T> From<[T; LEN]> for StackVec<LEN, T> {
    #[inline]
    fn from(value: [T; LEN]) -> Self { Self::from_iter(value) }
}
impl<const LEN: usize, T: Clone> From<&[T]> for StackVec<LEN, T> {
    #[inline]
    fn from(value: &[T]) -> Self { Self::from_iter(value.into_iter().cloned()) }
}
impl<const LEN: usize, T> From<Vec<T>> for StackVec<LEN, T> {
    #[inline]
    fn from(value: Vec<T>) -> Self { Self::from_iter(value) }
}
