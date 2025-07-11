#![allow(clippy::result_large_err)]

use deno_core::{error::CoreError, extension, op2};

use crate::permission::CustomPermissions;
use serde::de::DeserializeOwned;

pub trait StreamItem: DeserializeOwned + 'static {}
impl<T: DeserializeOwned + 'static> StreamItem for T {}

pub struct ChannelStorage<V: StreamItem> {
    pub stream_handler: Option<Box<dyn FnMut(V)>>,
}

#[op2(stack_trace)]
#[serde]
fn send_data_to_channel<V: StreamItem>(
    #[state] storage: &mut ChannelStorage<V>,
    #[serde] data: V,
) -> Result<(), CoreError> {
    if let Some(handler) = storage.stream_handler.as_mut() {
        (handler)(data);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputChannel {
    StdOut,
    StdErr,
}

// Type alias for the boxed handler function
pub type StdoutHandlerFn = Box<dyn FnMut(&str, OutputChannel)>;

pub struct StdoutHandler(pub Option<StdoutHandlerFn>);

#[op2(fast)]
pub fn op_stream_to_oramacore_print(
    #[state] storage: &mut StdoutHandler,
    #[string] data: &str,
    is_stderr: bool,
) -> Result<(), CoreError> {
    if let Some(handler) = storage.0.as_mut() {
        let channel = if is_stderr {
            OutputChannel::StdErr
        } else {
            OutputChannel::StdOut
        };
        (handler)(data, channel);
    }
    Ok(())
}

extension!(
    orama_extension,
    deps = [deno_fetch, deno_console],
    parameters = [V: StreamItem],
    ops = [ send_data_to_channel<V>, op_stream_to_oramacore_print ],
    esm_entry_point = "ext:orama_extension/runtime.js",
    esm = ["runtime.js"],
    options = {
        permissions: CustomPermissions,
        channel_storage: ChannelStorage<V>,
        stdout_handler: StdoutHandler,
    },
    state = |state, options| {
        state.put::<CustomPermissions>(options.permissions);
        state.put::<ChannelStorage<V>>(options.channel_storage);
        state.put::<StdoutHandler>(options.stdout_handler);
    },
);
