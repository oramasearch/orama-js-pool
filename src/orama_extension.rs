#![allow(clippy::result_large_err)]

use deno_core::{error::CoreError, extension, op2};

use crate::permission::CustomPermissions;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

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

// Shared cache structures
#[derive(Clone)]
struct CacheEntry {
    value: serde_json::Value,
    expires_at: Option<u64>,
}

#[derive(Clone)]
pub struct SharedCache {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl SharedCache {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for SharedCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct SharedKV {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl SharedKV {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[allow(dead_code)]
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self {
            data: Arc::new(RwLock::new(map)),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.read().unwrap().get(key).cloned()
    }

    #[allow(dead_code)]
    pub fn set(&self, key: String, value: String) {
        self.data.write().unwrap().insert(key, value);
    }

    #[allow(dead_code)]
    pub fn delete(&self, key: &str) {
        self.data.write().unwrap().remove(key);
    }
}

impl Default for SharedKV {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct SharedSecrets {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl SharedSecrets {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[allow(dead_code)]
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self {
            data: Arc::new(RwLock::new(map)),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.read().unwrap().get(key).cloned()
    }

    #[allow(dead_code)]
    pub fn set(&self, key: String, value: String) {
        self.data.write().unwrap().insert(key, value);
    }

    #[allow(dead_code)]
    pub fn delete(&self, key: &str) {
        self.data.write().unwrap().remove(key);
    }
}

impl Default for SharedSecrets {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[op2]
#[serde]
fn op_cache_get(
    #[state] cache: &SharedCache,
    #[string] key: String,
) -> Result<Option<serde_json::Value>, CoreError> {
    let mut cache_data = cache.data.write().unwrap();

    if let Some(entry) = cache_data.get(&key) {
        // Check if expired
        if let Some(expires_at) = entry.expires_at {
            if current_timestamp_ms() > expires_at {
                cache_data.remove(&key);
                return Ok(None);
            }
        }
        Ok(Some(entry.value.clone()))
    } else {
        Ok(None)
    }
}

#[op2]
fn op_cache_set(
    #[state] cache: &SharedCache,
    #[string] key: String,
    #[serde] value: serde_json::Value,
    #[serde] ttl_ms: Option<u64>,
) -> Result<(), CoreError> {
    let expires_at = ttl_ms.map(|ttl| current_timestamp_ms() + ttl);

    let entry = CacheEntry { value, expires_at };

    cache.data.write().unwrap().insert(key, entry);
    Ok(())
}

#[op2(fast)]
fn op_cache_delete(#[state] cache: &SharedCache, #[string] key: String) -> Result<(), CoreError> {
    cache.data.write().unwrap().remove(&key);
    Ok(())
}

#[op2]
#[string]
fn op_kv_get(#[state] kv: &SharedKV, #[string] key: String) -> Result<Option<String>, CoreError> {
    Ok(kv.get(&key))
}

#[op2]
#[string]
fn op_secret_get(
    #[state] secrets: &SharedSecrets,
    #[string] key: String,
) -> Result<Option<String>, CoreError> {
    Ok(secrets.get(&key))
}

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
    ops = [
        send_data_to_channel<V>,
        op_stream_to_oramacore_print,
        op_cache_get,
        op_cache_set,
        op_cache_delete,
        op_kv_get,
        op_secret_get,
    ],
    esm_entry_point = "ext:orama_extension/runtime.js",
    esm = ["runtime.js"],
    options = {
        permissions: CustomPermissions,
        channel_storage: ChannelStorage<V>,
        stdout_handler: StdoutHandler,
        shared_cache: SharedCache,
        shared_kv: SharedKV,
        shared_secrets: SharedSecrets,
    },
    state = |state, options| {
        state.put::<CustomPermissions>(options.permissions);
        state.put::<ChannelStorage<V>>(options.channel_storage);
        state.put::<StdoutHandler>(options.stdout_handler);
        state.put::<SharedCache>(options.shared_cache);
        state.put::<SharedKV>(options.shared_kv);
        state.put::<SharedSecrets>(options.shared_secrets);
    },
);
