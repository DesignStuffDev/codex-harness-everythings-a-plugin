use std::alloc::Layout;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

use super::{Bucket, Entry, Vec as Arena, MAX_ENTRIES};
use crate::bounded::CapacityError;
use crate::Utf32String;

#[test]
fn invalid_fixed_capacities_are_rejected_before_allocation() {
    for capacity in [0, MAX_ENTRIES + 1, u32::MAX] {
        assert!(matches!(
            Arena::<u32>::fixed_allocation_bytes(capacity),
            Err(CapacityError::InvalidCapacity)
        ));
        assert!(matches!(
            Arena::<u32>::try_with_fixed_capacity(capacity),
            Err(CapacityError::InvalidCapacity)
        ));
    }
}

#[test]
fn fixed_charge_and_buckets_cover_exact_boundary_without_growth() {
    for (capacity, slots) in [
        (1, 32),
        (31, 32),
        (32, 32),
        (33, 96),
        (95, 96),
        (96, 96),
        (97, 224),
        (2048, 4064),
    ] {
        let expected_bytes = slots * Entry::<u32>::layout(1).size();
        assert_eq!(
            Arena::<u32>::fixed_allocation_bytes(capacity).unwrap(),
            expected_bytes
        );
        let arena = Arena::try_with_fixed_capacity(capacity).unwrap();
        let pointers_before: Vec<_> = arena
            .buckets
            .iter()
            .map(|bucket| bucket.entries.load(Ordering::Relaxed))
            .collect();
        for value in 0..capacity {
            assert_eq!(
                arena.try_push_single(value, Utf32String::default()),
                Ok(value)
            );
        }
        assert!(matches!(
            arena.try_push_single(capacity, Utf32String::default()),
            Err(CapacityError::EntryLimit)
        ));
        let pointers_after: Vec<_> = arena
            .buckets
            .iter()
            .map(|bucket| bucket.entries.load(Ordering::Relaxed))
            .collect();
        assert_eq!(pointers_after, pointers_before);
        assert_eq!(
            (arena.fixed_capacity(), arena.count()),
            (Some(capacity), capacity)
        );
        let actual: Vec<_> = unsafe { arena.snapshot(0) }
            .map(|(index, item)| (index, *item.unwrap().data))
            .collect();
        assert_eq!(
            actual,
            (0..capacity)
                .map(|value| (value, value))
                .collect::<Vec<_>>()
        );
        assert!(arena.get(capacity).is_none());
    }
}

#[test]
fn concurrent_rejection_does_not_advance_count_or_lose_accepted_entries() {
    let capacity = 97;
    let arena = Arena::try_with_fixed_capacity(capacity).unwrap();
    let start = Barrier::new(8);
    let mut accepted = thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|worker| {
                let arena = &arena;
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    let mut accepted = Vec::new();
                    for attempt in 0..64 {
                        let value = worker * 64 + attempt;
                        match arena.try_push_single(value, value.to_string().as_str().into()) {
                            Ok(index) => accepted.push((index, value)),
                            Err(CapacityError::EntryLimit) => {}
                            Err(error) => panic!("unexpected admission error: {error:?}"),
                        }
                    }
                    accepted
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    accepted.sort_unstable();
    assert_eq!(arena.count(), capacity);
    assert_eq!(
        accepted.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
        (0..capacity).collect::<Vec<_>>()
    );
    for (index, expected_value) in accepted {
        let item = arena.get(index).unwrap();
        assert_eq!(*item.data, expected_value);
        assert_eq!(
            item.matcher_columns,
            &[Utf32String::from(expected_value.to_string())]
        );
    }
    assert!(arena.get(capacity).is_none());
}

#[test]
fn prepared_column_is_moved_and_full_arena_returns_error_without_panic() {
    let arena = Arena::try_with_fixed_capacity(1).unwrap();
    let characters: Box<[char]> = vec!['a', 'é', '中'].into_boxed_slice();
    let original_pointer = characters.as_ptr();
    let result = catch_unwind(AssertUnwindSafe(|| {
        arena.try_push_single("payload".to_owned(), Utf32String::Unicode(characters))
    }));
    assert_eq!(result.unwrap(), Ok(0));
    let item = arena.get(0).unwrap();
    let Utf32String::Unicode(characters) = &item.matcher_columns[0] else {
        panic!("prepared Unicode column changed representation");
    };
    assert_eq!(
        (item.data.as_str(), characters.as_ptr()),
        ("payload", original_pointer)
    );
    let rejected = catch_unwind(AssertUnwindSafe(|| {
        arena.try_push_single("rejected".to_owned(), Utf32String::default())
    }));
    assert!(matches!(rejected.unwrap(), Err(CapacityError::EntryLimit)));
    assert_eq!(arena.count(), 1);
}

#[test]
fn accepted_and_rejected_owned_payloads_are_dropped_once() {
    struct Payload(Arc<AtomicUsize>);

    impl Drop for Payload {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let drops = Arc::new(AtomicUsize::new(0));
    let arena = Arena::try_with_fixed_capacity(1).unwrap();
    arena
        .try_push_single(Payload(drops.clone()), Utf32String::from("accepted"))
        .unwrap();
    assert!(matches!(
        arena.try_push_single(Payload(drops.clone()), Utf32String::from("rejected")),
        Err(CapacityError::EntryLimit)
    ));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    drop(arena);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn legacy_injection_is_fenced_before_mutating_a_fixed_arena() {
    let arena = Arena::try_with_fixed_capacity(2).unwrap();
    assert!(catch_unwind(|| arena.push(1, |_, _| panic!("must not run"))).is_err());
    assert!(catch_unwind(|| arena.extend(0..1, |_, _| panic!("must not run"))).is_err());
    assert_eq!((arena.count(), arena.get(0).is_some()), (0, false));
    assert_eq!(arena.try_push_single(2, Utf32String::default()), Ok(0));
}

#[test]
fn fixed_injection_rejects_a_legacy_arena_without_mutating_it() {
    let arena = Arena::<u32>::with_capacity(1, 1);
    assert!(matches!(
        arena.try_push_single(1, Utf32String::default()),
        Err(CapacityError::PlanMismatch)
    ));
    assert_eq!((arena.fixed_capacity(), arena.count()), (None, 0));
}

#[test]
fn fixed_layout_rejects_a_bucket_larger_than_isize_max() {
    let oversized_entry = Layout::from_size_align(isize::MAX as usize, 1).unwrap();
    assert!(matches!(
        Bucket::<u8>::try_layout(2, oversized_entry),
        Err(CapacityError::ArithmeticOverflow)
    ));
}
