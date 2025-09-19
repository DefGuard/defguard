pub trait AsCsv {
    fn as_csv(&self) -> String;
}

impl<T, I> AsCsv for I
where
    I: ?Sized + std::iter::IntoIterator<Item = T>,
    for<'a> &'a I: IntoIterator<Item = &'a T>,
    T: ToString,
{
    fn as_csv(&self) -> String {
        self.into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}
