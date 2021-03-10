#![allow(unused)]


#[derive(Clone, Copy, Debug)]
pub struct Area(i64, i64);


impl Area {

    pub fn with_shape(size: usize) -> Self {
        Self(0, size as i64)
    }

    pub fn covering(i0: usize, i1: usize) -> Self {
        Self(i0 as i64, i1 as i64)
    }

    pub fn contains(&self, i: i64) -> bool {
        self.0 <= i && i < self.1
    }

    pub fn expand(&self, amount: i64) -> Self {
        Self(self.0 - amount, self.1 + amount)
    }

    pub fn iter(&self) -> impl Iterator<Item = i64> {
        self.0..self.1
    }
}


#[derive(Clone)]
pub struct Patch {
    pub area: Area,
    pub data: Vec<f64>,
}


impl Patch {

}


#[derive(Clone)]
pub struct Quilt {
    pub area: Area,
    pub patches: Vec<Patch>
}


impl Quilt {

    pub fn new(area: Area) -> Self {
        Self {
            area,
            patches: Vec::new(),
        }
    }

    fn patches_containing(&self, i: i64) -> impl Iterator<Item = &Patch> {
        self.patches.iter().filter(move |p| p.area.contains(i))
    }

    pub fn sample(&self, mut i: i64) -> f64 {

        while i < 0 {
            i += self.area.1 - self.area.0
        }
        while i >= self.area.1 {
            i -= self.area.1 - self.area.0
        }

        let patch = self.patches_containing(i).next().unwrap();
        patch.data[(i - patch.area.0) as usize]
    }

    pub fn fabricate_patch(&self, area: Area) -> Patch {
        Patch {
            area,
            data: (area.0..area.1).map(|i| self.sample(i)).collect()
        }
    }

    pub fn insert_with_function<F>(&mut self, area: Area, f: F)
    where
        F: Fn(i64) -> f64
    {
        let data: Vec<_> = (area.0..area.1).map(f).collect();
        let patch = Patch { area, data };
        self.patches.push(patch)
    }

    pub fn insert(&mut self, patch: Patch) {
        self.patches.push(patch)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Patch> {
        self.patches.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Patch> {
        self.patches.iter_mut()
    }
}
