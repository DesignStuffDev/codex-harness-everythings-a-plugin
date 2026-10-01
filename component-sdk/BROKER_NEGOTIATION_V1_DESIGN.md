# P03 offer/acknowledgement contract — accepted design, implementation pending

This is the next DTO/validation slice proposal, separate from frozen broker-limit code. It adds no active broker or service call API. Keep the inactive catalog guard until coordinated negotiation and retained lifecycle enforcement pass their later gates. Declaration format version, broker extension version, per-service version and package semver remain distinct.

## Proposed types and signatures

Use private modules with explicit component-api exports. Every field below is private in the checked public type; signatures expose immutable views. All checked types support Clone and serde; opaque handles have redacted Debug and no Display. No constructor mints authority: live host state must later verify the complete connection/service/operation/handle binding.

```rust
pub const BROKER_EXTENSION_VERSION: u32 = 1;

pub struct BrokerConnectionHandle { /* checked private String */ }
impl BrokerConnectionHandle {
    pub fn new(value: String) -> Result<Self, BrokerNegotiationError>;
}
pub struct ServiceAuthorityHandle { /* checked private String */ }
impl ServiceAuthorityHandle {
    pub fn new(value: String) -> Result<Self, BrokerNegotiationError>;
}

pub struct GrantedOperationV1 { /* name, max_request_bytes, max_response_bytes */ }
impl GrantedOperationV1 {
    pub fn new(name: String, max_request_bytes: u64, max_response_bytes: u64)
        -> Result<Self, BrokerNegotiationError>;
    pub fn name(&self) -> &str;
    pub fn max_request_bytes(&self) -> u64;
    pub fn max_response_bytes(&self) -> u64;
}

pub struct ServiceGrantV1 { /* name, version, operations, authority */ }
impl ServiceGrantV1 {
    pub fn new(name: String, version: u32, operations: Vec<GrantedOperationV1>,
        authority: ServiceAuthorityHandle) -> Result<Self, BrokerNegotiationError>;
    pub fn name(&self) -> &str;
    pub fn version(&self) -> u32;
    pub fn operations(&self) -> &[GrantedOperationV1];
    pub fn authority(&self) -> &ServiceAuthorityHandle;
}

pub struct BrokerOfferV1 { /* version, connection, limits, services */ }
impl BrokerOfferV1 {
    pub fn new(connection: BrokerConnectionHandle, limits: BrokerConnectionLimits,
        services: Vec<ServiceGrantV1>) -> Result<Self, BrokerNegotiationError>;
    pub fn version(&self) -> u32;
    pub fn connection(&self) -> &BrokerConnectionHandle;
    pub fn limits(&self) -> &BrokerConnectionLimits;
    pub fn services(&self) -> &[ServiceGrantV1];
    pub fn validate_requirements(&self, requirements: &ServiceRequirementsV1)
        -> Result<(), BrokerNegotiationError>;
    pub fn validate_acknowledgement(&self, acknowledgement: &BrokerAcknowledgementV1)
        -> Result<(), BrokerNegotiationError>;
}

pub struct BrokerAcknowledgementV1 { /* same checked wire shape as offer */ }
impl BrokerAcknowledgementV1 {
    pub fn from_offer(offer: &BrokerOfferV1) -> Self;
}
```

`BrokerNegotiationError` is a typed no-payload enum: `UnsupportedVersion`, `InvalidConnectionHandle`, `InvalidAuthorityHandle`, `TooManyServices`, `InvalidServiceName`, `InvalidServiceVersion`, `InvalidOperationCount`, `InvalidOperationName`, `DuplicateService`, `DuplicateOperation`, `InvalidOperationLimit`, `OperationLimitExceedsConnection`, `UndeclaredService`, `MissingRequiredService`, `OperationSetMismatch`, `AcknowledgementMismatch`. Its Display is fixed text; no supplied names/handles. Serde parser errors are not a redacted public diagnostic contract: future transport must map them to a typed fixed malformed-message failure rather than forwarding raw parser text.

No raw-handle string getter is required: handles can be cloned, compared and serialized for exact echo; live authority remains in the host registry. No public mutable fields, Deref or unchecked conversion permits bypassing validation. Handle types must remain distinct so connection IDs cannot be accidentally substituted for service authority handles.

## Exact proposed wire shape

Offer and acknowledgement have identical field names and mandatory members:

```json
{
  "version": 1,
  "connection": "opaque-connection",
  "limits": { "...": "all explicit BrokerConnectionLimits fields" },
  "services": [{
    "name": "host.diagnostic",
    "version": 1,
    "operations": [{ "name": "read", "max_request_bytes": 1024, "max_response_bytes": 1024 }],
    "authority": "opaque-service-grant"
  }]
}
```

This JSON illustrates shape, not a valid concrete limits object or minted grant. BrokerOfferV1::new fixes broker version1; raw decode rejects any other version. The wrapper remains optional only at the later initialization envelope; plain peers omit the whole extension, rather than manufacturing an empty offer. A negotiated offer may contain zero services when all declared services are optional and unavailable. It never exceeds32 descriptors; each present descriptor has1–32 operations. A service with incomplete optional operations is omitted whole.

Each handle is1–256 UTF-8 bytes. Service names use1–128 bytes and operation names1–256 bytes; case and Unicode bytes are exact with no wildcard, trimming or Unicode normalization. Service versions are positive. Both per-operation byte limits are positive and no greater than the checked connection maximum for that direction. The offer retains the complete checked connection limits unchanged. This proposal does not add a process budget to the wire; process-wide accounting is host/runtime policy.

## Canonical comparison: order independent, duplicates rejected

1. Check operation count, lengths, positive caps and uniqueness by operation name before canonicalization. Different caps on a repeated name are still a duplicate error.
2. Store each service's operation vector sorted by exact operation-name ordering.
3. Check service count and uniqueness by the exact `(name, version)` tuple. Different handles/operations on a duplicate tuple are still an error.
4. Store each offer/ack service vector sorted by `(name, version)`; validate every per-operation cap against the enclosed connection limits.
5. Deserialize through private `deny_unknown_fields` raw structs and the same checked constructors. Do not go through serde_json::Value or a map that erases duplicate keys. The future transport must parse the original bounded handshake bytes, including envelope-level duplicate extensions/dependency_broker members; the current Value-based handshake does not satisfy this requirement.
6. `from_offer` clones the already checked canonical body. Raw acknowledgement deserialization canonicalizes by the same rules.
7. `validate_acknowledgement` compares broker version, connection handle, every connection-limit field, the entire canonical service set, each service authority handle and the complete canonical operation set with both directional caps. Any missing, extra or altered value fails. This explicitly does not use equality on the original unnormalized declaration Vec.

An acknowledgement may reorder entries but may neither narrow nor widen the offered grants. Canonical private checked storage makes complete derived equality safe internally; host callers use the named validation method. Serializable data and successful equality do not prove live authority, revocation state, matching owner generation or cleanup.

`validate_requirements` is separate from parsing/equality. Every offered `(name, version)` must have a matching declaration with exactly the same operation-name set, and every required descriptor must be present. Optional descriptors may be absent, but a present one must be complete. Unexpected services/versions or operation additions fail. This prevents accidentally offering authority the package did not request. Only host composition can choose compatible caps and issue handles; this validator does not select implementations or discover services.

## Review-sized implementation and acceptance order

1. Add handles/operation/service-grant checked DTOs and strict malformed/duplicate tests. Keep declarations and inactive guard unchanged.
2. Add checked offer/ack bodies, canonical constructors, requirement matching and complete acknowledgement validation. Reuse the frozen checked connection limits. No runtime linkage.
3. API-only gates exercise raw JSON duplicates/unknowns/versions,32-service and32-operation boundaries, per-operation caps, reordered equivalent offers/acks, empty optional-only grant sets, and changed/missing/extra descriptors/operations/handles/limits. Include required missing and optional partial descriptor failures. Redacted Debug assertions must cover both handle types; canonical errors must not include input handle values.
4. Only after these APIs are reviewed and tested, separately implement the accepted connection reservation/negotiation/retained lifecycle and leaf-call guards. With any required requirement, missing acknowledgement fails startup. Optional-only missing acknowledgement selects plain mode with the broker disabled, no scopes and no broker frames. Malformed or present-invalid acknowledgement always fails. A present valid acknowledgement must match the complete offer exactly. Retain child cleanup on every startup failure. This note supplies no implementation of those obligations.

Raw Vec/String input materializes before these constructors run; enclosing frame limits and later ledgers must bound resource use. Canonicalization is validation, not a pre-serde allocation or RSS guarantee. No installed component, worker execution, native catalog extraction or GUI claim arises from this proposal.

Independent design review identified and resolved the optional-only missing-ack wording above. This document is a design contract, not evidence of implemented negotiation, live authority or installed service execution.
