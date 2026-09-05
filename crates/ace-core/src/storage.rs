//! Bounded-collection storage.
//!
//! `Vec<T, N>` holds at most `N` items. Where those items live depends on the
//! `alloc` feature — inline via `heapless::Vec` without it, heap-backed via
//! `alloc::vec::Vec` with it — but **the bound is enforced identically either
//! way**, and the fallible API is identical either way.
//!
//! That uniformity is the point. It was previously untrue: `alloc` mode aliased
//! `alloc::vec::Vec<T>` directly, which discards `N` and grows without limit, so
//! bounded memory did not hold in the configuration the test suite runs in, and
//! `push` returned `Result` in one configuration and `()` in the other. Error
//! handling therefore could not be written once and compiled in both.
//!
//! The API deliberately mirrors `heapless::Vec`: `push` returns the rejected
//! item so a caller can recover rather than lose it, and `extend_from_slice`
//! returns `Result<(), ()>`. Read-only access comes from `Deref<Target = [T]>`,
//! so `len`, `iter`, `get`, `first`, `contains`, and indexing all work as normal.

#[cfg(feature = "alloc")]
extern crate alloc;

use core::ops::{Deref, DerefMut, RangeBounds};

/// The collection was at, or would exceed, its capacity.
///
/// Named rather than `()` so a caller reading `Result<(), CapacityError>` can see
/// what failed without consulting the signature. [`Vec::push`] instead returns the
/// rejected item, which carries strictly more information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityError;

impl core::fmt::Display for CapacityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("bounded collection is at capacity")
    }
}

/// A collection bounded to `N` items in every configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vec<T, const N: usize> {
    #[cfg(not(feature = "alloc"))]
    inner: heapless::Vec<T, N>,
    #[cfg(feature = "alloc")]
    inner: alloc::vec::Vec<T>,
}

impl<T, const N: usize> Vec<T, N> {
    /// Creates an empty collection.
    pub const fn new() -> Self {
        Self {
            #[cfg(not(feature = "alloc"))]
            inner: heapless::Vec::new(),
            #[cfg(feature = "alloc")]
            inner: alloc::vec::Vec::new(),
        }
    }

    /// The maximum number of items this collection can hold.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Returns true when the collection is at capacity.
    pub fn is_full(&self) -> bool {
        self.inner.len() >= N
    }

    /// Appends an item.
    ///
    /// Returns `Err(item)` if the collection is already at capacity, handing the
    /// rejected item back so the caller can recover it rather than drop it. The
    /// bound is enforced in both configurations.
    pub fn push(&mut self, item: T) -> Result<(), T> {
        #[cfg(not(feature = "alloc"))]
        {
            self.inner.push(item)
        }
        #[cfg(feature = "alloc")]
        {
            if self.inner.len() >= N {
                return Err(item);
            }
            self.inner.push(item);
            Ok(())
        }
    }

    /// Removes and returns the item at `index`, shifting later items left.
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> T {
        self.inner.remove(index)
    }

    /// Removes all items.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Retains only the items for which `f` returns true.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.retain(f);
    }

    /// Removes the items in `range` and yields them.
    pub fn drain<R>(&mut self, range: R) -> impl Iterator<Item = T> + '_
    where
        R: RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    /// Borrows the contents as a slice.
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    /// Mutably borrows the contents as a slice.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.inner
    }

    /// Collects `iter` into a new collection, failing if it does not fit.
    ///
    /// `FromIterator` is deliberately **not** implemented. `heapless`'s
    /// implementation stops silently once full, so `iter.collect()` into a
    /// bounded collection is a silent-truncation hazard that compiles cleanly and
    /// loses data at runtime. Requiring this constructor makes the caller decide
    /// what an overflow means.
    pub fn from_iter_checked<I>(iter: I) -> Result<Self, CapacityError>
    where
        I: IntoIterator<Item = T>,
    {
        let mut out = Self::new();
        for item in iter {
            if out.push(item).is_err() {
                return Err(CapacityError);
            }
        }
        Ok(out)
    }
}

impl<T: Clone, const N: usize> Vec<T, N> {
    /// Appends every item in `other`.
    ///
    /// Returns `Err(CapacityError)` if `other` does not fit in the remaining
    /// capacity. On error nothing is appended, so the collection is left
    /// unchanged rather than partially extended.
    pub fn extend_from_slice(&mut self, other: &[T]) -> Result<(), CapacityError> {
        if self.inner.len() + other.len() > N {
            return Err(CapacityError);
        }

        #[cfg(not(feature = "alloc"))]
        {
            self.inner
                .extend_from_slice(other)
                .map_err(|_| CapacityError)
        }
        #[cfg(feature = "alloc")]
        {
            self.inner.extend_from_slice(other);
            Ok(())
        }
    }

    /// Resizes to `len`, filling new slots with `value`.
    ///
    /// Returns `Err(CapacityError)` if `len` exceeds `N`.
    pub fn resize(&mut self, len: usize, value: T) -> Result<(), CapacityError> {
        if len > N {
            return Err(CapacityError);
        }

        #[cfg(not(feature = "alloc"))]
        {
            self.inner.resize(len, value).map_err(|_| CapacityError)
        }
        #[cfg(feature = "alloc")]
        {
            self.inner.resize(len, value);
            Ok(())
        }
    }
}

impl<T, const N: usize> Default for Vec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Deref for Vec<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, const N: usize> DerefMut for Vec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T, const N: usize> IntoIterator for Vec<T, N> {
    type Item = T;
    #[cfg(not(feature = "alloc"))]
    type IntoIter = <heapless::Vec<T, N> as IntoIterator>::IntoIter;
    #[cfg(feature = "alloc")]
    type IntoIter = <alloc::vec::Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a Vec<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut Vec<T, N> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::{CapacityError, Vec};

    #[test]
    fn push_rejects_at_capacity_and_returns_the_item() {
        let mut v: Vec<u8, 2> = Vec::new();
        assert_eq!(v.push(1), Ok(()));
        assert_eq!(v.push(2), Ok(()));
        assert!(v.is_full());

        // The bound must hold in BOTH configurations. Before this newtype the
        // alloc build discarded N and this push silently succeeded.
        assert_eq!(v.push(3), Err(3), "capacity must be enforced");
        assert_eq!(v.len(), 2);
        assert_eq!(v.as_slice(), &[1, 2]);
    }

    #[test]
    fn extend_from_slice_is_all_or_nothing() {
        let mut v: Vec<u8, 4> = Vec::new();
        v.extend_from_slice(&[1, 2, 3]).unwrap();

        assert_eq!(
            v.extend_from_slice(&[4, 5]),
            Err(CapacityError),
            "must not partially extend"
        );
        assert_eq!(v.as_slice(), &[1, 2, 3], "collection left unchanged");

        v.extend_from_slice(&[4]).unwrap();
        assert_eq!(v.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn resize_beyond_capacity_is_rejected() {
        let mut v: Vec<u8, 3> = Vec::new();
        assert_eq!(v.resize(3, 7), Ok(()));
        assert_eq!(v.resize(4, 7), Err(CapacityError));
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn capacity_reports_n_not_current_allocation() {
        let v: Vec<u8, 8> = Vec::new();
        assert_eq!(v.capacity(), 8);
        assert!(v.is_empty());
    }

    #[test]
    fn drain_yields_and_empties() {
        let mut v: Vec<u8, 4> = Vec::new();
        v.extend_from_slice(&[1, 2, 3]).unwrap();

        let drained: heapless::Vec<u8, 4> = v.drain(..).collect();
        assert_eq!(drained.as_slice(), &[1, 2, 3]);
        assert!(v.is_empty());
    }

    #[test]
    fn slice_access_comes_through_deref() {
        let mut v: Vec<u8, 4> = Vec::new();
        v.extend_from_slice(&[10, 20, 30]).unwrap();

        assert_eq!(v.len(), 3);
        assert_eq!(v.first(), Some(&10));
        assert_eq!(v.get(1), Some(&20));
        assert_eq!(v[2], 30);
        assert!(v.contains(&20));
        assert_eq!(v.iter().sum::<u8>(), 60);
    }

    #[test]
    fn retain_and_remove() {
        let mut v: Vec<u8, 8> = Vec::new();
        v.extend_from_slice(&[1, 2, 3, 4]).unwrap();

        v.retain(|&x| x % 2 == 0);
        assert_eq!(v.as_slice(), &[2, 4]);

        assert_eq!(v.remove(0), 2);
        assert_eq!(v.as_slice(), &[4]);
    }

    // region: The bound must stay a ceiling, not a reservation

    /// With `alloc`, `N` must not become a static reservation.
    ///
    /// The whole point of the alloc configuration is to keep initialisation cheap and
    /// grow on demand rather than reserving the worst case up front. `N` is a ceiling
    /// checked on push. A regression - switching to `with_capacity(N)`, or an inline
    /// array - would show up here as a size that scales with `N`.
    #[cfg(feature = "alloc")]
    #[test]
    fn alloc_mode_size_is_independent_of_the_bound() {
        use core::mem::size_of;

        assert_eq!(
            size_of::<Vec<u8, 8>>(),
            size_of::<Vec<u8, 65_536>>(),
            "a bound of 8 and a bound of 65536 must occupy the same space"
        );
        assert_eq!(
            size_of::<Vec<u8, 65_536>>(),
            size_of::<alloc::vec::Vec<u8>>(),
            "the newtype must add nothing to a heap-backed vector"
        );
    }

    /// With `alloc`, an unused collection must not have touched the heap yet.
    #[cfg(feature = "alloc")]
    #[test]
    fn alloc_mode_allocates_nothing_until_first_push() {
        let mut v: Vec<u8, 4_096> = Vec::new();

        assert_eq!(v.capacity(), 4_096, "the bound is still reported");
        assert_eq!(
            v.inner.capacity(),
            0,
            "no heap allocation before first use - initialisation must stay cheap"
        );

        v.push(1).unwrap();
        assert!(
            v.inner.capacity() > 0 && v.inner.capacity() < 4_096,
            "growth is on demand, not straight to the bound: got {}",
            v.inner.capacity()
        );
    }

    /// Without `alloc` the storage is inline, so it necessarily scales with `N`.
    /// This is the trade the two configurations exist to express.
    #[cfg(not(feature = "alloc"))]
    #[test]
    fn no_alloc_mode_storage_is_inline_and_scales_with_the_bound() {
        use core::mem::size_of;

        assert!(size_of::<Vec<u8, 64>>() > size_of::<Vec<u8, 8>>());
        assert!(
            size_of::<Vec<u8, 64>>() >= 64,
            "inline storage must hold the whole bound"
        );
    }

    // endregion: The bound must stay a ceiling, not a reservation
}
