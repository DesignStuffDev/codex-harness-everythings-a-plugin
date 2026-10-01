# P03 service declaration API — first coherent slice

This freeze contains the declaration API and explicit inactive-catalog guard only. Broker limit DTOs, negotiation, outcomes, runtime ledgers and guarded session APIs remain separate staged work.


`component-api` exports:

```rust
pub enum ServicePresence { Required, Optional }
pub struct HostServiceRequirement { /* private checked fields */ }
impl HostServiceRequirement {
    pub fn new(name: String, version: u32, operations: Vec<String>, presence: ServicePresence)
        -> Result<Self, ServiceRequirementsError>;
    pub fn name(&self) -> &str;
    pub fn version(&self) -> u32;
    pub fn operations(&self) -> &[String];
    pub fn presence(&self) -> ServicePresence;
}
pub struct ServiceRequirementsV1 { /* private version/services */ }
impl ServiceRequirementsV1 {
    pub fn new(services: Vec<HostServiceRequirement>) -> Result<Self, ServiceRequirementsError>;
    pub fn version(&self) -> u32;
    pub fn services(&self) -> &[HostServiceRequirement];
}
```

The optional field is exactly `ComponentSpec.service_requirements: Option<ServiceRequirementsV1>`, with serde default and `skip_serializing_if = "Option::is_none"`. Wire shape is `{"version":1,"services":[{"name":"host.echo","version":1,"operations":["echo"],"presence":"required"}]}`. None is omitted; absent/null decode as None and canonical serialization omits it. A present object requires1–32 unique service/version descriptors; omit the field instead of an empty list. Service versions are positive integers, declaration version must equal1. Service names use1–128 UTF-8 bytes, operations1–256; operation lists contain1–32 unique values. Optionality is explicit, not inferred. Derived Eq compares stored vectors in order and is only DTO identity, not authority/acknowledgement equality. Later negotiation must compare validated canonical maps/sets keyed by service/version and operation names with exact caps and handles; do not use derived Vec Eq for order-insensitive acknowledgement.

Deserialize passes through private raw structures with `deny_unknown_fields` and checked constructors. It does not round-trip through Value. Input byte limits still belong to the enclosing loader/framer: current `read_manifest` bounds the file to1MiB before parsing; DTO validation is not a streaming JSON allocator limit. Installed package semver dependencies are unchanged. Valid declarations are temporarily rejected by actual `read_manifest` with `host service requirements are not supported by this host` until coordinated activation. Both required and optional declarations fail closed during this disabled slice.


Adding the public `ComponentSpec` field is a Rust source-literal API addition. Existing JSON packages remain compatible when the field is omitted. All six current workspace struct literals are adapted with `None`; external Rust literals must likewise supply it. No component/package version, process protocol or runtime grant is activated here.
