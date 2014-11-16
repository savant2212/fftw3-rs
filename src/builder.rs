use ffi;
use libc::{c_uint, c_int, c_void};
use num::complex::Complex64;

use plan::RawPlan;

/// How much effort FFTW should put into computing the best strategy
/// to use.
///
/// The `FFTW_WISDOM_ONLY` rigor level is replaced by the
pub enum Rigor {
    Estimate,
    Measure,
    Patient,
    Exhaustive,
}
impl Rigor {
    fn flags(self) -> c_uint {
        match self {
            Estimate => ffi::FFTW_ESTIMATE,
            Measure => ffi::FFTW_MEASURE,
            Patient => ffi::FFTW_PATIENT,
            Exhaustive => ffi::FFTW_EXHAUSTIVE,
        }
    }
}

/// The direction of the transform to perform..
pub enum Direction {
    Forward, Backward
}

/// Control the basic properties of a set of transforms.
pub struct Planner {
    rigor: Rigor,
    wisdom_restriction: bool,
    direction: Direction,
}

impl Planner {
    /// Construct a new planner with default values.
    ///
    /// This defaults to a forward transform with estimate rigor.
    pub fn new() -> Planner {
        Planner {
            rigor: Estimate,
            wisdom_restriction: false,
            direction: Forward,
        }
    }

    /// Set the rigor to use for this plan.
    pub fn rigor(&mut self, r: Rigor) -> &mut Planner {
        self.rigor = r;
        self
    }
    /// Set whether the planner should only be successfully created if
    /// there exists wisdom created with at least the rigor level set.
    pub fn wisdom_restriction(&mut self, wisdom_only: bool) -> &mut Planner {
        self.wisdom_restriction = wisdom_only;
        self
    }

    /// Set the direction of the transform to perform.
    pub fn direction(&mut self, direction: Direction) -> &mut Planner {
        self.direction = direction;
        self
    }

    fn flags(&self) -> c_uint {
        self.rigor.flags() | if self.wisdom_restriction {
            ffi::FFTW_WISDOM_ONLY
        } else {
            0
        }
    }
    fn dir(&self) -> c_int {
        match self.direction {
            Forward => ffi::FFTW_FORWARD,
            Backward => ffi::FFTW_BACKWARD,
        }
    }

    pub fn inplace(&self) -> InPlacePlanner {
        InPlacePlanner { plan: *self }
    }

    pub fn c2c<I, O>(&self, in_: I, out: O) -> PlanMem<I, O>
        where I: DerefMut<[Complex64]>, O: DerefMut<[Complex64]>
    {
        assert!(in_.len() <= 0x7F_FF_FF_FF);
        assert!(in_.len() <= out.len());
        let dims = Detailed(vec![Dim { n: in_.len(), in_stride: 1, out_stride: 1 }]);
        PlanMem {
            plan: *self,
            in_: in_,
            out: Some(out),
            planner: c2c,

            dims: dims,
            how_many: Contiguous(vec![1]),
        }
    }
    pub fn c2r<I, O>(&self, in_: I, out: O) -> PlanMem<I, O>
        where I: DerefMut<[Complex64]>, O: DerefMut<[f64]>
    {
        assert!(in_.len() <= 0x7F_FF_FF_FF);
        assert!(in_.len() <= out.len() / 2 + 1);
        let dims = Detailed(vec![Dim { n: 2 * (in_.len() - 1),
                                       in_stride: 1, out_stride: 1 }]);
        PlanMem {
            plan: *self,
            in_: in_,
            out: Some(out),
            planner: c2r,

            dims: dims,
            how_many: Contiguous(vec![1]),
        }
    }
    pub fn r2c<I, O>(&self, in_: I, out: O) -> PlanMem<I, O>
        where I: DerefMut<[f64]>, O: DerefMut<[Complex64]>
    {
        assert!(in_.len() <= 0x7F_FF_FF_FF);
        assert!(in_.len() / 2 + 1 <= out.len());
        let dims = Detailed(vec![Dim { n: in_.len(), in_stride: 1, out_stride: 1 }]);
        PlanMem {
            plan: *self,
            in_: in_,
            out: Some(out),
            planner: r2c,

            dims: dims,
            how_many: Contiguous(vec![1]),
        }
    }
    #[cfg(r2r_is_hard)]
    pub fn r2r<I, O>(&self, in_: I, out: O) -> PlanMem<I, O>
        where I: DerefMut<[f64]>, O: DerefMut<[f64]>
    {
        assert!(in_.len() <= 0x7F_FF_FF_FF);
        assert!(in_.len() <= out.len());
        let dims = Detailed(vec![Dim { n: in_.len(), in_stride: 1, out_stride: 1 }]);
        PlanMem {
            plan: *self,
            in_: in_,
            out: Some(out),
            planner: r2r,

            dims: dims,
            how_many: Contiguous(vec![1]),
        }
    }
}

pub struct InPlacePlanner {
    plan: Planner
}

impl InPlacePlanner {
    pub fn c2c<I>(&self, in_: I) -> PlanMem<I, I>
        where I: DerefMut<[Complex64]>
    {
        assert!(in_.len() <= 0x7F_FF_FF_FF);
        let dims = Detailed(vec![Dim { n: in_.len(), in_stride: 1, out_stride: 1 }]);
        PlanMem {
            plan: self.plan,
            in_: in_,
            out: None,
            planner: c2c,

            dims: dims,
            how_many: Contiguous(vec![1]),
        }
    }
}

type GuruPlanner =
    unsafe fn(rank: c_int, dims: *const ffi::fftw_iodim64,
              howmany_rank: c_int, howmany_dims: *const ffi::fftw_iodim64,
              in_: *mut c_void, out: *mut c_void,
              sign: c_int, flags: c_uint) -> ffi::fftw_plan;

unsafe fn c2c(rank: c_int, dims: *const ffi::fftw_iodim64,
              howmany_rank: c_int, howmany_dims: *const ffi::fftw_iodim64,
              in_: *mut c_void, out: *mut c_void,
              sign: c_int, flags: c_uint) -> ffi::fftw_plan {
    ffi::fftw_plan_guru64_dft(rank, dims,
                            howmany_rank, howmany_dims,
                            in_ as *mut _, out as *mut _,
                            sign, flags)
}

unsafe fn r2c(rank: c_int, dims: *const ffi::fftw_iodim64,
              howmany_rank: c_int, howmany_dims: *const ffi::fftw_iodim64,
              in_: *mut c_void, out: *mut c_void,
              _sign: c_int, flags: c_uint) -> ffi::fftw_plan {
    ffi::fftw_plan_guru64_dft_r2c(rank, dims,
                                howmany_rank, howmany_dims,
                                in_ as *mut _, out as *mut _,
                                flags)
}
unsafe fn c2r(rank: c_int, dims: *const ffi::fftw_iodim64,
              howmany_rank: c_int, howmany_dims: *const ffi::fftw_iodim64,
              in_: *mut c_void, out: *mut c_void,
              _sign: c_int, flags: c_uint) -> ffi::fftw_plan {
    ffi::fftw_plan_guru64_dft_c2r(rank, dims,
                                howmany_rank, howmany_dims,
                                in_ as *mut _, out as *mut _,
                                flags)
}
#[cfg(r2r_is_hard)]
unsafe fn r2r(n: c_int, in_: *mut c_void, out: *mut c_void,
              sign: c_int, flags: c_uint) -> ffi::fftw_plan {
    ffi::fftw_plan_r2r_1d(n,
                          in_ as *mut f64, out as *mut f64,
                          sign, flags)
}


#[repr(C)]
pub struct Dim {
    pub n: uint,
    pub in_stride: uint,
    pub out_stride: uint,
}

enum Dims {
    Contiguous(Vec<uint>),
    Detailed(Vec<Dim>),
}

pub struct PlanMem<I, O> {
    plan: Planner,
    dims: Dims,
    how_many: Dims,
    in_: I,
    out: Option<O>,
    planner: GuruPlanner
}

impl<X, Y, I: DerefMut<[X]>, O: DerefMut<[Y]>> PlanMem<I, O> {
    pub fn dimensions(mut self, dims: Vec<uint>) -> PlanMem<I, O> {
        unimplemented!()
        self.dims = Contiguous(dims);
        self
    }
    pub fn multiples(mut self, number: uint) -> PlanMem<I, O> {
        unimplemented!()
        self.how_many = Contiguous(vec![number]);
        self
    }
    pub fn plan(mut self) -> Result<Planned<I, O>, PlanMem<I, O>> {
        let plan;
        {
            let in_ptr = self.in_.as_mut_ptr() as *mut c_void;
            let out_ptr = match self.out {
                None => in_ptr,
                Some(ref mut o) => o.as_mut_ptr() as *mut c_void,
            };
            let dims = match self.dims {
                Contiguous(_) => unimplemented!(),
                Detailed(ref v) => v.as_slice()
            };
            assert!(dims.len() == 1);

            plan = RawPlan::new(|| unsafe {
                (self.planner)(
                    dims.len() as c_int, dims.as_ptr() as *const ffi::fftw_iodim64,
                    0, [].as_ptr(),
                    in_ptr,
                    out_ptr,
                    self.plan.dir(),
                    self.plan.flags())
            });
        }
        match plan {
            None => Err(self),
            Some(p) => Ok(Planned { mem: self, plan: p })
        }
    }
}

pub struct Planned<I, O> {
    mem: PlanMem<I, O>,
    plan: RawPlan,
}

impl<X, Y, I: DerefMut<[X]>, O: DerefMut<[Y]>> Planned<I, O> {
    pub fn input(&mut self) -> &mut [X] {
        &mut *self.mem.in_
    }
    pub fn output(&mut self) -> Option<&mut [Y]> {
        self.mem.out.as_mut().map(|o| &mut **o)
    }

    pub fn execute(&mut self) {
        unsafe {
            self.plan.execute()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use libc::ptrdiff_t;
    use ffi::Struct_fftw_iodim64_do_not_use_me;
    use super::Dim;

    #[test]
    fn iodims_are_compatible() {
        // handle 32-bit and 64-bit platforms properly
        let n = 0x0102_0304_0506_0708u64 as uint;
        let is = 0x090A_0B0C_0D0E_0F00u64 as uint;
        let os = 0x1122_3344_5566_7788u64 as uint;

        let d = Dim { n: n, in_stride: is, out_stride: os };
        let f = Struct_fftw_iodim64_do_not_use_me {
            n: n as ptrdiff_t, is: is as ptrdiff_t, os: os as ptrdiff_t
        };
        type T = (uint, uint, uint);
        unsafe {
            assert_eq!(mem::transmute::<_, T>(d), mem::transmute::<_, T>(f));
        }
    }
}
