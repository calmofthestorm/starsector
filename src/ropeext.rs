use std::borrow::Cow;
use std::ops::{Bound, RangeBounds};

use ropey::{Rope, RopeSlice};

macro_rules! common_rope_ext_trait {
    () => {
        fn to_contiguous<'a>(&'a self) -> Cow<'a, str>;
        fn discontangle<'a, 'b>(&'a self, contiguous: &'b str, s: &'b str) -> RopeSlice<'a>;
        fn is_empty(&self) -> bool;
        fn slice_bytes<R>(&self, byte_range: R) -> RopeSlice
        where
            R: RangeBounds<usize>;

        /// This searches *bytes*, not *chars*, and returns a byte index. Be very
        /// careful with Unicode.
        fn memchr(&self, needle: u8, offset: usize) -> usize;
    };
}

pub trait RopeExt {
    fn push(&mut self, c: char);
    fn push_str(&mut self, s: &str);
    fn push_string(&mut self, s: String);
    common_rope_ext_trait!();
}

pub trait RopeSliceExt {
    common_rope_ext_trait!();
}

macro_rules! common_rope_ext_impl {
    () => {
        fn to_contiguous<'a>(&'a self) -> Cow<'a, str> {
            let mut it = self.chunks();
            match it.next() {
                None => Cow::default(),
                Some(chunk) if it.next().is_none() => chunk.into(),
                _ => self.to_string().into(),
            }
        }

        // FIXME: Wrap this in a Contiguous type to make it safer.
        fn discontangle<'a, 'b>(&'a self, contiguous: &'b str, s: &'b str) -> RopeSlice<'a> {
            if s.is_empty() {
                return self.slice(0..0);
            }

            let offset = s.as_ptr() as usize - contiguous.as_ptr() as usize;
            let start = self.byte_to_char(offset);
            let end = self.byte_to_char(offset + s.len());
            self.slice(start..end)
        }

        fn is_empty(&self) -> bool {
            return self.len_bytes() == 0;
        }

        fn slice_bytes<R>(&self, byte_range: R) -> RopeSlice
        where
            R: RangeBounds<usize>,
        {
            match (byte_range.start_bound(), byte_range.end_bound()) {
                (Bound::Included(start), Bound::Excluded(end)) => {
                    self.slice(self.byte_to_char(*start)..self.byte_to_char(*end))
                }
                (Bound::Included(start), Bound::Included(end)) => {
                    self.slice(self.byte_to_char(*start)..=self.byte_to_char(*end))
                }
                (Bound::Excluded(start), Bound::Included(end)) => {
                    self.slice(self.byte_to_char(*start + 1)..=self.byte_to_char(*end))
                }
                (Bound::Excluded(start), Bound::Excluded(end)) => {
                    self.slice(self.byte_to_char(*start + 1)..self.byte_to_char(*end))
                }
                (Bound::Unbounded, Bound::Unbounded) => self.slice(..),
                (Bound::Unbounded, Bound::Included(end)) => self.slice(..=self.byte_to_char(*end)),
                (Bound::Unbounded, Bound::Excluded(end)) => self.slice(..self.byte_to_char(*end)),
                (Bound::Included(start), Bound::Unbounded) => {
                    self.slice(self.byte_to_char(*start)..)
                }
                (Bound::Excluded(start), Bound::Unbounded) => {
                    self.slice(self.byte_to_char(*start + 1)..)
                }
            }
        }

        fn memchr(&self, needle: u8, offset: usize) -> usize {
            let mut bygones = offset;
            let (chunks, chunk_start, _, _) = self.chunks_at_byte(offset);
            let mut skip = offset - chunk_start;

            for mut chunk in chunks {
                if skip > 0 {
                    chunk = &chunk[skip..];
                    skip = 0;
                }

                match memchr::memchr(needle, &chunk.as_bytes()) {
                    Some(index) => return index + bygones,
                    None => {
                        bygones += chunk.len();
                    }
                }
            }

            bygones
        }
    };
}

impl RopeExt for Rope {
    fn push(&mut self, c: char) {
        let pos = self.len_chars();
        self.insert_char(pos, c);
    }

    fn push_str(&mut self, s: &str) {
        let pos = self.len_chars();
        self.insert(pos, s);
    }

    fn push_string(&mut self, s: String) {
        self.append(Rope::from(s));
    }

    common_rope_ext_impl!();
}

impl RopeSliceExt for RopeSlice<'_> {
    common_rope_ext_impl!();
}
