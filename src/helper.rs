//! General functions and data types across the codebase.

/// Return `pos` as a u32 (usize -> u32).
///
/// # Panics
/// - if `pos > u32::MAX`.
#[track_caller]
#[must_use]
pub(crate) const fn u32_index(pos: usize) -> u32 {
    assert!(pos <= u32::MAX as usize, "pos should be at most u32::MAX");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Verified `pos` <= u32::MAX already"
    )]
    {
        pos as u32
    }
}

/// A typed, zero-based index backed by a `u32`.
pub trait Id: Copy {
    /// Construct an ID from a table index.
    ///
    /// # Panics
    /// Panics if `index` exceeds [`u32::MAX`].
    fn from_index(index: usize) -> Self;

    /// Return the index represented by this ID.
    fn index(self) -> usize;
}

/// Define a distinct `u32` ID type with a private underlying value.
///
/// Attributes and visibility precede the type name:
///
/// ```
/// use rforsp::{define_id, helper::{Id, Table}};
///
/// define_id!(pub ExampleId);
/// let mut entries = Table::<ExampleId, &str>::new();
/// let id = entries.push("example");
/// assert_eq!(id.index(), 0);
/// assert_eq!(entries.get(id), &"example");
/// ```
#[macro_export]
macro_rules! define_id {
    ($(#[$attr:meta])* $vis:vis $name:ident) => {
        #[doc = concat!("Typed index represented by `", stringify!($name), "`.")]
        $(#[$attr])*
        #[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
        $vis struct $name(u32);

        impl Default for $name {
            fn default() -> Self {
                Self(0)
            }
        }

        impl $crate::helper::Id for $name {
            #[track_caller]
            fn from_index(index: usize) -> Self {
                Self(u32::try_from(index).expect("ID index exceeds u32::MAX"))
            }

            fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

/// An append-only table indexed by a distinct ID type.
///
/// IDs remain stable as entries are appended. An ID identifies an entry kind,
/// not a particular table instance.
///
/// Different ID types cannot address the same table:
///
/// ```compile_fail
/// use rforsp::{define_id, helper::{Id, Table}};
///
/// define_id!(BodyId);
/// define_id!(DatumId);
/// let bodies = Table::<BodyId, ()>::new();
/// let _body = bodies.get(DatumId::from_index(0));
/// ```
#[derive(Debug)]
pub struct Table<I, T> {
    /// Entries in allocation order.
    entries: Vec<T>,
    /// Associate the table with its ID type without storing IDs.
    marker: std::marker::PhantomData<fn() -> I>,
}

impl<I: Id, T> Table<I, T> {
    /// Construct an empty table.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            marker: std::marker::PhantomData,
        }
    }

    /// Append an entry and return its ID.
    ///
    /// # Panics
    /// Panics if the next index exceeds [`u32::MAX`].
    #[must_use]
    #[track_caller]
    pub fn push(&mut self, value: T) -> I {
        let id = I::from_index(self.entries.len());
        self.entries.push(value);
        id
    }

    /// Retrieve an entry by ID.
    ///
    /// # Panics
    /// Panics if the ID is outside this table.
    #[must_use]
    #[track_caller]
    pub fn get(&self, id: I) -> &T {
        self.entries.get(id.index()).expect("ID outside table")
    }

    /// Retrieve an entry mutably by ID.
    ///
    /// # Panics
    /// Panics if the ID is outside this table.
    #[must_use]
    #[track_caller]
    pub fn get_mut(&mut self, id: I) -> &mut T {
        self.entries.get_mut(id.index()).expect("ID outside table")
    }

    /// Iterate over IDs and entries in allocation order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (I, &T)> + DoubleEndedIterator {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (I::from_index(index), entry))
    }

    /// Return the number of entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether the table has no entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<I: Id, T> Default for Table<I, T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocate dense IDs without storing an entry for each ID.
#[derive(Debug)]
pub struct IdCounter<I> {
    /// Number of IDs allocated so far.
    next: usize,
    /// Associate the counter with its ID type without storing IDs.
    marker: std::marker::PhantomData<fn() -> I>,
}

impl<I: Id> IdCounter<I> {
    /// Construct a counter with no allocated IDs.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: 0,
            marker: std::marker::PhantomData,
        }
    }

    /// Allocate the next ID.
    ///
    /// # Panics
    /// Panics if the next index exceeds [`u32::MAX`] or the count exceeds
    /// [`usize::MAX`].
    #[must_use]
    #[track_caller]
    pub fn allocate(&mut self) -> I {
        let id = I::from_index(self.next);
        self.next = self
            .next
            .checked_add(1)
            .expect("ID count exceeds usize::MAX");
        id
    }

    /// Return the number of IDs allocated.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.next
    }

    /// Return whether no IDs have been allocated.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.next == 0
    }
}

impl<I: Id> Default for IdCounter<I> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Id, IdCounter, Table};

    define_id!(TestId);

    #[test]
    fn table_allocates_stable_ids_and_supports_mutation() {
        let mut table = Table::<TestId, String>::default();
        assert!(table.is_empty(), "new table has no entries");
        let first = table.push("first".into());
        let second = table.push("second".into());

        assert_eq!(first.index(), 0, "first entry has index zero");
        assert_eq!(second.index(), 1, "IDs increase densely");
        assert_eq!(table.len(), 2, "each push adds one entry");
        table.get_mut(first).push_str(" updated");
        assert_eq!(table.get(first), "first updated", "mutation uses the ID");
        assert_eq!(table.get(second), "second", "other entry is unchanged");
        assert_eq!(
            table
                .iter()
                .map(|(id, value)| (id.index(), value.as_str()))
                .collect::<Vec<_>>(),
            [(0, "first updated"), (1, "second")],
            "iteration retains IDs and allocation order"
        );
    }

    #[test]
    #[should_panic(expected = "ID outside table")]
    fn table_rejects_out_of_bounds_ids() {
        let table = Table::<TestId, ()>::new();
        let _entry = table.get(TestId::from_index(0));
    }

    #[test]
    fn counter_allocates_dense_ids_without_entries() {
        let mut counter = IdCounter::<TestId>::default();
        assert!(counter.is_empty(), "new counter has no IDs");
        assert_eq!(counter.allocate().index(), 0, "first ID has index zero");
        assert_eq!(
            counter.allocate().index(),
            1,
            "second ID follows the first"
        );
        assert_eq!(counter.len(), 2, "count records both allocations");
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn counter_allocates_the_last_u32_id() {
        let mut counter = IdCounter::<TestId> {
            next: u32::MAX as usize,
            ..IdCounter::new()
        };
        assert_eq!(
            counter.allocate().index(),
            u32::MAX as usize,
            "last u32 index is valid"
        );
        assert_eq!(
            counter.len(),
            u32::MAX as usize + 1,
            "count can exceed the largest ID"
        );
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    #[should_panic(expected = "ID index exceeds u32::MAX")]
    fn counter_rejects_id_overflow() {
        let mut counter = IdCounter::<TestId> {
            next: u32::MAX as usize + 1,
            ..IdCounter::new()
        };
        let _id = counter.allocate();
    }
}
