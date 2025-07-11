use deno_core::{error::CoreError, extension, op2};

use crate::permission::CustomPermissions;
use serde::de::DeserializeOwned;

pub trait StreamItem: DeserializeOwned + 'static {}
impl<T: DeserializeOwned + 'static> StreamItem for T {}

pub struct ChannelStorage<V: StreamItem> {
    pub handler: Option<Box<dyn FnMut(V)>>,
}

#[op2(stack_trace)]
#[serde]
fn send_data_to_channel<V: StreamItem>(
    #[state] storage: &mut ChannelStorage<V>,
    #[serde] data: V,
) -> Result<(), CoreError> {
    if let Some(handler) = storage.handler.as_mut() {
        (handler)(data);
    }

    Ok(())
}

extension!(
    orama_extension,
    deps = [deno_fetch],
    parameters = [V: StreamItem],
    ops = [ send_data_to_channel<V> ],
    esm_entry_point = "ext:orama_extension/runtime.js",
    esm = ["runtime.js"],
    options = {
        permissions: CustomPermissions,
        channel_storage: ChannelStorage<V>,
    },
    state = |state, options| {
        state.put::<CustomPermissions>(options.permissions);
        state.put::<ChannelStorage<V>>(options.channel_storage);
    },
);
