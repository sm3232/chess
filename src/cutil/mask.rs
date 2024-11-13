use std::u64;

use crate::cutil::point::Point;

#[derive(Clone,Copy)]
pub struct Mask {
    pub raw: u64
}


impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bv = &mut self.raw.to_ne_bytes();
        let mut str = "\n".to_owned();
        for byte in bv.iter() {
            let x = byte.reverse_bits();
            str.push_str(&format!("{x:08b}\n"));
        }
        return write!(f, "{str}");
    }

}

impl Default for Mask {
    fn default() -> Self {
        return Mask { raw: 0u64 };
    }
}
impl std::ops::BitXor<Point> for Mask {
    type Output = Mask;
    fn bitxor(self, rhs: Point) -> Mask {
        Mask { raw: Mask::from_point(rhs).raw ^ self.raw }
    }
}
impl std::ops::BitXor<u64> for Mask {
    type Output = Mask;
    fn bitxor(self, rhs: u64) -> Mask {
        Mask { raw: self.raw ^ rhs }
    }
}
impl std::ops::BitXor<Mask> for Mask {
    type Output = Mask;
    fn bitxor(self, rhs: Mask) -> Mask {
        Mask { raw: self.raw ^ rhs.raw }
    }
}
impl std::ops::BitXorAssign<Mask> for Mask {
    fn bitxor_assign(&mut self, rhs: Mask) {
        self.raw ^= rhs.raw;
    }
}
impl std::ops::BitXorAssign<u64> for Mask {
    fn bitxor_assign(&mut self, rhs: u64) {
        self.raw ^= rhs;
    }
}
impl std::ops::BitXorAssign<Point> for Mask {
    fn bitxor_assign(&mut self, rhs: Point) {
        *self ^= Mask::from_point(rhs);
    }
}
impl std::ops::BitOrAssign<Mask> for Mask {
    fn bitor_assign(&mut self, rhs: Mask) {
        self.raw |= rhs.raw;
    }
}
impl std::ops::BitOrAssign<u64> for Mask {
    fn bitor_assign(&mut self, rhs: u64) {
        self.raw |= rhs;
    }
}
impl std::ops::BitOrAssign<Point> for Mask {
    fn bitor_assign(&mut self, rhs: Point) {
        *self |= Mask::from_point(rhs);
    }
}

impl std::ops::BitOr<Mask> for Mask {
    type Output = Mask;
    fn bitor(self, rhs: Mask) -> Mask {
        return Mask { raw: self.raw | rhs.raw };
    }
}
impl std::ops::BitOr<u64> for Mask {
    type Output = Mask;
    fn bitor(self, rhs: u64) -> Mask {
        return Mask { raw: self.raw | rhs };
    }
}
impl std::ops::BitOr<Point> for Mask {
    type Output = Mask;
    fn bitor(self, rhs: Point) -> Mask {
        return self | Mask::from_point(rhs);
    }
}

impl std::ops::BitAndAssign<Mask> for Mask {
    fn bitand_assign(&mut self, rhs: Mask) {
        self.raw |= rhs.raw;
    }
}
impl std::ops::BitAndAssign<u64> for Mask {
    fn bitand_assign(&mut self, rhs: u64) {
        self.raw |= rhs;
    }
}
impl std::ops::BitAndAssign<Point> for Mask {
    fn bitand_assign(&mut self, rhs: Point) {
        return *self &= Mask::from_point(rhs);
    }
}


impl std::ops::BitAnd<Mask> for Mask {
    type Output = Mask;
    fn bitand(self, rhs: Mask) -> Mask {
        return Mask { raw: self.raw & rhs.raw };
    }
}
impl std::ops::BitAnd<u64> for Mask {
    type Output = Mask;
    fn bitand(self, rhs: u64) -> Mask {
        return Mask { raw: self.raw & rhs };
        
    }
}
impl std::ops::BitAnd<Point> for Mask {
    type Output = Mask;
    fn bitand(self, rhs: Point) -> Mask {
        return self & Mask::from_point(rhs);
    }
}


impl Mask {
    pub fn shiftl(&mut self, n: i32){
        self.raw <<= n;
    }
    pub fn shiftr(&mut self, n: i32){
        self.raw >>= n;
    }
    pub fn any(&self) -> bool {
        return self.raw > 0;
    }
    pub fn none(&self) -> bool {
        return !self.any();
    }
    pub fn not(&mut self) -> Self {
        self.raw = !self.raw;
        return *self;
    }
    pub fn get_not(&self) -> Self {
        Mask { raw: !self.raw }
    }

    pub fn to_point_vector(&self) -> Vec<Point> {
        let bv = &mut self.raw.to_ne_bytes();
        let mut v: Vec<Point> = Vec::new();
        
        for (index, byte) in bv.iter().enumerate() {
            for bit in 0..8 {
                if byte & (1 << bit) != 0 {
                    v.push(Point { x: bit, y: index as i32 });
                }
            }
        }
        return v;
    }
    pub fn to_point(&self) -> Option<Point> {
        let bv = &mut self.raw.to_ne_bytes();
        for (index, byte) in bv.iter().enumerate() {
            for bit in 0..8 {
                if byte & (1 << bit) != 0 {
                    return Some(Point { x: bit, y: index as i32 });
                }
            }
        }
        return None;
    }
    pub fn isolated_bits(&self) -> Vec<Mask> {
        let mut rv = Vec::new();
        let bv = &self.raw.to_ne_bytes();
        for (index, byte) in bv.iter().enumerate() {
            for bit in 0..8 {
                if byte & (1 << bit) != 0 {

                    rv.push(Mask{raw: 1u64 << (index * 8) << bit});
                }
            }
        }
        return rv;

    }



    pub fn from_point(point: Point) -> Mask {
        if point.x > 7 || point.y > 7 { return Mask::default() };
        let mut mask = Mask { raw: 1u64 };
            mask.shiftl(8 * (point.y));
            mask.shiftl(1 * (point.x));
        return mask;
    }
    pub fn of_column(col: i32) -> Mask {
        let bit = 1u64 << col;
        let mut mask = Mask { raw: bit };
        for _ in 0..7 {
            mask.shiftl(8);
            mask |= bit;
        }
        return mask;
    }

}
