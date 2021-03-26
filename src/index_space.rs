use core::ops::Range;




#[derive(Clone)]
pub struct IndexSpace2d {
    di: Range<i64>,
    dj: Range<i64>,
}




impl IndexSpace2d {

    pub fn new(di: Range<i64>, dj: Range<i64>) -> Self {
        Self { di, dj }
    }

    pub fn extend(&self, delta: i64) -> Self {
        Self::new(
            self.di.start - delta .. self.di.end + delta,
            self.dj.start - delta .. self.dj.end + delta)
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        self.clone().into_iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = (i64, i64)> {
        let Self { di, dj } = self;
        di.map(move |i| dj.clone().map(move |j| (i, j))).flatten()
    }
}
