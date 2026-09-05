pub struct Program {
    buff: Vec<u16>,
    pos: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum RawInst {
    Compressed(u16),
    Normal(u32)
}

impl Iterator for Program {
    type Item = RawInst;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buff.len() {
            return None;
        }
        let v1 = self.buff[self.pos as usize];
        if v1 & 0x3 != 3 {
            self.pos += 1;
            return Some(RawInst::Compressed(v1));
        } else {
            if self.pos+1 >= self.buff.len() {
                return None;
            }
            let v2 = self.buff[self.pos+1 as usize];
            let norm = u32::from_le_bytes([v1 as u8, (v1>>8) as u8, v2 as u8, (v2>>8) as u8]);
            self.pos += 2;
            return Some(RawInst::Normal(norm));
        }
    }
}

impl From<Vec<u16>> for Program {
    fn from(value: Vec<u16>) -> Self {
        Self {buff: value, pos: 0}
    }
}
