#![allow(unused)]

use super::error::Error;
use super::geometry::{Direction, Vector3d};
use std::ops::{Add, Div, Mul, Sub};

pub trait Conserved {
    fn mass_density(&self) -> f64;
    fn momentum_1(&self) -> f64;
    fn momentum_2(&self) -> f64;
    fn momentum_3(&self) -> f64;
    fn energy_density(&self) -> f64;

    fn momentum_vector(&self) -> Vector3d {
        Vector3d::new(self.momentum_1(), self.momentum_2(), 0.0)
    }

    fn momentum(&self, direction: Direction) -> f64 {
        match direction {
            Direction::I => self.momentum_1(),
            Direction::J => self.momentum_2(),
            Direction::K => 0.0,
        }
    }

    fn momentum_squared(&self) -> f64 {
        let p1 = self.momentum_1();
        let p2 = self.momentum_2();
        p1 * p1 + p2 * p2
    }

    fn to_primitive_ref(&self, gamma_law_index: f64, prim: &mut PrimitiveRef) -> Result<(), Error> {
        let ek = 0.5 * self.momentum_squared() / self.mass_density();
        let et = self.energy_density() - ek;
        let pg = et * (gamma_law_index - 1.0);
        let v1 = self.momentum_1() / self.mass_density();
        let v2 = self.momentum_2() / self.mass_density();

        Ok(())
        // prim.set_checked(self.mass_density(), v1, v2, pg)

        // if self.mass_density() < 0.0 {
        //     Err(Error::NegativeMassDensity(self.mass_density()))
        // } else if pg < 0.0 {
        //     Err(Error::NegativeGasPressure(pg))
        // } else {
        //     Ok(PrimitiveRef(self.mass_density(), v1, v2, pg))
        // }
    }
}

pub trait Primitive {
    fn mass_density(&self) -> f64;
    fn velocity_1(&self) -> f64;
    fn velocity_2(&self) -> f64;
    fn velocity_3(&self) -> f64;
    fn gas_pressure(&self) -> f64;

    fn velocity(&self, direction: Direction) -> f64 {
        match direction {
            Direction::I => self.velocity_1(),
            Direction::J => self.velocity_2(),
            Direction::K => 0.0,
        }
    }

    fn velocity_squared(&self) -> f64 {
        let v1 = self.velocity_1();
        let v2 = self.velocity_2();
        v1 * v1 + v2 * v2
    }

    fn sound_speed_squared(&self, gamma_law_index: f64) -> f64 {
        gamma_law_index * self.gas_pressure() / self.mass_density()
    }

    fn specific_kinetic_energy(&self) -> f64 {
        0.5 * self.velocity_squared()
    }

    fn specific_internal_energy(&self, gamma_law_index: f64) -> f64 {
        self.gas_pressure() / self.mass_density() / (gamma_law_index - 1.0)
    }

    fn mach_number(&self, gamma_law_index: f64) -> f64 {
        (self.velocity_squared() / self.sound_speed_squared(gamma_law_index)).sqrt()
    }

    fn outer_wavespeeds(&self, direction: Direction, gamma_law_index: f64) -> (f64, f64) {
        let cs = self.sound_speed_squared(gamma_law_index).sqrt();
        let vn = self.velocity(direction);
        (vn - cs, vn + cs)
    }

    fn max_signal_speed(&self, gamma_law_index: f64) -> f64 {
        f64::sqrt(self.velocity_squared()) + f64::sqrt(self.sound_speed_squared(gamma_law_index))
    }

    // fn to_conserved(&self, gamma_law_index: f64) -> Conserved {
    //     let d   = self.mass_density();
    //     let p   = self.gas_pressure();
    //     let vsq = self.velocity_squared();

    //     Conserved(
    //         d,
    //         d * self.velocity_1(),
    //         d * self.velocity_2(),
    //         d * vsq * 0.5 + p / (gamma_law_index - 1.0)
    //     )
    // }

    // fn flux_vector(&self, direction: Direction, gamma_law_index: f64) -> Conserved {
    //     let pg = self.gas_pressure();
    //     let vn = self.velocity(direction);
    //     let u = self.to_conserved(gamma_law_index);

    //     Conserved(
    //          u.0 * vn,
    //          u.1 * vn + pg * direction.along(Direction::I),
    //          u.2 * vn + pg * direction.along(Direction::J),
    //          u.3 * vn + pg * vn)
    // }

    // fn reflect(&self, direction: Direction) -> Primitive {
    //     match direction {
    //         Direction::I => Primitive(self.0, -self.1, self.2, self.3),
    //         Direction::J => Primitive(self.0, self.1, -self.2, self.3),
    //         Direction::K => panic!(),
    //     }
    // }
}

pub struct ConservedRef<'a>(&'a [f64]);
pub struct PrimitiveRef<'a>(&'a [f64]);

impl<'a> Conserved for ConservedRef<'a> {
    fn mass_density(&self) -> f64 {
        self.0[0]
    }

    fn momentum_1(&self) -> f64 {
        self.0[1]
    }

    fn momentum_2(&self) -> f64 {
        self.0[2]
    }

    fn momentum_3(&self) -> f64 {
        0.0
    }

    fn energy_density(&self) -> f64 {
        self.0[3]
    }
}

impl<'a> ConservedRef<'a> {
    fn from_slice(cons: &'a [f64]) -> Self {
        Self(cons)
    }
}

impl<'a> PrimitiveRef<'a> {
    fn from_slice(prim: &'a [f64]) -> Self {
        Self(prim)
    }

    pub fn as_array(&self) -> [f64; 4] {
        [self.0[0], self.0[1], self.0[2], self.0[3]]
    }
}

impl<'a> Primitive for PrimitiveRef<'a> {
    fn mass_density(&self) -> f64 {
        self.0[0]
    }

    fn velocity_1(&self) -> f64 {
        self.0[1]
    }

    fn velocity_2(&self) -> f64 {
        self.0[2]
    }

    fn velocity_3(&self) -> f64 {
        0.0
    }

    fn gas_pressure(&self) -> f64 {
        self.0[3]
    }
}
