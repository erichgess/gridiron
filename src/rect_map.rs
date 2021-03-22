use core::ops::{Range, RangeBounds};
use crate::interval_map::IntervalMap;




/// Type alias for a 1d range
type Rectangle<T> = (Range<T>, Range<T>);




/// Type alias for a 2d range, by-reference
type RectangleRef<'a, T> = (&'a Range<T>, &'a Range<T>);




/**
 * An associative map where the keys are `Rectangle` objects. Supports point,
 * rectangle, generic 2d range-based queries to iterate over key-value pairs.
 */
pub struct RectangleMap<T: Ord + Copy, V> {
    map: IntervalMap<T, IntervalMap<T, V>>
}




// ============================================================================
impl<T: Ord + Copy, V> RectangleMap<T, V> {

    pub fn new() -> Self {
        Self { map: IntervalMap::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.iter().map(|(_, m)| m.len()).sum()
    }

    pub fn contains(&self, key: RectangleRef<T>) -> bool {
        self.map.get(key.0).map_or(false, |m| m.contains(key.1))
    }

    pub fn get(&self, key: RectangleRef<T>) -> Option<&V> {
        self.map.get(key.0).and_then(|m| m.get(key.1))
    }

    pub fn get_mut(&mut self, key: RectangleRef<T>) -> Option<&mut V> {
        self.map.get_mut(key.0).and_then(|m| m.get_mut(key.1))
    }

    pub fn insert(&mut self, area: Rectangle<T>, value: V) -> &mut V {
        let (di, dj) = area;
        self.map
            .require(di)
            .insert(dj, value)
    }

    pub fn require(&mut self, area: Rectangle<T>) -> &mut V where V: Default {
        let (di, dj) = area;
        self.map
            .require(di)
            .require(dj)
    }

    pub fn remove(&mut self, key: RectangleRef<T>) {
        if let Some(m) = self.map.get_mut(key.0) {
            m.remove(key.1);
            if m.is_empty() {
                self.map.remove(key.0)                
            }
        }
    }

    pub fn into_balanced(self) -> Self {
        Self { map: self.map.into_sorted().map(|(k, m)| (k, m.into_balanced())).collect() }
    }

    pub fn into_iter(self) -> impl Iterator<Item = (Rectangle<T>, V)> {
        self.map
            .into_iter()
            .map(|(di, l)| l.into_iter().map(move |(dj, m)| ((di.clone(), dj), m)))
            .flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = (RectangleRef<T>, &V)> {
        self.map
            .iter()
            .map(|(di, l)| l.iter().map(move |(dj, m)| ((di, dj), m)))
            .flatten()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (RectangleRef<T>, &mut V)> {
        self.map
            .iter_mut()
            .map(|(di, l)| l.iter_mut().map(move |(dj, m)| ((di, dj), m)))
            .flatten()
    }

    pub fn query_point<'a>(&'a self, point: (&'a T, &'a T)) -> impl Iterator<Item = (RectangleRef<'a, T>, &'a V)> {
        self.map
            .query_point(point.0)
            .map(move |(di, l)| l.query_point(point.1).map(move |(dj, m)| ((di, dj), m)))
            .flatten()
    }

    pub fn query_rect<'a>(&'a self, rect: RectangleRef<'a, T>) -> impl Iterator<Item = (RectangleRef<'a, T>, &'a V)> {
        self.query_bounds(rect.0, rect.1)
    }

    pub fn query_bounds<'a, R, S>(&'a self, r: &'a R, s: &'a S) -> impl Iterator<Item = (RectangleRef<'a, T>, &'a V)>
    where
        R: RangeBounds<T>,
        S: RangeBounds<T>,
    {
        self.map
            .query_range(r)
            .map(move |(di, l)| l.query_range(s).map(move |(dj, m)| ((di, dj), m)))
            .flatten()
    }
}




// ============================================================================
impl<T: Ord + Copy, V> Default for RectangleMap<T, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + Copy, V> IntoIterator for RectangleMap<T, V> {
    type Item = (Rectangle<T>, V);
    type IntoIter = impl Iterator<Item = (Rectangle<T>, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_iter()
    }
}

impl<'a, T: Ord + Copy, V> IntoIterator for &'a RectangleMap<T, V> {
    type Item = (RectangleRef<'a, T>, &'a V);
    type IntoIter = impl Iterator<Item = (RectangleRef<'a, T>, &'a V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Ord + Copy, V> IntoIterator for &'a mut RectangleMap<T, V> {
    type Item = (RectangleRef<'a, T>, &'a mut V);
    type IntoIter = impl Iterator<Item = (RectangleRef<'a, T>, &'a mut V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use super::RectangleMap;

    #[test]
    fn can_query_points() {
        let mut rect_map = RectangleMap::new();

        rect_map.insert((0..10, 0..10), 1);
        rect_map.insert((20..30, 20..30), 2);
        rect_map.insert((9..21, 9..21), 3);

        assert_eq!(rect_map.query_point((&5, &12)).count(), 0);
        assert_eq!(rect_map.query_point((&5, &5)).count(), 1);
        assert_eq!(rect_map.query_point((&2, &2)).count(), 1);
        assert_eq!(rect_map.query_point((&12, &12)).count(), 1);
    }
}
