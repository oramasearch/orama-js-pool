use deno_core::extension;

use crate::permission::CustomPermissions;

extension!(
    orama_extension,
    deps = [deno_fetch],
    esm_entry_point = "ext:orama_extension/runtime.js",
    esm = ["runtime.js"],
    options = {
        permissions: CustomPermissions,
    },
    state = |state, options| {
        state.put::<CustomPermissions>(options.permissions);
    },
);
