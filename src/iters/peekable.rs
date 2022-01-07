use std::fmt;

/// An iterator extension trait that provides the `take_while_p` method for the [`Peekable`]
/// iterator.
pub trait PeekableExt<I>: Iterator
where
    I: Iterator,
{
    /// take_while_p behaves the same as the `take_while` method only the `take_while_p`
    /// form doesn't consume the first item where the predicate returns false.
    fn take_while_p<P>(&mut self, predicate: P) -> PeekingTakeWhile<'_, I, P>
    where
        P: FnMut(&Self::Item) -> bool;
}

impl<I: Iterator> PeekableExt<I> for std::iter::Peekable<I>
{
    #[inline]
    fn take_while_p<P>(&mut self, predicate: P) -> PeekingTakeWhile<'_, I, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        PeekingTakeWhile {
            iter: self,
            predicate,
        }
    }
}

/// The iterator returned by `take_while_p`
pub struct PeekingTakeWhile<'a, I, P>
where
    I: Iterator,
{
    pub(crate) iter: &'a mut std::iter::Peekable<I>,
    pub(crate) predicate: P,
}

impl<I, P> fmt::Debug for PeekingTakeWhile<'_, I, P>
where
    I: Iterator + fmt::Debug,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("PeekingTakeWhile").field("iter", &self.iter).finish()
    }
}

impl<I, P> Iterator for PeekingTakeWhile<'_, I, P>
where
    I: Iterator,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next_if(&mut self.predicate)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // can't know a lower bound, due to the predicate
        (0, self.iter.size_hint().1)
    }

    #[inline]
    fn fold<B, F>(mut self, mut accum: B, mut f: F) -> B
    where
        F: FnMut(B, I::Item) -> B,
    {
        while let Some(x) = self.iter.next_if(&mut self.predicate) {
            accum = f(accum, x);
        }
        accum
    }
}

#[cfg(test)]
mod tests
{
    use crate::prelude::*;

    #[test]
    fn test_take_while_p_should_keep_first_false() {

        // take_while_p keeps the first false
        let mut iter = vec![1, 2, 3, 4, 5].into_iter().peekable();
        assert_eq!(iter.take_while_p(|&x| x <= 3).collect::<Vec<i32>>(), vec![1, 2, 3]);
        assert_eq!(iter.collect::<Vec<i32>>(), vec![4, 5]);

        let mut iter = vec![1, 2, 3, 4, 5].into_iter().peekable();
        assert_eq!(iter.by_ref().take_while_p(|&x| x <= 3).collect::<Vec<i32>>(), vec![1, 2, 3]);
        assert_eq!(iter.collect::<Vec<i32>>(), vec![4, 5]);

        // take_while consumes the first false
        let mut iter = vec![1, 2, 3, 4, 5].into_iter().peekable();
        assert_eq!(iter.by_ref().take_while(|&x| x <= 3).collect::<Vec<i32>>(), vec![1, 2, 3]);
        assert_eq!(iter.collect::<Vec<i32>>(), vec![5]);
    }
}

