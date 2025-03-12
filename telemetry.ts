

export const builtinTracer = function() { throw new Error('Tracing is not enabled'); };
export const enterSpan = function() { throw new Error('Tracing is not enabled'); };
export const restoreContext = function() { throw new Error('Tracing is not enabled'); }
export const TRACING_ENABLED = false;
