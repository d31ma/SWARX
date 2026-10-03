use crate::models::{NativeCommandBindingEvent, NativeCommandIdentity, NativeCommandOwner};

pub struct CommandBindingFixture;

impl CommandBindingFixture {
    const VERSION: i64 = 1;

    pub fn bind(
        owner: NativeCommandOwner,
        identity: NativeCommandIdentity,
    ) -> NativeCommandBindingEvent {
        NativeCommandBindingEvent {
            version: Self::VERSION,
            kind: "logical.command.bind".to_owned(),
            owner,
            identity: Some(identity),
            binding_id: None,
        }
    }

    pub fn renew(
        owner: NativeCommandOwner,
        identity: NativeCommandIdentity,
        binding_id: String,
    ) -> NativeCommandBindingEvent {
        NativeCommandBindingEvent {
            version: Self::VERSION,
            kind: "logical.command.renew".to_owned(),
            owner,
            identity: Some(identity),
            binding_id: Some(binding_id),
        }
    }

    pub fn unbind(owner: NativeCommandOwner, binding_id: String) -> NativeCommandBindingEvent {
        NativeCommandBindingEvent {
            version: Self::VERSION,
            kind: "logical.command.unbind".to_owned(),
            owner,
            identity: None,
            binding_id: Some(binding_id),
        }
    }
}
