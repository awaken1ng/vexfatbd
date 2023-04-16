pub struct Chain<T>
where
    T: Iterator,
{
    slices: Vec<T>,
}

impl<T> Chain<T>
where
    T: Iterator,
{
    pub fn new(slices: Vec<T>) -> Self {
        Self { slices }
    }
}

impl<T> Iterator for Chain<T>
where
    T: Iterator,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        for slice in self.slices.iter_mut() {
            if let Some(a) = slice.next() {
                return Some(a);
            }
        }

        None
    }
}

pub fn unsigned_rounded_up_div<T>(a: T, b: T) -> T
where
    T: num_traits::Unsigned,
{
    a.sub(T::one()).div(b).add(T::one())
}

pub fn unsigned_align_to<T>(a: T, b: T) -> T
where
    T: num_traits::Unsigned + Copy,
{
    unsigned_rounded_up_div(a, b).mul(b)
}

#[test]
fn chain() {
    let chain = Chain::new(vec![b"123".iter(), b"456".iter()]);
    let out: Vec<_> = chain.skip(2).take(3).cloned().collect();
    assert_eq!(out, b"345");
}

#[test]
fn rounding_up() {
    assert_eq!(unsigned_rounded_up_div(5u32, 1), 5);
    assert_eq!(unsigned_rounded_up_div(5u32, 2), 3);
    assert_eq!(unsigned_rounded_up_div(5u32, 3), 2);
    assert_eq!(unsigned_rounded_up_div(5u32, 4), 2);
    assert_eq!(unsigned_rounded_up_div(5u32, 5), 1);
}

#[test]
fn alignment() {
    assert_eq!(unsigned_align_to(5u32, 8), 8);
    assert_eq!(unsigned_align_to(15u32, 8), 16);
}
