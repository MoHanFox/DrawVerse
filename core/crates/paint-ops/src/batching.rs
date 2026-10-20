//! Bound temporary work without imposing a limit on the complete operation.

/// Plan a nonempty prefix, halving temporary work until it fits. `None` at one
/// item asks the caller to stream that item instead. Planning must not publish.
pub fn plan_batch<T, P, E>(
    items: &[T],
    preferred: usize,
    mut plan: impl FnMut(&[T]) -> Result<Option<P>, E>,
) -> Result<(usize, Option<P>), E> {
    if items.is_empty() {
        return Ok((0, None));
    }
    let mut count = items.len().min(preferred.max(1));
    loop {
        let result = plan(&items[..count])?;
        if result.is_some() || count == 1 {
            return Ok((count, result));
        }
        count = count.div_ceil(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitting_preserves_order_and_streams_an_oversized_item() {
        let work = [1, 2, 3, 99, 4, 5, 6];
        let mut seen = Vec::new();
        let mut streamed = Vec::new();
        let mut offset = 0;
        while offset < work.len() {
            let (count, result) = plan_batch(&work[offset..], 4, |batch| {
                Ok::<_, ()>((batch.iter().sum::<i32>() <= 8).then_some(()))
            })
            .unwrap();
            assert!(count > 0 && count <= 4);
            if result.is_none() {
                assert_eq!(count, 1);
                streamed.push(work[offset]);
            }
            seen.extend_from_slice(&work[offset..offset + count]);
            offset += count;
        }
        assert_eq!(seen, work);
        assert_eq!(streamed, [99]);
        assert_eq!(
            plan_batch::<i32, (), ()>(&[], 0, |_| panic!()).unwrap().0,
            0
        );
    }
}
