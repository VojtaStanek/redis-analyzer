pub trait Tree {
    type Children: IntoIterator<Item = Self>;

    fn children(&self) -> Self::Children;
}
