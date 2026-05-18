use super::lensid::LensId;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LensIdCounter {
    hashmap: nohash_hasher::IntMap<LensId, usize>,
}

impl LensIdCounter {
    pub fn new() -> Self {
        Self {
            hashmap: nohash_hasher::IntMap::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            hashmap: nohash_hasher::IntMap::with_capacity_and_hasher(
                capacity,
                std::hash::BuildHasherDefault::default(),
            ),
        }
    }

    pub fn add_single(&mut self, lens_id: LensId) {
        *self.hashmap.entry(lens_id).or_insert(0) += 1;
    }

    pub fn add_single_with_count(&mut self, lens_id: LensId, count: usize) {
        *self.hashmap.entry(lens_id).or_insert(0) += count;
    }

    pub fn add(&mut self, other: Self) {
        for (lens_id, count) in other.hashmap {
            self.add_single_with_count(lens_id, count);
        }
    }

    pub fn add_from(&mut self, other: &Self) {
        for (lens_id, count) in other.iter() {
            // Only clone the LensId, not the entire counter
            self.add_single_with_count(lens_id.clone(), *count);
        }
    }

    /// Returns an iterator over the unique LensIds (keys).
    pub fn keys(&self) -> impl Iterator<Item = &LensId> {
        self.hashmap.keys()
    }

    /// Returns an iterator over (LensId, count) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&LensId, &usize)> {
        self.hashmap.iter()
    }

    /// Returns the count for a specific LensId, or 0 if not present.
    pub fn get(&self, lens_id: &LensId) -> usize {
        self.hashmap.get(lens_id).copied().unwrap_or(0)
    }

    /// Returns the number of unique LensIds.
    pub fn len(&self) -> usize {
        self.hashmap.len()
    }

    /// Returns true if the counter is empty.
    pub fn is_empty(&self) -> bool {
        self.hashmap.is_empty()
    }

    /// Consumes the counter and returns the inner HashMap.
    pub fn into_inner(self) -> nohash_hasher::IntMap<LensId, usize> {
        self.hashmap
    }
}

impl From<Vec<LensId>> for LensIdCounter {
    fn from(vector: Vec<LensId>) -> Self {
        // Vec already knows its length, so we can optimize
        let mut counter = Self::with_capacity(vector.len());
        counter.extend(vector); // Uses Extend trait
        counter
    }
}

impl FromIterator<LensId> for LensIdCounter {
    fn from_iter<T: IntoIterator<Item = LensId>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (lower_bound, _) = iter.size_hint();

        let mut counter = Self::with_capacity(lower_bound);
        counter.extend(iter);
        counter
    }
}

impl Extend<LensId> for LensIdCounter {
    fn extend<T: IntoIterator<Item = LensId>>(&mut self, iter: T) {
        for lens_id in iter {
            self.add_single(lens_id);
        }
    }
}
