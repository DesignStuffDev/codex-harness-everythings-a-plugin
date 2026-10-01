#![allow(clippy::expect_used)]

use super::*;
use pretty_assertions::assert_eq;
use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeSeq;
use serde_json::json;

#[test]
fn request_limits_match_exact_serialized_bytes() -> Result<()> {
    let values = [
        Value::Null,
        json!("é🦀"),
        json!("\"\\\n\t\u{0000}"),
        json!({"é\"": [false, -12345, 0.03125, {"nested": "\\"}]}),
        json!([]),
        json!({}),
    ];
    for value in values {
        let bytes = serde_json::to_vec(&value)?.len() as u64;
        let limits = SessionPayloadLimits {
            request_bytes: NonZeroU64::new(bytes),
            ..SessionPayloadLimits::default()
        };
        limits.check_request(&value)?;
        let limits = SessionPayloadLimits {
            request_bytes: NonZeroU64::new(bytes - 1),
            ..limits
        };
        let error = limits.check_request(&value).expect_err("one byte over cap");
        assert_eq!(
            error.downcast_ref::<PayloadLimitExceeded>(),
            Some(&PayloadLimitExceeded {
                kind: PayloadKind::Request,
                limit_bytes: bytes - 1,
            })
        );
    }
    Ok(())
}

#[test]
fn escaped_strings_use_the_counting_serializer_after_the_lower_bound() {
    let value = json!({"history": ["\n\n\n\n\n\n\n\n"]});
    let limit = 30;
    assert_eq!(lower_bound_fits(&value, limit), Some(()));
    assert_eq!(
        check_value(&value, PayloadKind::Request, limit)
            .expect_err("escaped bytes exceed the budget")
            .downcast_ref::<PayloadLimitExceeded>(),
        Some(&PayloadLimitExceeded {
            kind: PayloadKind::Request,
            limit_bytes: limit,
        })
    );
}

#[test]
fn lower_bound_rejects_large_nested_strings_and_wide_containers() {
    let string = json!({"history": [{"message": "x".repeat(1024 * 1024)}]});
    let array = Value::Array(vec![Value::Null; 1024]);
    assert_eq!(lower_bound_fits(&string, 64), None);
    assert_eq!(lower_bound_fits(&array, 64), None);
}

#[test]
fn serializer_stops_at_the_first_over_budget_write() {
    struct UnreachableTail;

    impl Serialize for UnreachableTail {
        fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
            let mut sequence = serializer.serialize_seq(Some(2))?;
            sequence.serialize_element("\n\n\n\n\n\n\n\n")?;
            panic!("serializer must stop before visiting the next element");
        }
    }

    let mut counter = LimitedCounter {
        remaining: 8,
        exceeded: false,
    };
    assert!(serde_json::to_writer(&mut counter, &UnreachableTail).is_err());
    assert!(counter.exceeded);
}

#[test]
fn incoming_declared_lengths_are_checked_without_payloads() -> Result<()> {
    let limits = SessionPayloadLimits {
        request_bytes: NonZeroU64::new(7),
        reply_bytes: NonZeroU64::new(64 * 1024),
    };
    limits.validate()?;
    for (kind, limit_bytes) in [(PayloadKind::Request, 7), (PayloadKind::Reply, 64 * 1024)] {
        limits.check_incoming(kind, limit_bytes)?;
        assert_eq!(
            limits
                .check_incoming(kind, limit_bytes + 1)
                .expect_err("declared payload exceeds limit")
                .downcast_ref::<PayloadLimitExceeded>(),
            Some(&PayloadLimitExceeded { kind, limit_bytes })
        );
    }
    Ok(())
}

#[test]
fn reply_configuration_cannot_reject_legal_control_replies() -> Result<()> {
    let limits = SessionPayloadLimits {
        request_bytes: NonZeroU64::new(1),
        reply_bytes: NonZeroU64::new(64 * 1024 - 1),
    };
    assert!(limits.validate().is_err());
    let limits = SessionPayloadLimits {
        reply_bytes: NonZeroU64::new(64 * 1024),
        ..limits
    };
    limits.validate()?;
    let boundary = Value::String("x".repeat(64 * 1024 - 2));
    limits.check_reply(&boundary, /*is_control*/ true)?;
    limits.check_reply(&boundary, /*is_control*/ false)?;
    SessionPayloadLimits::check_control(&boundary)?;
    let oversized = Value::String("x".repeat(64 * 1024 - 1));
    assert!(limits.check_reply(&oversized, /*is_control*/ true).is_err());
    assert!(
        limits
            .check_reply(&oversized, /*is_control*/ false)
            .is_err()
    );
    assert!(SessionPayloadLimits::check_control(&oversized).is_err());
    Ok(())
}

#[test]
fn ordinary_caps_do_not_replace_the_control_budget() -> Result<()> {
    let limits = SessionPayloadLimits {
        request_bytes: NonZeroU64::new(1),
        reply_bytes: NonZeroU64::new(128 * 1024),
    };
    limits.validate()?;
    SessionPayloadLimits::check_control(&Value::Null)?;
    limits.check_reply(&Value::Null, /*is_control*/ true)?;
    let value = Value::String("x".repeat(64 * 1024));
    limits.check_reply(&value, /*is_control*/ false)?;
    assert_eq!(
        limits
            .check_reply(&value, /*is_control*/ true)
            .expect_err("control reply keeps its fixed cap")
            .downcast_ref::<PayloadLimitExceeded>(),
        Some(&PayloadLimitExceeded {
            kind: PayloadKind::Reply,
            limit_bytes: 64 * 1024,
        })
    );
    Ok(())
}

#[test]
fn default_caps_preserve_large_history_payloads() -> Result<()> {
    let limits = SessionPayloadLimits::default();
    let value = json!({"history": "x".repeat(17 * 1024 * 1024)});
    limits.validate()?;
    limits.check_request(&value)?;
    limits.check_reply(&value, /*is_control*/ false)?;
    limits.check_incoming(PayloadKind::Request, u64::MAX)?;
    limits.check_incoming(PayloadKind::Reply, u64::MAX)?;
    Ok(())
}
