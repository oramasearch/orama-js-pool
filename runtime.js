
import "ext:deno_url/01_urlpattern.js"
import "ext:deno_web/05_base64.js"
import "ext:deno_web/16_image_data.js"
import "ext:deno_web/08_text_encoding.js"
import "ext:deno_web/04_global_interfaces.js"
import "ext:deno_fetch/27_eventsource.js"
import "ext:deno_web/10_filereader.js"
import "ext:deno_web/13_message_port.js"
import "ext:deno_web/15_performance.js"
import "ext:deno_web/14_compression.js"

import "ext:orama_extension/runtime.js"

// The following lines are copied from deno_core repository
// See: https://github.com/denoland/deno/blob/5c8d894aa3858e8aa8130b7d2058033a0c7de539/runtime/js/98_global_scope_shared.js
// That code is licensed under MIT license
import { core } from "ext:core/mod.js";

import * as event from "ext:deno_web/02_event.js";
import * as timers from "ext:deno_web/02_timers.js";
import * as base64 from "ext:deno_web/05_base64.js";
import * as encoding from "ext:deno_web/08_text_encoding.js";
import * as console from "ext:deno_console/01_console.js";
import * as compression from "ext:deno_web/14_compression.js";
import * as performance from "ext:deno_web/15_performance.js";
import * as url from "ext:deno_url/00_url.js";
import * as urlPattern from "ext:deno_url/01_urlpattern.js";
import * as headers from "ext:deno_fetch/20_headers.js";
import * as streams from "ext:deno_web/06_streams.js";
import * as fileReader from "ext:deno_web/10_filereader.js";
import * as file from "ext:deno_web/09_file.js";
import * as formData from "ext:deno_fetch/21_formdata.js";
import * as request from "ext:deno_fetch/23_request.js";
import * as response from "ext:deno_fetch/23_response.js";
import * as fetch from "ext:deno_fetch/26_fetch.js";
import * as eventSource from "ext:deno_fetch/27_eventsource.js";
import * as messagePort from "ext:deno_web/13_message_port.js";
import * as webidl from "ext:deno_webidl/00_webidl.js";
import { DOMException } from "ext:deno_web/01_dom_exception.js";
import * as abortSignal from "ext:deno_web/03_abort_signal.js";
import * as imageData from "ext:deno_web/16_image_data.js";

console.setNoColorFns(
    () => true,
    () => true,
)

const windowOrWorkerGlobalScope = {
    AbortController: core.propNonEnumerable(abortSignal.AbortController),
    AbortSignal: core.propNonEnumerable(abortSignal.AbortSignal),
    Blob: core.propNonEnumerable(file.Blob),
    ByteLengthQueuingStrategy: core.propNonEnumerable(
        streams.ByteLengthQueuingStrategy,
    ),
    CloseEvent: core.propNonEnumerable(event.CloseEvent),
    CompressionStream: core.propNonEnumerable(compression.CompressionStream),
    CountQueuingStrategy: core.propNonEnumerable(
        streams.CountQueuingStrategy,
    ),
    CustomEvent: core.propNonEnumerable(event.CustomEvent),
    DecompressionStream: core.propNonEnumerable(compression.DecompressionStream),
    DOMException: core.propNonEnumerable(DOMException),
    ErrorEvent: core.propNonEnumerable(event.ErrorEvent),
    Event: core.propNonEnumerable(event.Event),
    EventTarget: core.propNonEnumerable(event.EventTarget),
    File: core.propNonEnumerable(file.File),
    FileReader: core.propNonEnumerable(fileReader.FileReader),
    FormData: core.propNonEnumerable(formData.FormData),
    Headers: core.propNonEnumerable(headers.Headers),
    ImageData: core.propNonEnumerable(imageData.ImageData),
    MessageEvent: core.propNonEnumerable(event.MessageEvent),
    Performance: core.propNonEnumerable(performance.Performance),
    PerformanceEntry: core.propNonEnumerable(performance.PerformanceEntry),
    PerformanceMark: core.propNonEnumerable(performance.PerformanceMark),
    PerformanceMeasure: core.propNonEnumerable(performance.PerformanceMeasure),
    PromiseRejectionEvent: core.propNonEnumerable(event.PromiseRejectionEvent),
    ProgressEvent: core.propNonEnumerable(event.ProgressEvent),
    ReadableStream: core.propNonEnumerable(streams.ReadableStream),
    ReadableStreamDefaultReader: core.propNonEnumerable(
        streams.ReadableStreamDefaultReader,
    ),
    Request: core.propNonEnumerable(request.Request),
    Response: core.propNonEnumerable(response.Response),
    TextDecoder: core.propNonEnumerable(encoding.TextDecoder),
    TextEncoder: core.propNonEnumerable(encoding.TextEncoder),
    TextDecoderStream: core.propNonEnumerable(encoding.TextDecoderStream),
    TextEncoderStream: core.propNonEnumerable(encoding.TextEncoderStream),
    TransformStream: core.propNonEnumerable(streams.TransformStream),
    URL: core.propNonEnumerable(url.URL),
    URLPattern: core.propNonEnumerable(urlPattern.URLPattern),
    URLSearchParams: core.propNonEnumerable(url.URLSearchParams),
    MessageChannel: core.propNonEnumerable(messagePort.MessageChannel),
    MessagePort: core.propNonEnumerable(messagePort.MessagePort),
    WritableStream: core.propNonEnumerable(streams.WritableStream),
    WritableStreamDefaultWriter: core.propNonEnumerable(
        streams.WritableStreamDefaultWriter,
    ),
    WritableStreamDefaultController: core.propNonEnumerable(
        streams.WritableStreamDefaultController,
    ),
    ReadableByteStreamController: core.propNonEnumerable(
        streams.ReadableByteStreamController,
    ),
    ReadableStreamBYOBReader: core.propNonEnumerable(
        streams.ReadableStreamBYOBReader,
    ),
    ReadableStreamBYOBRequest: core.propNonEnumerable(
        streams.ReadableStreamBYOBRequest,
    ),
    ReadableStreamDefaultController: core.propNonEnumerable(
        streams.ReadableStreamDefaultController,
    ),
    TransformStreamDefaultController: core.propNonEnumerable(
        streams.TransformStreamDefaultController,
    ),
    atob: core.propWritable(base64.atob),
    btoa: core.propWritable(base64.btoa),
    clearInterval: core.propWritable(timers.clearInterval),
    clearTimeout: core.propWritable(timers.clearTimeout),
    console: core.propNonEnumerable(
        // This is changed!!!!!
        new console.Console((msg, level) => {
            return Deno.core.ops.op_stream_to_oramacore_print(msg, level > 1)
        }),
    ),
    fetch: core.propWritable(fetch.fetch),
    EventSource: core.propWritable(eventSource.EventSource),
    performance: core.propWritable(performance.performance),
    reportError: core.propWritable(event.reportError),
    setInterval: core.propWritable(timers.setInterval),
    setTimeout: core.propWritable(timers.setTimeout),
    structuredClone: core.propWritable(messagePort.structuredClone),
    [webidl.brand]: core.propNonEnumerable(webidl.brand),
};

for (const [name, desc] of Object.entries(windowOrWorkerGlobalScope)) {
    Object.defineProperty(globalThis, name, desc);
}
