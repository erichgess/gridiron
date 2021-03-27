use core::iter::FromIterator;
use core::ops::{Range, RangeBounds};
use crate::interval_map::IntervalMap;




/// Type alias for a 2d range
pub type Rectangle<T> = (Range<T>, Range<T>);




/// Type alias for a 2d range, by-reference
pub type RectangleRef<'a, T> = (&'a Range<T>, &'a Range<T>);




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

    pub fn insert<I>(&mut self, space: I, value: V) -> &mut V
    where
        I: Into<Rectangle<T>>
    {
        let (di, dj) = space.into();
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
        Self {
            map: self.map
                .into_sorted()
                .map(|(k, m)| (k, m.into_balanced()))
                .collect()
        }
    }

    fn into_iter_internal(self) -> impl Iterator<Item = (Rectangle<T>, V)> {
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

    pub fn query_point(
        &self,
        point: (T, T)) -> impl Iterator<Item = (RectangleRef<T>, &V)> {
        self.map
            .query_point(point.0)
            .map(move |(di, l)| l.query_point(point.1).map(move |(dj, m)| ((di, dj), m)))
            .flatten()
    }

    pub fn query_rect<I>(
        &self,
        space: I) -> impl Iterator<Item = (RectangleRef<T>, &V)>
    where
        I: Into<Rectangle<T>>
    {
        let rect = space.into();
        self.query_bounds(rect.0, rect.1)
    }

    pub fn query_bounds<R, S>(
        &self,
        r: R,
        s: S) -> impl Iterator<Item = (RectangleRef<T>, &V)>
    where
        R: RangeBounds<T> + Clone,
        S: RangeBounds<T> + Clone,
    {
        self.map
            .query_range(r)
            .map(move |(di, l)| l.query_range(s.clone()).map(move |(dj, m)| ((di, dj), m)))
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
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_iter_internal()
    }
}

impl<'a, T: Ord + Copy, V> IntoIterator for &'a RectangleMap<T, V> {
    type Item = (RectangleRef<'a, T>, &'a V);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Ord + Copy, V> IntoIterator for &'a mut RectangleMap<T, V> {
    type Item = (RectangleRef<'a, T>, &'a mut V);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'a, T: 'a + Ord + Copy, V> FromIterator<(RectangleRef<'a, T>, V)> for RectangleMap<T, V> {
    fn from_iter<I: IntoIterator<Item = (RectangleRef<'a, T>, V)>>(iter: I) -> Self {
        let mut result = Self::new();

        for (rect, item) in iter {
            result.insert((rect.0.clone(), rect.1.clone()), item);
        }
        result
    }
}

impl<T: Ord + Copy, V> FromIterator<(Rectangle<T>, V)> for RectangleMap<T, V> {
    fn from_iter<I: IntoIterator<Item = (Rectangle<T>, V)>>(iter: I) -> Self {
        let mut result = Self::new();

        for (rect, item) in iter {
            result.insert(rect, item);
        }
        result
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

        assert_eq!(rect_map.query_point((5, 12)).count(), 0);
        assert_eq!(rect_map.query_point((5, 5)).count(), 1);
        assert_eq!(rect_map.query_point((2, 2)).count(), 1);
        assert_eq!(rect_map.query_point((12, 12)).count(), 1);
    }
}
