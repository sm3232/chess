use crate::lib::point::Point;
#[derive(Clone,Copy)]
pub struct Mask {
    pub raw: u64
}

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bv = &mut self.raw.to_le_bytes();
        let mut str = "\n".to_owned();
        for byte in bv.iter() {
            let x = byte.reverse_bits();
            str.push_str(&format!("{x:08b}\n"));
        }
        return write!(f, "{str}");
    }
}

impl Default for Mask {
    fn default() -> Self { Self { raw: 0u64 } }
}

#[allow(dead_code)]
impl Mask {
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

    pub fn to_painter_rect(&self, sqsize: f32) -> eframe::egui::Rect {
        return eframe::egui::Rect {
            min: (self.to_point().unwrap_or(Point { x: -100, y: -100 }) * sqsize).into(),
            max: ((self.to_point().unwrap_or(Point { x: -100, y: -100 }) + Point { x: 1, y: 1 }) * sqsize).into()
        };
    }

    pub fn to_point_or_00(&self) -> String {
        let point = self.to_point();
        if let Some(p) = point {
            return format!("{p}");
        } else {
            return "(,)".to_string();
        }
    }

    pub fn str(&self) -> String {
        let bv = &mut self.raw.to_ne_bytes();
        let mut str = "\n".to_owned();
        for byte in bv.iter() {
            let x = byte.reverse_bits();
            str.push_str(&format!("{x:08b}\n"));
        }
        return format!("{str}");
    }

    pub fn get_y_gap<'a>(m1: &'a Mask, m2: &'a Mask) -> (usize, &'a Mask) {
        let (mut from, to, low) = if m1.raw < m2.raw {
            (m1.raw, m2.raw, m1)
        } else if m1.raw > m2.raw {
            (m2.raw, m1.raw, m2)
        } else {
            return (0, m1);
        };
        let mut gap = 0usize;
        return loop {
            from <<= 8;
            gap += 1;
            if from > to {
                break (gap, low);
            }
            if gap > 8 { panic!("Y Gap > 8") };
        };
    }

    pub fn get_x_gap(m1: &Mask, m2: &Mask) -> usize {
        let (mut from, to) = if (m1.raw % 8) < (m2.raw % 8) {
            (m1.raw, m2.raw)
        } else if (m1.raw % 8) > (m2.raw % 8) {
            (m2.raw, m1.raw)
        } else {
            return 0;
        };
        let mut gap = 0usize;
        return loop {
            from <<= 1;
            gap += 1;
            if from > to {
                break gap;
            }
            if gap > 8 { panic!("X Gap > 8") };
        };
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

    pub fn point_add(mask: &Mask, point: &Point) -> Mask {
        return if let Some(mask_as_point) = &mask.to_point() {
            Mask::from_point(mask_as_point + point)
        } else {
            Mask::default()
        };
    }

    pub fn from_point(point: Point) -> Mask {
        if point.x > 7 || point.y > 7 || point.x < 0 || point.y < 0 { return Mask::default() };
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
    
    pub fn as_index(&self) -> usize {
        let bv = &mut self.raw.to_ne_bytes();
        for (index, byte) in bv.iter().enumerate() {
            for bit in 0..8 {
                if byte & (1 << bit) != 0 {
                    return (index * 8 + bit) as usize;
                }
            }
        }
        return 0usize;
    }

    pub fn from_castle_bytes(bytes: u8) -> Self {
        let mut m = Self { raw: 0u64 };
        if (bytes & 0b0000_0001) != 0 { m |= Mask::from_point(Point {x: 7, y: 0}) };
        if (bytes & 0b0000_0010) != 0 { m |= Mask::from_point(Point {x: 0, y: 0}) };
        if (bytes & 0b0000_0100) != 0 { m |= Mask::from_point(Point {x: 7, y: 7}) };
        if (bytes & 0b0000_1000) != 0 { m |= Mask::from_point(Point {x: 0, y: 7}) };
        return m;
    }

    #[inline(always)]
    pub fn from_index(index: usize) -> Mask { Mask { raw: (1u64 << index) } }

    #[inline(always)]
    pub fn bit_count(&self) -> u32 { self.raw.count_ones() }

    #[inline(always)]
    pub fn shiftl(&mut self, n: i32){ self.raw <<= n }

    #[inline(always)]
    pub fn shiftr(&mut self, n: i32){ self.raw >>= n }

    #[inline(always)]
    pub fn shiftup(&mut self, n: usize) { self.raw <<= n * 8 }

    #[inline(always)]
    pub fn shiftdown(&mut self, n: usize) { self.raw >>= n * 8 }

    #[inline(always)]
    pub fn any(&self) -> bool { self.raw > 0 }

    #[inline(always)]
    pub fn none(&self) -> bool { !self.any() }

    #[inline(always)]
    pub fn get_not(&self) -> Self { Mask { raw: !self.raw } }

    #[inline(always)]
    pub fn not(&mut self) -> Self {
        self.raw = !self.raw;
        return *self;
    }
}


impl PartialEq<Point> for Mask {
    #[inline(always)]
    fn eq(&self, other: &Point) -> bool {
        return Mask::from_point(*other).raw == self.raw;
    }
}
impl PartialEq<usize> for Mask {
    #[inline(always)]
    fn eq(&self, other: &usize) -> bool {
        return self.bit_count() == 1 && (
            (1u64 << other) == self.raw
        );
    }
}
impl PartialEq<Mask> for Mask {
    #[inline(always)]
    fn eq(&self, other: &Mask) -> bool {
        return self.raw == other.raw;
        
    }
}


/*      XOR     */
impl std::ops::BitXor<Point> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Point) -> Self {
        Self { raw: Self::from_point(rhs).raw ^ self.raw }
    }
}
impl std::ops::BitXor<u64> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: u64) -> Self {
        Self { raw: self.raw ^ rhs }
    }
}
impl std::ops::BitXor<Mask> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self {
        Self { raw: self.raw ^ rhs.raw }
    }
}

/*      XOR ASSIGN     */
impl std::ops::BitXorAssign<Point> for Mask {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Point) {
        self.raw ^= Self::from_point(rhs).raw;
    }
}
impl std::ops::BitXorAssign<u64> for Mask {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: u64) {
        self.raw ^= rhs;
    }
}
impl std::ops::BitXorAssign<Mask> for Mask {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.raw ^= rhs.raw;
    }
}

/*      OR      */
impl std::ops::BitOr<Point> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Point) -> Self {
        return self | Self::from_point(rhs);
    }
}
impl std::ops::BitOr<u64> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: u64) -> Self {
        return Self { raw: self.raw | rhs };
    }
}
impl std::ops::BitOr<Mask> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        return Self { raw: self.raw | rhs.raw };
    }
}

/*      OR ASSIGN       */
impl std::ops::BitOrAssign<Point> for Mask {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Point) {
        self.raw |= Self::from_point(rhs).raw;
    }
}
impl std::ops::BitOrAssign<u64> for Mask {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: u64) {
        self.raw |= rhs;
    }
}
impl std::ops::BitOrAssign<Mask> for Mask {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.raw |= rhs.raw;
    }
}
impl std::ops::BitOrAssign<usize> for Mask {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: usize) { self.raw |= Mask::from_index(rhs).raw }
}

/*      AND      */
impl std::ops::BitAnd<Point> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Point) -> Self {
        return self & Self::from_point(rhs);
    }
}
impl std::ops::BitAnd<u64> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: u64) -> Self {
        return Self { raw: self.raw & rhs };
        
    }
}
impl std::ops::BitAnd<Mask> for Mask {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self {
        return Self { raw: self.raw & rhs.raw };
    }
}

/*      AND ASSIGN      */
impl std::ops::BitAndAssign<Point> for Mask {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Point) {
        return self.raw &= Self::from_point(rhs).raw;
    }
}
impl std::ops::BitAndAssign<u64> for Mask {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: u64) {
        self.raw |= rhs;
    }
}
impl std::ops::BitAndAssign<Mask> for Mask {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.raw |= rhs.raw;
    }
}
