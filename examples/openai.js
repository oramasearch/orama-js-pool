const gt = "RFC3986", _t = {
  RFC1738: (s) => String(s).replace(/%20/g, "+"),
  RFC3986: (s) => String(s)
}, Os = "RFC1738", $s = Array.isArray, Q = (() => {
  const s = [];
  for (let e = 0; e < 256; ++e)
    s.push("%" + ((e < 16 ? "0" : "") + e.toString(16)).toUpperCase());
  return s;
})(), it = 1024, Fs = (s, e, t, n, r) => {
  if (s.length === 0)
    return s;
  let a = s;
  if (typeof s == "symbol" ? a = Symbol.prototype.toString.call(s) : typeof s != "string" && (a = String(s)), t === "iso-8859-1")
    return escape(a).replace(/%u[0-9a-f]{4}/gi, function(o) {
      return "%26%23" + parseInt(o.slice(2), 16) + "%3B";
    });
  let i = "";
  for (let o = 0; o < a.length; o += it) {
    const c = a.length >= it ? a.slice(o, o + it) : a, u = [];
    for (let g = 0; g < c.length; ++g) {
      let l = c.charCodeAt(g);
      if (l === 45 || l === 46 || l === 95 || l === 126 || l >= 48 && l <= 57 || l >= 65 && l <= 90 || l >= 97 && l <= 122 || r === Os && (l === 40 || l === 41)) {
        u[u.length] = c.charAt(g);
        continue;
      }
      if (l < 128) {
        u[u.length] = Q[l];
        continue;
      }
      if (l < 2048) {
        u[u.length] = Q[192 | l >> 6] + Q[128 | l & 63];
        continue;
      }
      if (l < 55296 || l >= 57344) {
        u[u.length] = Q[224 | l >> 12] + Q[128 | l >> 6 & 63] + Q[128 | l & 63];
        continue;
      }
      g += 1, l = 65536 + ((l & 1023) << 10 | c.charCodeAt(g) & 1023), u[u.length] = Q[240 | l >> 18] + Q[128 | l >> 12 & 63] + Q[128 | l >> 6 & 63] + Q[128 | l & 63];
    }
    i += u.join("");
  }
  return i;
};
function Ts(s) {
  return !s || typeof s != "object" ? !1 : !!(s.constructor && s.constructor.isBuffer && s.constructor.isBuffer(s));
}
function en(s, e) {
  if ($s(s)) {
    const t = [];
    for (let n = 0; n < s.length; n += 1)
      t.push(e(s[n]));
    return t;
  }
  return e(s);
}
const ks = Object.prototype.hasOwnProperty, bn = {
  brackets(s) {
    return String(s) + "[]";
  },
  comma: "comma",
  indices(s, e) {
    return String(s) + "[" + e + "]";
  },
  repeat(s) {
    return String(s);
  }
}, z = Array.isArray, Ms = Array.prototype.push, xn = function(s, e) {
  Ms.apply(s, z(e) ? e : [e]);
}, Ns = Date.prototype.toISOString, I = {
  addQueryPrefix: !1,
  allowDots: !1,
  allowEmptyArrays: !1,
  arrayFormat: "indices",
  charset: "utf-8",
  charsetSentinel: !1,
  delimiter: "&",
  encode: !0,
  encodeDotInKeys: !1,
  encoder: Fs,
  encodeValuesOnly: !1,
  format: gt,
  formatter: _t[gt],
  indices: !1,
  serializeDate(s) {
    return Ns.call(s);
  },
  skipNulls: !1,
  strictNullHandling: !1
};
function Ds(s) {
  return typeof s == "string" || typeof s == "number" || typeof s == "boolean" || typeof s == "symbol" || typeof s == "bigint";
}
const ot = {};
function Sn(s, e, t, n, r, a, i, o, c, u, g, l, h, f, b, m, A, E) {
  let p = s, F = E, C = 0, T = !1;
  for (; (F = F.get(ot)) !== void 0 && !T; ) {
    const P = F.get(s);
    if (C += 1, typeof P < "u") {
      if (P === C)
        throw new RangeError("Cyclic object value");
      T = !0;
    }
    typeof F.get(ot) > "u" && (C = 0);
  }
  if (typeof u == "function" ? p = u(e, p) : p instanceof Date ? p = h?.(p) : t === "comma" && z(p) && (p = en(p, function(P) {
    return P instanceof Date ? h?.(P) : P;
  })), p === null) {
    if (a)
      return c && !m ? c(e, I.encoder, A, "key", f) : e;
    p = "";
  }
  if (Ds(p) || Ts(p)) {
    if (c) {
      const P = m ? e : c(e, I.encoder, A, "key", f);
      return [
        b?.(P) + "=" + b?.(c(p, I.encoder, A, "value", f))
      ];
    }
    return [b?.(e) + "=" + b?.(String(p))];
  }
  const D = [];
  if (typeof p > "u")
    return D;
  let B;
  if (t === "comma" && z(p))
    m && c && (p = en(p, c)), B = [{ value: p.length > 0 ? p.join(",") || null : void 0 }];
  else if (z(u))
    B = u;
  else {
    const P = Object.keys(p);
    B = g ? P.sort(g) : P;
  }
  const k = o ? String(e).replace(/\./g, "%2E") : String(e), O = n && z(p) && p.length === 1 ? k + "[]" : k;
  if (r && z(p) && p.length === 0)
    return O + "[]";
  for (let P = 0; P < B.length; ++P) {
    const R = B[P], Yt = typeof R == "object" && typeof R.value < "u" ? R.value : p[R];
    if (i && Yt === null)
      continue;
    const at = l && o ? R.replace(/\./g, "%2E") : R, Is = z(p) ? typeof t == "function" ? t(O, at) : O : O + (l ? "." + at : "[" + at + "]");
    E.set(s, C);
    const Zt = /* @__PURE__ */ new WeakMap();
    Zt.set(ot, E), xn(D, Sn(
      Yt,
      Is,
      t,
      n,
      r,
      a,
      i,
      o,
      t === "comma" && m && z(p) ? null : c,
      u,
      g,
      l,
      h,
      f,
      b,
      m,
      A,
      Zt
    ));
  }
  return D;
}
function Bs(s = I) {
  if (typeof s.allowEmptyArrays < "u" && typeof s.allowEmptyArrays != "boolean")
    throw new TypeError("`allowEmptyArrays` option can only be `true` or `false`, when provided");
  if (typeof s.encodeDotInKeys < "u" && typeof s.encodeDotInKeys != "boolean")
    throw new TypeError("`encodeDotInKeys` option can only be `true` or `false`, when provided");
  if (s.encoder !== null && typeof s.encoder < "u" && typeof s.encoder != "function")
    throw new TypeError("Encoder has to be a function.");
  const e = s.charset || I.charset;
  if (typeof s.charset < "u" && s.charset !== "utf-8" && s.charset !== "iso-8859-1")
    throw new TypeError("The charset option must be either utf-8, iso-8859-1, or undefined");
  let t = gt;
  if (typeof s.format < "u") {
    if (!ks.call(_t, s.format))
      throw new TypeError("Unknown format option provided.");
    t = s.format;
  }
  const n = _t[t];
  let r = I.filter;
  (typeof s.filter == "function" || z(s.filter)) && (r = s.filter);
  let a;
  if (s.arrayFormat && s.arrayFormat in bn ? a = s.arrayFormat : "indices" in s ? a = s.indices ? "indices" : "repeat" : a = I.arrayFormat, "commaRoundTrip" in s && typeof s.commaRoundTrip != "boolean")
    throw new TypeError("`commaRoundTrip` must be a boolean, or absent");
  const i = typeof s.allowDots > "u" ? s.encodeDotInKeys ? !0 : I.allowDots : !!s.allowDots;
  return {
    addQueryPrefix: typeof s.addQueryPrefix == "boolean" ? s.addQueryPrefix : I.addQueryPrefix,
    allowDots: i,
    allowEmptyArrays: typeof s.allowEmptyArrays == "boolean" ? !!s.allowEmptyArrays : I.allowEmptyArrays,
    arrayFormat: a,
    charset: e,
    charsetSentinel: typeof s.charsetSentinel == "boolean" ? s.charsetSentinel : I.charsetSentinel,
    commaRoundTrip: !!s.commaRoundTrip,
    delimiter: typeof s.delimiter > "u" ? I.delimiter : s.delimiter,
    encode: typeof s.encode == "boolean" ? s.encode : I.encode,
    encodeDotInKeys: typeof s.encodeDotInKeys == "boolean" ? s.encodeDotInKeys : I.encodeDotInKeys,
    encoder: typeof s.encoder == "function" ? s.encoder : I.encoder,
    encodeValuesOnly: typeof s.encodeValuesOnly == "boolean" ? s.encodeValuesOnly : I.encodeValuesOnly,
    filter: r,
    format: t,
    formatter: n,
    serializeDate: typeof s.serializeDate == "function" ? s.serializeDate : I.serializeDate,
    skipNulls: typeof s.skipNulls == "boolean" ? s.skipNulls : I.skipNulls,
    sort: typeof s.sort == "function" ? s.sort : null,
    strictNullHandling: typeof s.strictNullHandling == "boolean" ? s.strictNullHandling : I.strictNullHandling
  };
}
function Ls(s, e = {}) {
  let t = s;
  const n = Bs(e);
  let r, a;
  typeof n.filter == "function" ? (a = n.filter, t = a("", t)) : z(n.filter) && (a = n.filter, r = a);
  const i = [];
  if (typeof t != "object" || t === null)
    return "";
  const o = bn[n.arrayFormat], c = o === "comma" && n.commaRoundTrip;
  r || (r = Object.keys(t)), n.sort && r.sort(n.sort);
  const u = /* @__PURE__ */ new WeakMap();
  for (let h = 0; h < r.length; ++h) {
    const f = r[h];
    n.skipNulls && t[f] === null || xn(i, Sn(
      t[f],
      f,
      o,
      c,
      n.allowEmptyArrays,
      n.strictNullHandling,
      n.skipNulls,
      n.encodeDotInKeys,
      n.encode ? n.encoder : null,
      n.filter,
      n.sort,
      n.allowDots,
      n.serializeDate,
      n.format,
      n.formatter,
      n.encodeValuesOnly,
      n.charset,
      u
    ));
  }
  const g = i.join(n.delimiter);
  let l = n.addQueryPrefix === !0 ? "?" : "";
  return n.charsetSentinel && (n.charset === "iso-8859-1" ? l += "utf8=%26%2310003%3B&" : l += "utf8=%E2%9C%93&"), g.length > 0 ? l + g : "";
}
const ce = "4.86.2";
let tn = !1, Pe, Cn, An, wt, Pn, En, Rn, vn, In;
function Us(s, e = { auto: !1 }) {
  if (tn)
    throw new Error(`you must \`import 'openai/shims/${s.kind}'\` before importing anything else from openai`);
  if (Pe)
    throw new Error(`can't \`import 'openai/shims/${s.kind}'\` after \`import 'openai/shims/${Pe}'\``);
  tn = e.auto, Pe = s.kind, Cn = s.fetch, An = s.FormData, wt = s.File, Pn = s.ReadableStream, En = s.getMultipartRequestOptions, Rn = s.getDefaultAgent, vn = s.fileFromPath, In = s.isFsReadStream;
}
class js {
  constructor(e) {
    this.body = e;
  }
  get [Symbol.toStringTag]() {
    return "MultipartBody";
  }
}
function Ws({ manuallyImported: s } = {}) {
  const e = s ? "You may need to use polyfills" : "Add one of these imports before your first `import … from 'openai'`:\n- `import 'openai/shims/node'` (if you're running on Node)\n- `import 'openai/shims/web'` (otherwise)\n";
  let t, n, r, a;
  try {
    t = fetch, n = Request, r = Response, a = Headers;
  } catch (i) {
    throw new Error(`this environment is missing the following Web Fetch API type: ${i.message}. ${e}`);
  }
  return {
    kind: "web",
    fetch: t,
    Request: n,
    Response: r,
    Headers: a,
    FormData: typeof FormData < "u" ? FormData : class {
      constructor() {
        throw new Error(`file uploads aren't supported in this environment yet as 'FormData' is undefined. ${e}`);
      }
    },
    Blob: typeof Blob < "u" ? Blob : class {
      constructor() {
        throw new Error(`file uploads aren't supported in this environment yet as 'Blob' is undefined. ${e}`);
      }
    },
    File: typeof File < "u" ? File : class {
      constructor() {
        throw new Error(`file uploads aren't supported in this environment yet as 'File' is undefined. ${e}`);
      }
    },
    ReadableStream: typeof ReadableStream < "u" ? ReadableStream : class {
      constructor() {
        throw new Error(`streaming isn't supported in this environment yet as 'ReadableStream' is undefined. ${e}`);
      }
    },
    getMultipartRequestOptions: async (i, o) => ({
      ...o,
      body: new js(i)
    }),
    getDefaultAgent: (i) => {
    },
    fileFromPath: () => {
      throw new Error("The `fileFromPath` function is only supported in Node. See the README for more details: https://www.github.com/openai/openai-node#file-uploads");
    },
    isFsReadStream: (i) => !1
  };
}
Pe || Us(Ws(), { auto: !0 });
class _ extends Error {
}
class N extends _ {
  constructor(e, t, n, r) {
    super(`${N.makeMessage(e, t, n)}`), this.status = e, this.headers = r, this.request_id = r?.["x-request-id"], this.error = t;
    const a = t;
    this.code = a?.code, this.param = a?.param, this.type = a?.type;
  }
  static makeMessage(e, t, n) {
    const r = t?.message ? typeof t.message == "string" ? t.message : JSON.stringify(t.message) : t ? JSON.stringify(t) : n;
    return e && r ? `${e} ${r}` : e ? `${e} status code (no body)` : r || "(no status code or body)";
  }
  static generate(e, t, n, r) {
    if (!e || !r)
      return new ze({ message: n, cause: bt(t) });
    const a = t?.error;
    return e === 400 ? new On(e, a, n, r) : e === 401 ? new $n(e, a, n, r) : e === 403 ? new Fn(e, a, n, r) : e === 404 ? new Tn(e, a, n, r) : e === 409 ? new kn(e, a, n, r) : e === 422 ? new Mn(e, a, n, r) : e === 429 ? new Nn(e, a, n, r) : e >= 500 ? new Dn(e, a, n, r) : new N(e, a, n, r);
  }
}
class K extends N {
  constructor({ message: e } = {}) {
    super(void 0, void 0, e || "Request was aborted.", void 0);
  }
}
class ze extends N {
  constructor({ message: e, cause: t }) {
    super(void 0, void 0, e || "Connection error.", void 0), t && (this.cause = t);
  }
}
class It extends ze {
  constructor({ message: e } = {}) {
    super({ message: e ?? "Request timed out." });
  }
}
class On extends N {
}
class $n extends N {
}
class Fn extends N {
}
class Tn extends N {
}
class kn extends N {
}
class Mn extends N {
}
class Nn extends N {
}
class Dn extends N {
}
class Bn extends _ {
  constructor() {
    super("Could not parse response content as the length limit was reached");
  }
}
class Ln extends _ {
  constructor() {
    super("Could not parse response content as the request was rejected by the content filter");
  }
}
var ke = function(s, e, t, n, r) {
  if (n === "m") throw new TypeError("Private method is not writable");
  if (n === "a" && !r) throw new TypeError("Private accessor was defined without a setter");
  if (typeof e == "function" ? s !== e || !r : !e.has(s)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return n === "a" ? r.call(s, t) : r ? r.value = t : e.set(s, t), t;
}, re = function(s, e, t, n) {
  if (t === "a" && !n) throw new TypeError("Private accessor was defined without a getter");
  if (typeof e == "function" ? s !== e || !n : !e.has(s)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return t === "m" ? n : t === "a" ? n.call(s) : n ? n.value : e.get(s);
}, J;
class Ye {
  constructor() {
    J.set(this, void 0), this.buffer = new Uint8Array(), ke(this, J, null, "f");
  }
  decode(e) {
    if (e == null)
      return [];
    const t = e instanceof ArrayBuffer ? new Uint8Array(e) : typeof e == "string" ? new TextEncoder().encode(e) : e;
    let n = new Uint8Array(this.buffer.length + t.length);
    n.set(this.buffer), n.set(t, this.buffer.length), this.buffer = n;
    const r = [];
    let a;
    for (; (a = Js(this.buffer, re(this, J, "f"))) != null; ) {
      if (a.carriage && re(this, J, "f") == null) {
        ke(this, J, a.index, "f");
        continue;
      }
      if (re(this, J, "f") != null && (a.index !== re(this, J, "f") + 1 || a.carriage)) {
        r.push(this.decodeText(this.buffer.slice(0, re(this, J, "f") - 1))), this.buffer = this.buffer.slice(re(this, J, "f")), ke(this, J, null, "f");
        continue;
      }
      const i = re(this, J, "f") !== null ? a.preceding - 1 : a.preceding, o = this.decodeText(this.buffer.slice(0, i));
      r.push(o), this.buffer = this.buffer.slice(a.index), ke(this, J, null, "f");
    }
    return r;
  }
  decodeText(e) {
    if (e == null)
      return "";
    if (typeof e == "string")
      return e;
    if (typeof Buffer < "u") {
      if (e instanceof Buffer)
        return e.toString();
      if (e instanceof Uint8Array)
        return Buffer.from(e).toString();
      throw new _(`Unexpected: received non-Uint8Array (${e.constructor.name}) stream chunk in an environment with a global "Buffer" defined, which this library assumes to be Node. Please report this error.`);
    }
    if (typeof TextDecoder < "u") {
      if (e instanceof Uint8Array || e instanceof ArrayBuffer)
        return this.textDecoder ?? (this.textDecoder = new TextDecoder("utf8")), this.textDecoder.decode(e);
      throw new _(`Unexpected: received non-Uint8Array/ArrayBuffer (${e.constructor.name}) in a web platform. Please report this error.`);
    }
    throw new _("Unexpected: neither Buffer nor TextDecoder are available as globals. Please report this error.");
  }
  flush() {
    return this.buffer.length ? this.decode(`
`) : [];
  }
}
J = /* @__PURE__ */ new WeakMap();
Ye.NEWLINE_CHARS = /* @__PURE__ */ new Set([`
`, "\r"]);
Ye.NEWLINE_REGEXP = /\r\n|[\n\r]/g;
function Js(s, e) {
  for (let r = e ?? 0; r < s.length; r++) {
    if (s[r] === 10)
      return { preceding: r, index: r + 1, carriage: !1 };
    if (s[r] === 13)
      return { preceding: r, index: r + 1, carriage: !0 };
  }
  return null;
}
function Hs(s) {
  for (let n = 0; n < s.length - 1; n++) {
    if (s[n] === 10 && s[n + 1] === 10 || s[n] === 13 && s[n + 1] === 13)
      return n + 2;
    if (s[n] === 13 && s[n + 1] === 10 && n + 3 < s.length && s[n + 2] === 13 && s[n + 3] === 10)
      return n + 4;
  }
  return -1;
}
function Un(s) {
  if (s[Symbol.asyncIterator])
    return s;
  const e = s.getReader();
  return {
    async next() {
      try {
        const t = await e.read();
        return t?.done && e.releaseLock(), t;
      } catch (t) {
        throw e.releaseLock(), t;
      }
    },
    async return() {
      const t = e.cancel();
      return e.releaseLock(), await t, { done: !0, value: void 0 };
    },
    [Symbol.asyncIterator]() {
      return this;
    }
  };
}
class Z {
  constructor(e, t) {
    this.iterator = e, this.controller = t;
  }
  static fromSSEResponse(e, t) {
    let n = !1;
    async function* r() {
      if (n)
        throw new Error("Cannot iterate over a consumed stream, use `.tee()` to split the stream.");
      n = !0;
      let a = !1;
      try {
        for await (const i of Xs(e, t))
          if (!a) {
            if (i.data.startsWith("[DONE]")) {
              a = !0;
              continue;
            }
            if (i.event === null) {
              let o;
              try {
                o = JSON.parse(i.data);
              } catch (c) {
                throw console.error("Could not parse message into JSON:", i.data), console.error("From chunk:", i.raw), c;
              }
              if (o && o.error)
                throw new N(void 0, o.error, void 0, void 0);
              yield o;
            } else {
              let o;
              try {
                o = JSON.parse(i.data);
              } catch (c) {
                throw console.error("Could not parse message into JSON:", i.data), console.error("From chunk:", i.raw), c;
              }
              if (i.event == "error")
                throw new N(void 0, o.error, o.message, void 0);
              yield { event: i.event, data: o };
            }
          }
        a = !0;
      } catch (i) {
        if (i instanceof Error && i.name === "AbortError")
          return;
        throw i;
      } finally {
        a || t.abort();
      }
    }
    return new Z(r, t);
  }
  static fromReadableStream(e, t) {
    let n = !1;
    async function* r() {
      const i = new Ye(), o = Un(e);
      for await (const c of o)
        for (const u of i.decode(c))
          yield u;
      for (const c of i.flush())
        yield c;
    }
    async function* a() {
      if (n)
        throw new Error("Cannot iterate over a consumed stream, use `.tee()` to split the stream.");
      n = !0;
      let i = !1;
      try {
        for await (const o of r())
          i || o && (yield JSON.parse(o));
        i = !0;
      } catch (o) {
        if (o instanceof Error && o.name === "AbortError")
          return;
        throw o;
      } finally {
        i || t.abort();
      }
    }
    return new Z(a, t);
  }
  [Symbol.asyncIterator]() {
    return this.iterator();
  }
  tee() {
    const e = [], t = [], n = this.iterator(), r = (a) => ({
      next: () => {
        if (a.length === 0) {
          const i = n.next();
          e.push(i), t.push(i);
        }
        return a.shift();
      }
    });
    return [
      new Z(() => r(e), this.controller),
      new Z(() => r(t), this.controller)
    ];
  }
  toReadableStream() {
    const e = this;
    let t;
    const n = new TextEncoder();
    return new Pn({
      async start() {
        t = e[Symbol.asyncIterator]();
      },
      async pull(r) {
        try {
          const { value: a, done: i } = await t.next();
          if (i)
            return r.close();
          const o = n.encode(JSON.stringify(a) + `
`);
          r.enqueue(o);
        } catch (a) {
          r.error(a);
        }
      },
      async cancel() {
        await t.return?.();
      }
    });
  }
}
async function* Xs(s, e) {
  if (!s.body)
    throw e.abort(), new _("Attempted to iterate over a response with no body");
  const t = new Vs(), n = new Ye(), r = Un(s.body);
  for await (const a of qs(r))
    for (const i of n.decode(a)) {
      const o = t.decode(i);
      o && (yield o);
    }
  for (const a of n.flush()) {
    const i = t.decode(a);
    i && (yield i);
  }
}
async function* qs(s) {
  let e = new Uint8Array();
  for await (const t of s) {
    if (t == null)
      continue;
    const n = t instanceof ArrayBuffer ? new Uint8Array(t) : typeof t == "string" ? new TextEncoder().encode(t) : t;
    let r = new Uint8Array(e.length + n.length);
    r.set(e), r.set(n, e.length), e = r;
    let a;
    for (; (a = Hs(e)) !== -1; )
      yield e.slice(0, a), e = e.slice(a);
  }
  e.length > 0 && (yield e);
}
class Vs {
  constructor() {
    this.event = null, this.data = [], this.chunks = [];
  }
  decode(e) {
    if (e.endsWith("\r") && (e = e.substring(0, e.length - 1)), !e) {
      if (!this.event && !this.data.length)
        return null;
      const a = {
        event: this.event,
        data: this.data.join(`
`),
        raw: this.chunks
      };
      return this.event = null, this.data = [], this.chunks = [], a;
    }
    if (this.chunks.push(e), e.startsWith(":"))
      return null;
    let [t, n, r] = Ks(e, ":");
    return r.startsWith(" ") && (r = r.substring(1)), t === "event" ? this.event = r : t === "data" && this.data.push(r), null;
  }
}
function Ks(s, e) {
  const t = s.indexOf(e);
  return t !== -1 ? [s.substring(0, t), e, s.substring(t + e.length)] : [s, "", ""];
}
const jn = (s) => s != null && typeof s == "object" && typeof s.url == "string" && typeof s.blob == "function", Wn = (s) => s != null && typeof s == "object" && typeof s.name == "string" && typeof s.lastModified == "number" && Ze(s), Ze = (s) => s != null && typeof s == "object" && typeof s.size == "number" && typeof s.type == "string" && typeof s.text == "function" && typeof s.slice == "function" && typeof s.arrayBuffer == "function", Gs = (s) => Wn(s) || jn(s) || In(s);
async function Jn(s, e, t) {
  if (s = await s, Wn(s))
    return s;
  if (jn(s)) {
    const r = await s.blob();
    e || (e = new URL(s.url).pathname.split(/[\\/]/).pop() ?? "unknown_file");
    const a = Ze(r) ? [await r.arrayBuffer()] : [r];
    return new wt(a, e, t);
  }
  const n = await Qs(s);
  if (e || (e = Ys(s) ?? "unknown_file"), !t?.type) {
    const r = n[0]?.type;
    typeof r == "string" && (t = { ...t, type: r });
  }
  return new wt(n, e, t);
}
async function Qs(s) {
  let e = [];
  if (typeof s == "string" || ArrayBuffer.isView(s) || s instanceof ArrayBuffer)
    e.push(s);
  else if (Ze(s))
    e.push(await s.arrayBuffer());
  else if (Zs(s))
    for await (const t of s)
      e.push(t);
  else
    throw new Error(`Unexpected data type: ${typeof s}; constructor: ${s?.constructor?.name}; props: ${zs(s)}`);
  return e;
}
function zs(s) {
  return `[${Object.getOwnPropertyNames(s).map((t) => `"${t}"`).join(", ")}]`;
}
function Ys(s) {
  return lt(s.name) || lt(s.filename) || lt(s.path)?.split(/[\\/]/).pop();
}
const lt = (s) => {
  if (typeof s == "string")
    return s;
  if (typeof Buffer < "u" && s instanceof Buffer)
    return String(s);
}, Zs = (s) => s != null && typeof s == "object" && typeof s[Symbol.asyncIterator] == "function", nn = (s) => s && typeof s == "object" && s.body && s[Symbol.toStringTag] === "MultipartBody", pe = async (s) => {
  const e = await er(s.body);
  return En(e, s);
}, er = async (s) => {
  const e = new An();
  return await Promise.all(Object.entries(s || {}).map(([t, n]) => yt(e, t, n))), e;
}, yt = async (s, e, t) => {
  if (t !== void 0) {
    if (t == null)
      throw new TypeError(`Received null for "${e}"; to pass null in FormData, you must use the string 'null'`);
    if (typeof t == "string" || typeof t == "number" || typeof t == "boolean")
      s.append(e, String(t));
    else if (Gs(t)) {
      const n = await Jn(t);
      s.append(e, n);
    } else if (Array.isArray(t))
      await Promise.all(t.map((n) => yt(s, e + "[]", n)));
    else if (typeof t == "object")
      await Promise.all(Object.entries(t).map(([n, r]) => yt(s, `${e}[${n}]`, r)));
    else
      throw new TypeError(`Invalid value given to form, expected a string, number, boolean, object, Array, File or Blob but got ${t} instead`);
  }
};
var tr = function(s, e, t, n, r) {
  if (n === "m") throw new TypeError("Private method is not writable");
  if (n === "a" && !r) throw new TypeError("Private accessor was defined without a setter");
  if (typeof e == "function" ? s !== e || !r : !e.has(s)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return n === "a" ? r.call(s, t) : r ? r.value = t : e.set(s, t), t;
}, nr = function(s, e, t, n) {
  if (t === "a" && !n) throw new TypeError("Private accessor was defined without a getter");
  if (typeof e == "function" ? s !== e || !n : !e.has(s)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return t === "m" ? n : t === "a" ? n.call(s) : n ? n.value : e.get(s);
}, Me;
async function Hn(s) {
  const { response: e } = s;
  if (s.options.stream)
    return he("response", e.status, e.url, e.headers, e.body), s.options.__streamClass ? s.options.__streamClass.fromSSEResponse(e, s.controller) : Z.fromSSEResponse(e, s.controller);
  if (e.status === 204)
    return null;
  if (s.options.__binaryResponse)
    return e;
  const t = e.headers.get("content-type");
  if (t?.includes("application/json") || t?.includes("application/vnd.api+json")) {
    const a = await e.json();
    return he("response", e.status, e.url, e.headers, a), Xn(a, e);
  }
  const r = await e.text();
  return he("response", e.status, e.url, e.headers, r), r;
}
function Xn(s, e) {
  return !s || typeof s != "object" || Array.isArray(s) ? s : Object.defineProperty(s, "_request_id", {
    value: e.headers.get("x-request-id"),
    enumerable: !1
  });
}
class et extends Promise {
  constructor(e, t = Hn) {
    super((n) => {
      n(null);
    }), this.responsePromise = e, this.parseResponse = t;
  }
  _thenUnwrap(e) {
    return new et(this.responsePromise, async (t) => Xn(e(await this.parseResponse(t), t), t.response));
  }
  asResponse() {
    return this.responsePromise.then((e) => e.response);
  }
  async withResponse() {
    const [e, t] = await Promise.all([this.parse(), this.asResponse()]);
    return { data: e, response: t, request_id: t.headers.get("x-request-id") };
  }
  parse() {
    return this.parsedPromise || (this.parsedPromise = this.responsePromise.then(this.parseResponse)), this.parsedPromise;
  }
  then(e, t) {
    return this.parse().then(e, t);
  }
  catch(e) {
    return this.parse().catch(e);
  }
  finally(e) {
    return this.parse().finally(e);
  }
}
class sr {
  constructor({
    baseURL: e,
    maxRetries: t = 2,
    timeout: n = 6e5,
    httpAgent: r,
    fetch: a
  }) {
    this.baseURL = e, this.maxRetries = ct("maxRetries", t), this.timeout = ct("timeout", n), this.httpAgent = r, this.fetch = a ?? Cn;
  }
  authHeaders(e) {
    return {};
  }
  defaultHeaders(e) {
    return {
      Accept: "application/json",
      "Content-Type": "application/json",
      "User-Agent": this.getUserAgent(),
      ...cr(),
      ...this.authHeaders(e)
    };
  }
  validateHeaders(e, t) {
  }
  defaultIdempotencyKey() {
    return `stainless-node-retry-${dr()}`;
  }
  get(e, t) {
    return this.methodRequest("get", e, t);
  }
  post(e, t) {
    return this.methodRequest("post", e, t);
  }
  patch(e, t) {
    return this.methodRequest("patch", e, t);
  }
  put(e, t) {
    return this.methodRequest("put", e, t);
  }
  delete(e, t) {
    return this.methodRequest("delete", e, t);
  }
  methodRequest(e, t, n) {
    return this.request(Promise.resolve(n).then(async (r) => {
      const a = r && Ze(r?.body) ? new DataView(await r.body.arrayBuffer()) : r?.body instanceof DataView ? r.body : r?.body instanceof ArrayBuffer ? new DataView(r.body) : r && ArrayBuffer.isView(r?.body) ? new DataView(r.body.buffer) : r?.body;
      return { method: e, path: t, ...r, body: a };
    }));
  }
  getAPIList(e, t, n) {
    return this.requestAPIList(t, { method: "get", path: e, ...n });
  }
  calculateContentLength(e) {
    if (typeof e == "string") {
      if (typeof Buffer < "u")
        return Buffer.byteLength(e, "utf8").toString();
      if (typeof TextEncoder < "u")
        return new TextEncoder().encode(e).length.toString();
    } else if (ArrayBuffer.isView(e))
      return e.byteLength.toString();
    return null;
  }
  buildRequest(e, { retryCount: t = 0 } = {}) {
    e = { ...e };
    const { method: n, path: r, query: a, headers: i = {} } = e, o = ArrayBuffer.isView(e.body) || e.__binaryRequest && typeof e.body == "string" ? e.body : nn(e.body) ? e.body.body : e.body ? JSON.stringify(e.body, null, 2) : null, c = this.calculateContentLength(o), u = this.buildURL(r, a);
    "timeout" in e && ct("timeout", e.timeout), e.timeout = e.timeout ?? this.timeout;
    const g = e.httpAgent ?? this.httpAgent ?? Rn(u), l = e.timeout + 1e3;
    typeof g?.options?.timeout == "number" && l > (g.options.timeout ?? 0) && (g.options.timeout = l), this.idempotencyHeader && n !== "get" && (e.idempotencyKey || (e.idempotencyKey = this.defaultIdempotencyKey()), i[this.idempotencyHeader] = e.idempotencyKey);
    const h = this.buildHeaders({ options: e, headers: i, contentLength: c, retryCount: t });
    return { req: {
      method: n,
      ...o && { body: o },
      headers: h,
      ...g && { agent: g },
      signal: e.signal ?? null
    }, url: u, timeout: e.timeout };
  }
  buildHeaders({ options: e, headers: t, contentLength: n, retryCount: r }) {
    const a = {};
    n && (a["content-length"] = n);
    const i = this.defaultHeaders(e);
    return on(a, i), on(a, t), nn(e.body) && Pe !== "node" && delete a["content-type"], De(i, "x-stainless-retry-count") === void 0 && De(t, "x-stainless-retry-count") === void 0 && (a["x-stainless-retry-count"] = String(r)), De(i, "x-stainless-timeout") === void 0 && De(t, "x-stainless-timeout") === void 0 && e.timeout && (a["x-stainless-timeout"] = String(e.timeout)), this.validateHeaders(a, t), a;
  }
  async prepareOptions(e) {
  }
  async prepareRequest(e, { url: t, options: n }) {
  }
  parseHeaders(e) {
    return e ? Symbol.iterator in e ? Object.fromEntries(Array.from(e).map((t) => [...t])) : { ...e } : {};
  }
  makeStatusError(e, t, n, r) {
    return N.generate(e, t, n, r);
  }
  request(e, t = null) {
    return new et(this.makeRequest(e, t));
  }
  async makeRequest(e, t) {
    const n = await e, r = n.maxRetries ?? this.maxRetries;
    t == null && (t = r), await this.prepareOptions(n);
    const { req: a, url: i, timeout: o } = this.buildRequest(n, { retryCount: r - t });
    if (await this.prepareRequest(a, { url: i, options: n }), he("request", i, n, a.headers), n.signal?.aborted)
      throw new K();
    const c = new AbortController(), u = await this.fetchWithTimeout(i, a, o, c).catch(bt);
    if (u instanceof Error) {
      if (n.signal?.aborted)
        throw new K();
      if (t)
        return this.retryRequest(n, t);
      throw u.name === "AbortError" ? new It() : new ze({ cause: u });
    }
    const g = ar(u.headers);
    if (!u.ok) {
      if (t && this.shouldRetry(u)) {
        const A = `retrying, ${t} attempts remaining`;
        return he(`response (error; ${A})`, u.status, i, g), this.retryRequest(n, t, g);
      }
      const l = await u.text().catch((A) => bt(A).message), h = ur(l), f = h ? void 0 : l;
      throw he(`response (error; ${t ? "(error; no more retries left)" : "(error; not retryable)"})`, u.status, i, g, f), this.makeStatusError(u.status, h, f, g);
    }
    return { response: u, options: n, controller: c };
  }
  requestAPIList(e, t) {
    const n = this.makeRequest(t, null);
    return new rr(this, n, e);
  }
  buildURL(e, t) {
    const n = hr(e) ? new URL(e) : new URL(this.baseURL + (this.baseURL.endsWith("/") && e.startsWith("/") ? e.slice(1) : e)), r = this.defaultQuery();
    return Vn(r) || (t = { ...r, ...t }), typeof t == "object" && t && !Array.isArray(t) && (n.search = this.stringifyQuery(t)), n.toString();
  }
  stringifyQuery(e) {
    return Object.entries(e).filter(([t, n]) => typeof n < "u").map(([t, n]) => {
      if (typeof n == "string" || typeof n == "number" || typeof n == "boolean")
        return `${encodeURIComponent(t)}=${encodeURIComponent(n)}`;
      if (n === null)
        return `${encodeURIComponent(t)}=`;
      throw new _(`Cannot stringify type ${typeof n}; Expected string, number, boolean, or null. If you need to pass nested query parameters, you can manually encode them, e.g. { query: { 'foo[key1]': value1, 'foo[key2]': value2 } }, and please open a GitHub issue requesting better support for your use case.`);
    }).join("&");
  }
  async fetchWithTimeout(e, t, n, r) {
    const { signal: a, ...i } = t || {};
    a && a.addEventListener("abort", () => r.abort());
    const o = setTimeout(() => r.abort(), n), c = {
      signal: r.signal,
      ...i
    };
    return c.method && (c.method = c.method.toUpperCase()), this.fetch.call(void 0, e, c).finally(() => {
      clearTimeout(o);
    });
  }
  shouldRetry(e) {
    const t = e.headers.get("x-should-retry");
    return t === "true" ? !0 : t === "false" ? !1 : e.status === 408 || e.status === 409 || e.status === 429 || e.status >= 500;
  }
  async retryRequest(e, t, n) {
    let r;
    const a = n?.["retry-after-ms"];
    if (a) {
      const o = parseFloat(a);
      Number.isNaN(o) || (r = o);
    }
    const i = n?.["retry-after"];
    if (i && !r) {
      const o = parseFloat(i);
      Number.isNaN(o) ? r = Date.parse(i) - Date.now() : r = o * 1e3;
    }
    if (!(r && 0 <= r && r < 60 * 1e3)) {
      const o = e.maxRetries ?? this.maxRetries;
      r = this.calculateDefaultRetryTimeoutMillis(t, o);
    }
    return await Ie(r), this.makeRequest(e, t - 1);
  }
  calculateDefaultRetryTimeoutMillis(e, t) {
    const a = t - e, i = Math.min(0.5 * Math.pow(2, a), 8), o = 1 - Math.random() * 0.25;
    return i * o * 1e3;
  }
  getUserAgent() {
    return `${this.constructor.name}/JS ${ce}`;
  }
}
class qn {
  constructor(e, t, n, r) {
    Me.set(this, void 0), tr(this, Me, e, "f"), this.options = r, this.response = t, this.body = n;
  }
  hasNextPage() {
    return this.getPaginatedItems().length ? this.nextPageInfo() != null : !1;
  }
  async getNextPage() {
    const e = this.nextPageInfo();
    if (!e)
      throw new _("No next page expected; please check `.hasNextPage()` before calling `.getNextPage()`.");
    const t = { ...this.options };
    if ("params" in e && typeof t.query == "object")
      t.query = { ...t.query, ...e.params };
    else if ("url" in e) {
      const n = [...Object.entries(t.query || {}), ...e.url.searchParams.entries()];
      for (const [r, a] of n)
        e.url.searchParams.set(r, a);
      t.query = void 0, t.path = e.url.toString();
    }
    return await nr(this, Me, "f").requestAPIList(this.constructor, t);
  }
  async *iterPages() {
    let e = this;
    for (yield e; e.hasNextPage(); )
      e = await e.getNextPage(), yield e;
  }
  async *[(Me = /* @__PURE__ */ new WeakMap(), Symbol.asyncIterator)]() {
    for await (const e of this.iterPages())
      for (const t of e.getPaginatedItems())
        yield t;
  }
}
class rr extends et {
  constructor(e, t, n) {
    super(t, async (r) => new n(e, r.response, await Hn(r), r.options));
  }
  async *[Symbol.asyncIterator]() {
    const e = await this;
    for await (const t of e)
      yield t;
  }
}
const ar = (s) => new Proxy(Object.fromEntries(
  s.entries()
), {
  get(e, t) {
    const n = t.toString();
    return e[n.toLowerCase()] || e[n];
  }
}), ir = {
  method: !0,
  path: !0,
  query: !0,
  body: !0,
  headers: !0,
  maxRetries: !0,
  stream: !0,
  timeout: !0,
  httpAgent: !0,
  signal: !0,
  idempotencyKey: !0,
  __metadata: !0,
  __binaryRequest: !0,
  __binaryResponse: !0,
  __streamClass: !0
}, U = (s) => typeof s == "object" && s !== null && !Vn(s) && Object.keys(s).every((e) => Kn(ir, e)), or = () => {
  if (typeof Deno < "u" && Deno.build != null)
    return {
      "X-Stainless-Lang": "js",
      "X-Stainless-Package-Version": ce,
      "X-Stainless-OS": rn(Deno.build.os),
      "X-Stainless-Arch": sn(Deno.build.arch),
      "X-Stainless-Runtime": "deno",
      "X-Stainless-Runtime-Version": typeof Deno.version == "string" ? Deno.version : Deno.version?.deno ?? "unknown"
    };
  if (typeof EdgeRuntime < "u")
    return {
      "X-Stainless-Lang": "js",
      "X-Stainless-Package-Version": ce,
      "X-Stainless-OS": "Unknown",
      "X-Stainless-Arch": `other:${EdgeRuntime}`,
      "X-Stainless-Runtime": "edge",
      "X-Stainless-Runtime-Version": process.version
    };
  if (Object.prototype.toString.call(typeof process < "u" ? process : 0) === "[object process]")
    return {
      "X-Stainless-Lang": "js",
      "X-Stainless-Package-Version": ce,
      "X-Stainless-OS": rn(process.platform),
      "X-Stainless-Arch": sn(process.arch),
      "X-Stainless-Runtime": "node",
      "X-Stainless-Runtime-Version": process.version
    };
  const s = lr();
  return s ? {
    "X-Stainless-Lang": "js",
    "X-Stainless-Package-Version": ce,
    "X-Stainless-OS": "Unknown",
    "X-Stainless-Arch": "unknown",
    "X-Stainless-Runtime": `browser:${s.browser}`,
    "X-Stainless-Runtime-Version": s.version
  } : {
    "X-Stainless-Lang": "js",
    "X-Stainless-Package-Version": ce,
    "X-Stainless-OS": "Unknown",
    "X-Stainless-Arch": "unknown",
    "X-Stainless-Runtime": "unknown",
    "X-Stainless-Runtime-Version": "unknown"
  };
};
function lr() {
  if (typeof navigator > "u" || !navigator)
    return null;
  const s = [
    { key: "edge", pattern: /Edge(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "ie", pattern: /MSIE(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "ie", pattern: /Trident(?:.*rv\:(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "chrome", pattern: /Chrome(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "firefox", pattern: /Firefox(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "safari", pattern: /(?:Version\W+(\d+)\.(\d+)(?:\.(\d+))?)?(?:\W+Mobile\S*)?\W+Safari/ }
  ];
  for (const { key: e, pattern: t } of s) {
    const n = t.exec(navigator.userAgent);
    if (n) {
      const r = n[1] || 0, a = n[2] || 0, i = n[3] || 0;
      return { browser: e, version: `${r}.${a}.${i}` };
    }
  }
  return null;
}
const sn = (s) => s === "x32" ? "x32" : s === "x86_64" || s === "x64" ? "x64" : s === "arm" ? "arm" : s === "aarch64" || s === "arm64" ? "arm64" : s ? `other:${s}` : "unknown", rn = (s) => (s = s.toLowerCase(), s.includes("ios") ? "iOS" : s === "android" ? "Android" : s === "darwin" ? "MacOS" : s === "win32" ? "Windows" : s === "freebsd" ? "FreeBSD" : s === "openbsd" ? "OpenBSD" : s === "linux" ? "Linux" : s ? `Other:${s}` : "Unknown");
let an;
const cr = () => an ?? (an = or()), ur = (s) => {
  try {
    return JSON.parse(s);
  } catch {
    return;
  }
}, fr = /^[a-z][a-z0-9+.-]*:/i, hr = (s) => fr.test(s), Ie = (s) => new Promise((e) => setTimeout(e, s)), ct = (s, e) => {
  if (typeof e != "number" || !Number.isInteger(e))
    throw new _(`${s} must be an integer`);
  if (e < 0)
    throw new _(`${s} must be a positive integer`);
  return e;
}, bt = (s) => {
  if (s instanceof Error)
    return s;
  if (typeof s == "object" && s !== null)
    try {
      return new Error(JSON.stringify(s));
    } catch {
    }
  return new Error(s);
}, Ne = (s) => {
  if (typeof process < "u")
    return process.env?.[s]?.trim() ?? void 0;
  if (typeof Deno < "u")
    return Deno.env?.get?.(s)?.trim();
};
function Vn(s) {
  if (!s)
    return !0;
  for (const e in s)
    return !1;
  return !0;
}
function Kn(s, e) {
  return Object.prototype.hasOwnProperty.call(s, e);
}
function on(s, e) {
  for (const t in e) {
    if (!Kn(e, t))
      continue;
    const n = t.toLowerCase();
    if (!n)
      continue;
    const r = e[t];
    r === null ? delete s[n] : r !== void 0 && (s[n] = r);
  }
}
const ln = /* @__PURE__ */ new Set(["authorization", "api-key"]);
function he(s, ...e) {
  if (typeof process < "u" && process?.env?.DEBUG === "true") {
    const t = e.map((n) => {
      if (!n)
        return n;
      if (n.headers) {
        const a = { ...n, headers: { ...n.headers } };
        for (const i in n.headers)
          ln.has(i.toLowerCase()) && (a.headers[i] = "REDACTED");
        return a;
      }
      let r = null;
      for (const a in n)
        ln.has(a.toLowerCase()) && (r ?? (r = { ...n }), r[a] = "REDACTED");
      return r ?? n;
    });
    console.log(`OpenAI:DEBUG:${s}`, ...t);
  }
}
const dr = () => "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (s) => {
  const e = Math.random() * 16 | 0;
  return (s === "x" ? e : e & 3 | 8).toString(16);
}), mr = () => typeof window < "u" && typeof window.document < "u" && typeof navigator < "u", pr = (s) => typeof s?.get == "function", De = (s, e) => {
  const t = e.toLowerCase();
  if (pr(s)) {
    const n = e[0]?.toUpperCase() + e.substring(1).replace(/([^\w])(\w)/g, (r, a, i) => a + i.toUpperCase());
    for (const r of [e, t, e.toUpperCase(), n]) {
      const a = s.get(r);
      if (a)
        return a;
    }
  }
  for (const [n, r] of Object.entries(s))
    if (n.toLowerCase() === t)
      return Array.isArray(r) ? (r.length <= 1 || console.warn(`Received ${r.length} entries for the ${e} header, using the first entry.`), r[0]) : r;
};
function ut(s) {
  return s != null && typeof s == "object" && !Array.isArray(s);
}
class gr extends qn {
  constructor(e, t, n, r) {
    super(e, t, n, r), this.data = n.data || [], this.object = n.object;
  }
  getPaginatedItems() {
    return this.data ?? [];
  }
  nextPageParams() {
    return null;
  }
  nextPageInfo() {
    return null;
  }
}
class X extends qn {
  constructor(e, t, n, r) {
    super(e, t, n, r), this.data = n.data || [], this.has_more = n.has_more || !1;
  }
  getPaginatedItems() {
    return this.data ?? [];
  }
  hasNextPage() {
    return this.has_more === !1 ? !1 : super.hasNextPage();
  }
  nextPageParams() {
    const e = this.nextPageInfo();
    if (!e)
      return null;
    if ("params" in e)
      return e.params;
    const t = Object.fromEntries(e.url.searchParams);
    return Object.keys(t).length ? t : null;
  }
  nextPageInfo() {
    const e = this.getPaginatedItems();
    if (!e.length)
      return null;
    const t = e[e.length - 1]?.id;
    return t ? { params: { after: t } } : null;
  }
}
class y {
  constructor(e) {
    this._client = e;
  }
}
let Gn = class extends y {
  list(e, t = {}, n) {
    return U(t) ? this.list(e, {}, t) : this._client.getAPIList(`/chat/completions/${e}/messages`, _r, { query: t, ...n });
  }
}, tt = class extends y {
  constructor() {
    super(...arguments), this.messages = new Gn(this._client);
  }
  create(e, t) {
    return this._client.post("/chat/completions", { body: e, ...t, stream: e.stream ?? !1 });
  }
  retrieve(e, t) {
    return this._client.get(`/chat/completions/${e}`, t);
  }
  update(e, t, n) {
    return this._client.post(`/chat/completions/${e}`, { body: t, ...n });
  }
  list(e = {}, t) {
    return U(e) ? this.list({}, e) : this._client.getAPIList("/chat/completions", nt, { query: e, ...t });
  }
  del(e, t) {
    return this._client.delete(`/chat/completions/${e}`, t);
  }
};
class nt extends X {
}
class _r extends X {
}
tt.ChatCompletionsPage = nt;
tt.Messages = Gn;
let st = class extends y {
  constructor() {
    super(...arguments), this.completions = new tt(this._client);
  }
};
st.Completions = tt;
st.ChatCompletionsPage = nt;
class Qn extends y {
  create(e, t) {
    return this._client.post("/audio/speech", {
      body: e,
      ...t,
      headers: { Accept: "application/octet-stream", ...t?.headers },
      __binaryResponse: !0
    });
  }
}
class zn extends y {
  create(e, t) {
    return this._client.post("/audio/transcriptions", pe({ body: e, ...t, __metadata: { model: e.model } }));
  }
}
class Yn extends y {
  create(e, t) {
    return this._client.post("/audio/translations", pe({ body: e, ...t, __metadata: { model: e.model } }));
  }
}
class Oe extends y {
  constructor() {
    super(...arguments), this.transcriptions = new zn(this._client), this.translations = new Yn(this._client), this.speech = new Qn(this._client);
  }
}
Oe.Transcriptions = zn;
Oe.Translations = Yn;
Oe.Speech = Qn;
class Ot extends y {
  create(e, t) {
    return this._client.post("/batches", { body: e, ...t });
  }
  retrieve(e, t) {
    return this._client.get(`/batches/${e}`, t);
  }
  list(e = {}, t) {
    return U(e) ? this.list({}, e) : this._client.getAPIList("/batches", $t, { query: e, ...t });
  }
  cancel(e, t) {
    return this._client.post(`/batches/${e}/cancel`, t);
  }
}
class $t extends X {
}
Ot.BatchesPage = $t;
class Ft extends y {
  create(e, t) {
    return this._client.post("/assistants", {
      body: e,
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  retrieve(e, t) {
    return this._client.get(`/assistants/${e}`, {
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  update(e, t, n) {
    return this._client.post(`/assistants/${e}`, {
      body: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  list(e = {}, t) {
    return U(e) ? this.list({}, e) : this._client.getAPIList("/assistants", Tt, {
      query: e,
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  del(e, t) {
    return this._client.delete(`/assistants/${e}`, {
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
}
class Tt extends X {
}
Ft.AssistantsPage = Tt;
function cn(s) {
  return typeof s.parse == "function";
}
const de = (s) => s?.role === "assistant", Zn = (s) => s?.role === "function", es = (s) => s?.role === "tool";
var q = function(s, e, t, n, r) {
  if (n === "m") throw new TypeError("Private method is not writable");
  if (n === "a" && !r) throw new TypeError("Private accessor was defined without a setter");
  if (typeof e == "function" ? s !== e || !r : !e.has(s)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return n === "a" ? r.call(s, t) : r ? r.value = t : e.set(s, t), t;
}, S = function(s, e, t, n) {
  if (t === "a" && !n) throw new TypeError("Private accessor was defined without a getter");
  if (typeof e == "function" ? s !== e || !n : !e.has(s)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return t === "m" ? n : t === "a" ? n.call(s) : n ? n.value : e.get(s);
}, xt, je, We, ye, be, Je, xe, te, Se, Ve, Ke, ue, ts;
class ns {
  constructor() {
    xt.add(this), this.controller = new AbortController(), je.set(this, void 0), We.set(this, () => {
    }), ye.set(this, () => {
    }), be.set(this, void 0), Je.set(this, () => {
    }), xe.set(this, () => {
    }), te.set(this, {}), Se.set(this, !1), Ve.set(this, !1), Ke.set(this, !1), ue.set(this, !1), q(this, je, new Promise((e, t) => {
      q(this, We, e, "f"), q(this, ye, t, "f");
    }), "f"), q(this, be, new Promise((e, t) => {
      q(this, Je, e, "f"), q(this, xe, t, "f");
    }), "f"), S(this, je, "f").catch(() => {
    }), S(this, be, "f").catch(() => {
    });
  }
  _run(e) {
    setTimeout(() => {
      e().then(() => {
        this._emitFinal(), this._emit("end");
      }, S(this, xt, "m", ts).bind(this));
    }, 0);
  }
  _connected() {
    this.ended || (S(this, We, "f").call(this), this._emit("connect"));
  }
  get ended() {
    return S(this, Se, "f");
  }
  get errored() {
    return S(this, Ve, "f");
  }
  get aborted() {
    return S(this, Ke, "f");
  }
  abort() {
    this.controller.abort();
  }
  on(e, t) {
    return (S(this, te, "f")[e] || (S(this, te, "f")[e] = [])).push({ listener: t }), this;
  }
  off(e, t) {
    const n = S(this, te, "f")[e];
    if (!n)
      return this;
    const r = n.findIndex((a) => a.listener === t);
    return r >= 0 && n.splice(r, 1), this;
  }
  once(e, t) {
    return (S(this, te, "f")[e] || (S(this, te, "f")[e] = [])).push({ listener: t, once: !0 }), this;
  }
  emitted(e) {
    return new Promise((t, n) => {
      q(this, ue, !0, "f"), e !== "error" && this.once("error", n), this.once(e, t);
    });
  }
  async done() {
    q(this, ue, !0, "f"), await S(this, be, "f");
  }
  _emit(e, ...t) {
    if (S(this, Se, "f"))
      return;
    e === "end" && (q(this, Se, !0, "f"), S(this, Je, "f").call(this));
    const n = S(this, te, "f")[e];
    if (n && (S(this, te, "f")[e] = n.filter((r) => !r.once), n.forEach(({ listener: r }) => r(...t))), e === "abort") {
      const r = t[0];
      !S(this, ue, "f") && !n?.length && Promise.reject(r), S(this, ye, "f").call(this, r), S(this, xe, "f").call(this, r), this._emit("end");
      return;
    }
    if (e === "error") {
      const r = t[0];
      !S(this, ue, "f") && !n?.length && Promise.reject(r), S(this, ye, "f").call(this, r), S(this, xe, "f").call(this, r), this._emit("end");
    }
  }
  _emitFinal() {
  }
}
je = /* @__PURE__ */ new WeakMap(), We = /* @__PURE__ */ new WeakMap(), ye = /* @__PURE__ */ new WeakMap(), be = /* @__PURE__ */ new WeakMap(), Je = /* @__PURE__ */ new WeakMap(), xe = /* @__PURE__ */ new WeakMap(), te = /* @__PURE__ */ new WeakMap(), Se = /* @__PURE__ */ new WeakMap(), Ve = /* @__PURE__ */ new WeakMap(), Ke = /* @__PURE__ */ new WeakMap(), ue = /* @__PURE__ */ new WeakMap(), xt = /* @__PURE__ */ new WeakSet(), ts = function(e) {
  if (q(this, Ve, !0, "f"), e instanceof Error && e.name === "AbortError" && (e = new K()), e instanceof K)
    return q(this, Ke, !0, "f"), this._emit("abort", e);
  if (e instanceof _)
    return this._emit("error", e);
  if (e instanceof Error) {
    const t = new _(e.message);
    return t.cause = e, this._emit("error", t);
  }
  return this._emit("error", new _(String(e)));
};
function ss(s) {
  return s?.$brand === "auto-parseable-response-format";
}
function $e(s) {
  return s?.$brand === "auto-parseable-tool";
}
function wr(s, e) {
  return !e || !rs(e) ? {
    ...s,
    choices: s.choices.map((t) => ({
      ...t,
      message: {
        ...t.message,
        parsed: null,
        ...t.message.tool_calls ? {
          tool_calls: t.message.tool_calls
        } : void 0
      }
    }))
  } : kt(s, e);
}
function kt(s, e) {
  const t = s.choices.map((n) => {
    if (n.finish_reason === "length")
      throw new Bn();
    if (n.finish_reason === "content_filter")
      throw new Ln();
    return {
      ...n,
      message: {
        ...n.message,
        ...n.message.tool_calls ? {
          tool_calls: n.message.tool_calls?.map((r) => br(e, r)) ?? void 0
        } : void 0,
        parsed: n.message.content && !n.message.refusal ? yr(e, n.message.content) : null
      }
    };
  });
  return { ...s, choices: t };
}
function yr(s, e) {
  return s.response_format?.type !== "json_schema" ? null : s.response_format?.type === "json_schema" ? "$parseRaw" in s.response_format ? s.response_format.$parseRaw(e) : JSON.parse(e) : null;
}
function br(s, e) {
  const t = s.tools?.find((n) => n.function?.name === e.function.name);
  return {
    ...e,
    function: {
      ...e.function,
      parsed_arguments: $e(t) ? t.$parseRaw(e.function.arguments) : t?.function.strict ? JSON.parse(e.function.arguments) : null
    }
  };
}
function xr(s, e) {
  if (!s)
    return !1;
  const t = s.tools?.find((n) => n.function?.name === e.function.name);
  return $e(t) || t?.function.strict || !1;
}
function rs(s) {
  return ss(s.response_format) ? !0 : s.tools?.some((e) => $e(e) || e.type === "function" && e.function.strict === !0) ?? !1;
}
function Sr(s) {
  for (const e of s ?? []) {
    if (e.type !== "function")
      throw new _(`Currently only \`function\` tool types support auto-parsing; Received \`${e.type}\``);
    if (e.function.strict !== !0)
      throw new _(`The \`${e.function.name}\` tool is not marked with \`strict: true\`. Only strict function tools can be auto-parsed`);
  }
}
var j = function(s, e, t, n) {
  if (t === "a" && !n) throw new TypeError("Private accessor was defined without a getter");
  if (typeof e == "function" ? s !== e || !n : !e.has(s)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return t === "m" ? n : t === "a" ? n.call(s) : n ? n.value : e.get(s);
}, L, St, Ge, Ct, At, Pt, as, Et;
const un = 10;
class is extends ns {
  constructor() {
    super(...arguments), L.add(this), this._chatCompletions = [], this.messages = [];
  }
  _addChatCompletion(e) {
    this._chatCompletions.push(e), this._emit("chatCompletion", e);
    const t = e.choices[0]?.message;
    return t && this._addMessage(t), e;
  }
  _addMessage(e, t = !0) {
    if ("content" in e || (e.content = null), this.messages.push(e), t) {
      if (this._emit("message", e), (Zn(e) || es(e)) && e.content)
        this._emit("functionCallResult", e.content);
      else if (de(e) && e.function_call)
        this._emit("functionCall", e.function_call);
      else if (de(e) && e.tool_calls)
        for (const n of e.tool_calls)
          n.type === "function" && this._emit("functionCall", n.function);
    }
  }
  async finalChatCompletion() {
    await this.done();
    const e = this._chatCompletions[this._chatCompletions.length - 1];
    if (!e)
      throw new _("stream ended without producing a ChatCompletion");
    return e;
  }
  async finalContent() {
    return await this.done(), j(this, L, "m", St).call(this);
  }
  async finalMessage() {
    return await this.done(), j(this, L, "m", Ge).call(this);
  }
  async finalFunctionCall() {
    return await this.done(), j(this, L, "m", Ct).call(this);
  }
  async finalFunctionCallResult() {
    return await this.done(), j(this, L, "m", At).call(this);
  }
  async totalUsage() {
    return await this.done(), j(this, L, "m", Pt).call(this);
  }
  allChatCompletions() {
    return [...this._chatCompletions];
  }
  _emitFinal() {
    const e = this._chatCompletions[this._chatCompletions.length - 1];
    e && this._emit("finalChatCompletion", e);
    const t = j(this, L, "m", Ge).call(this);
    t && this._emit("finalMessage", t);
    const n = j(this, L, "m", St).call(this);
    n && this._emit("finalContent", n);
    const r = j(this, L, "m", Ct).call(this);
    r && this._emit("finalFunctionCall", r);
    const a = j(this, L, "m", At).call(this);
    a != null && this._emit("finalFunctionCallResult", a), this._chatCompletions.some((i) => i.usage) && this._emit("totalUsage", j(this, L, "m", Pt).call(this));
  }
  async _createChatCompletion(e, t, n) {
    const r = n?.signal;
    r && (r.aborted && this.controller.abort(), r.addEventListener("abort", () => this.controller.abort())), j(this, L, "m", as).call(this, t);
    const a = await e.chat.completions.create({ ...t, stream: !1 }, { ...n, signal: this.controller.signal });
    return this._connected(), this._addChatCompletion(kt(a, t));
  }
  async _runChatCompletion(e, t, n) {
    for (const r of t.messages)
      this._addMessage(r, !1);
    return await this._createChatCompletion(e, t, n);
  }
  async _runFunctions(e, t, n) {
    const r = "function", { function_call: a = "auto", stream: i, ...o } = t, c = typeof a != "string" && a?.name, { maxChatCompletions: u = un } = n || {}, g = {};
    for (const h of t.functions)
      g[h.name || h.function.name] = h;
    const l = t.functions.map((h) => ({
      name: h.name || h.function.name,
      parameters: h.parameters,
      description: h.description
    }));
    for (const h of t.messages)
      this._addMessage(h, !1);
    for (let h = 0; h < u; ++h) {
      const b = (await this._createChatCompletion(e, {
        ...o,
        function_call: a,
        functions: l,
        messages: [...this.messages]
      }, n)).choices[0]?.message;
      if (!b)
        throw new _("missing message in ChatCompletion response");
      if (!b.function_call)
        return;
      const { name: m, arguments: A } = b.function_call, E = g[m];
      if (E) {
        if (c && c !== m) {
          const T = `Invalid function_call: ${JSON.stringify(m)}. ${JSON.stringify(c)} requested. Please try again`;
          this._addMessage({ role: r, name: m, content: T });
          continue;
        }
      } else {
        const T = `Invalid function_call: ${JSON.stringify(m)}. Available options are: ${l.map((D) => JSON.stringify(D.name)).join(", ")}. Please try again`;
        this._addMessage({ role: r, name: m, content: T });
        continue;
      }
      let p;
      try {
        p = cn(E) ? await E.parse(A) : A;
      } catch (T) {
        this._addMessage({
          role: r,
          name: m,
          content: T instanceof Error ? T.message : String(T)
        });
        continue;
      }
      const F = await E.function(p, this), C = j(this, L, "m", Et).call(this, F);
      if (this._addMessage({ role: r, name: m, content: C }), c)
        return;
    }
  }
  async _runTools(e, t, n) {
    const r = "tool", { tool_choice: a = "auto", stream: i, ...o } = t, c = typeof a != "string" && a?.function?.name, { maxChatCompletions: u = un } = n || {}, g = t.tools.map((f) => {
      if ($e(f)) {
        if (!f.$callback)
          throw new _("Tool given to `.runTools()` that does not have an associated function");
        return {
          type: "function",
          function: {
            function: f.$callback,
            name: f.function.name,
            description: f.function.description || "",
            parameters: f.function.parameters,
            parse: f.$parseRaw,
            strict: !0
          }
        };
      }
      return f;
    }), l = {};
    for (const f of g)
      f.type === "function" && (l[f.function.name || f.function.function.name] = f.function);
    const h = "tools" in t ? g.map((f) => f.type === "function" ? {
      type: "function",
      function: {
        name: f.function.name || f.function.function.name,
        parameters: f.function.parameters,
        description: f.function.description,
        strict: f.function.strict
      }
    } : f) : void 0;
    for (const f of t.messages)
      this._addMessage(f, !1);
    for (let f = 0; f < u; ++f) {
      const m = (await this._createChatCompletion(e, {
        ...o,
        tool_choice: a,
        tools: h,
        messages: [...this.messages]
      }, n)).choices[0]?.message;
      if (!m)
        throw new _("missing message in ChatCompletion response");
      if (!m.tool_calls?.length)
        return;
      for (const A of m.tool_calls) {
        if (A.type !== "function")
          continue;
        const E = A.id, { name: p, arguments: F } = A.function, C = l[p];
        if (C) {
          if (c && c !== p) {
            const k = `Invalid tool_call: ${JSON.stringify(p)}. ${JSON.stringify(c)} requested. Please try again`;
            this._addMessage({ role: r, tool_call_id: E, content: k });
            continue;
          }
        } else {
          const k = `Invalid tool_call: ${JSON.stringify(p)}. Available options are: ${Object.keys(l).map((O) => JSON.stringify(O)).join(", ")}. Please try again`;
          this._addMessage({ role: r, tool_call_id: E, content: k });
          continue;
        }
        let T;
        try {
          T = cn(C) ? await C.parse(F) : F;
        } catch (k) {
          const O = k instanceof Error ? k.message : String(k);
          this._addMessage({ role: r, tool_call_id: E, content: O });
          continue;
        }
        const D = await C.function(T, this), B = j(this, L, "m", Et).call(this, D);
        if (this._addMessage({ role: r, tool_call_id: E, content: B }), c)
          return;
      }
    }
  }
}
L = /* @__PURE__ */ new WeakSet(), St = function() {
  return j(this, L, "m", Ge).call(this).content ?? null;
}, Ge = function() {
  let e = this.messages.length;
  for (; e-- > 0; ) {
    const t = this.messages[e];
    if (de(t)) {
      const { function_call: n, ...r } = t, a = {
        ...r,
        content: t.content ?? null,
        refusal: t.refusal ?? null
      };
      return n && (a.function_call = n), a;
    }
  }
  throw new _("stream ended without producing a ChatCompletionMessage with role=assistant");
}, Ct = function() {
  for (let e = this.messages.length - 1; e >= 0; e--) {
    const t = this.messages[e];
    if (de(t) && t?.function_call)
      return t.function_call;
    if (de(t) && t?.tool_calls?.length)
      return t.tool_calls.at(-1)?.function;
  }
}, At = function() {
  for (let e = this.messages.length - 1; e >= 0; e--) {
    const t = this.messages[e];
    if (Zn(t) && t.content != null || es(t) && t.content != null && typeof t.content == "string" && this.messages.some((n) => n.role === "assistant" && n.tool_calls?.some((r) => r.type === "function" && r.id === t.tool_call_id)))
      return t.content;
  }
}, Pt = function() {
  const e = {
    completion_tokens: 0,
    prompt_tokens: 0,
    total_tokens: 0
  };
  for (const { usage: t } of this._chatCompletions)
    t && (e.completion_tokens += t.completion_tokens, e.prompt_tokens += t.prompt_tokens, e.total_tokens += t.total_tokens);
  return e;
}, as = function(e) {
  if (e.n != null && e.n > 1)
    throw new _("ChatCompletion convenience helpers only support n=1 at this time. To use n>1, please use chat.completions.create() directly.");
}, Et = function(e) {
  return typeof e == "string" ? e : e === void 0 ? "undefined" : JSON.stringify(e);
};
class Re extends is {
  static runFunctions(e, t, n) {
    const r = new Re(), a = {
      ...n,
      headers: { ...n?.headers, "X-Stainless-Helper-Method": "runFunctions" }
    };
    return r._run(() => r._runFunctions(e, t, a)), r;
  }
  static runTools(e, t, n) {
    const r = new Re(), a = {
      ...n,
      headers: { ...n?.headers, "X-Stainless-Helper-Method": "runTools" }
    };
    return r._run(() => r._runTools(e, t, a)), r;
  }
  _addMessage(e, t = !0) {
    super._addMessage(e, t), de(e) && e.content && this._emit("content", e.content);
  }
}
const os = 1, ls = 2, cs = 4, us = 8, fs = 16, hs = 32, ds = 64, ms = 128, ps = 256, gs = ms | ps, _s = fs | hs | gs | ds, ws = os | ls | _s, ys = cs | us, Cr = ws | ys, $ = {
  STR: os,
  NUM: ls,
  ARR: cs,
  OBJ: us,
  NULL: fs,
  BOOL: hs,
  NAN: ds,
  INFINITY: ms,
  MINUS_INFINITY: ps,
  INF: gs,
  SPECIAL: _s,
  ATOM: ws,
  COLLECTION: ys,
  ALL: Cr
};
class Ar extends Error {
}
class Pr extends Error {
}
function Er(s, e = $.ALL) {
  if (typeof s != "string")
    throw new TypeError(`expecting str, got ${typeof s}`);
  if (!s.trim())
    throw new Error(`${s} is empty`);
  return Rr(s.trim(), e);
}
const Rr = (s, e) => {
  const t = s.length;
  let n = 0;
  const r = (h) => {
    throw new Ar(`${h} at position ${n}`);
  }, a = (h) => {
    throw new Pr(`${h} at position ${n}`);
  }, i = () => (l(), n >= t && r("Unexpected end of input"), s[n] === '"' ? o() : s[n] === "{" ? c() : s[n] === "[" ? u() : s.substring(n, n + 4) === "null" || $.NULL & e && t - n < 4 && "null".startsWith(s.substring(n)) ? (n += 4, null) : s.substring(n, n + 4) === "true" || $.BOOL & e && t - n < 4 && "true".startsWith(s.substring(n)) ? (n += 4, !0) : s.substring(n, n + 5) === "false" || $.BOOL & e && t - n < 5 && "false".startsWith(s.substring(n)) ? (n += 5, !1) : s.substring(n, n + 8) === "Infinity" || $.INFINITY & e && t - n < 8 && "Infinity".startsWith(s.substring(n)) ? (n += 8, 1 / 0) : s.substring(n, n + 9) === "-Infinity" || $.MINUS_INFINITY & e && 1 < t - n && t - n < 9 && "-Infinity".startsWith(s.substring(n)) ? (n += 9, -1 / 0) : s.substring(n, n + 3) === "NaN" || $.NAN & e && t - n < 3 && "NaN".startsWith(s.substring(n)) ? (n += 3, NaN) : g()), o = () => {
    const h = n;
    let f = !1;
    for (n++; n < t && (s[n] !== '"' || f && s[n - 1] === "\\"); )
      f = s[n] === "\\" ? !f : !1, n++;
    if (s.charAt(n) == '"')
      try {
        return JSON.parse(s.substring(h, ++n - Number(f)));
      } catch (b) {
        a(String(b));
      }
    else if ($.STR & e)
      try {
        return JSON.parse(s.substring(h, n - Number(f)) + '"');
      } catch {
        return JSON.parse(s.substring(h, s.lastIndexOf("\\")) + '"');
      }
    r("Unterminated string literal");
  }, c = () => {
    n++, l();
    const h = {};
    try {
      for (; s[n] !== "}"; ) {
        if (l(), n >= t && $.OBJ & e)
          return h;
        const f = o();
        l(), n++;
        try {
          const b = i();
          Object.defineProperty(h, f, { value: b, writable: !0, enumerable: !0, configurable: !0 });
        } catch (b) {
          if ($.OBJ & e)
            return h;
          throw b;
        }
        l(), s[n] === "," && n++;
      }
    } catch {
      if ($.OBJ & e)
        return h;
      r("Expected '}' at end of object");
    }
    return n++, h;
  }, u = () => {
    n++;
    const h = [];
    try {
      for (; s[n] !== "]"; )
        h.push(i()), l(), s[n] === "," && n++;
    } catch {
      if ($.ARR & e)
        return h;
      r("Expected ']' at end of array");
    }
    return n++, h;
  }, g = () => {
    if (n === 0) {
      s === "-" && $.NUM & e && r("Not sure what '-' is");
      try {
        return JSON.parse(s);
      } catch (f) {
        if ($.NUM & e)
          try {
            return s[s.length - 1] === "." ? JSON.parse(s.substring(0, s.lastIndexOf("."))) : JSON.parse(s.substring(0, s.lastIndexOf("e")));
          } catch {
          }
        a(String(f));
      }
    }
    const h = n;
    for (s[n] === "-" && n++; s[n] && !",]}".includes(s[n]); )
      n++;
    n == t && !($.NUM & e) && r("Unterminated number literal");
    try {
      return JSON.parse(s.substring(h, n));
    } catch {
      s.substring(h, n) === "-" && $.NUM & e && r("Not sure what '-' is");
      try {
        return JSON.parse(s.substring(h, s.lastIndexOf("e")));
      } catch (b) {
        a(String(b));
      }
    }
  }, l = () => {
    for (; n < t && ` 
\r	`.includes(s[n]); )
      n++;
  };
  return i();
}, fn = (s) => Er(s, $.ALL ^ $.NUM);
var oe = function(s, e, t, n, r) {
  if (n === "m") throw new TypeError("Private method is not writable");
  if (n === "a" && !r) throw new TypeError("Private accessor was defined without a setter");
  if (typeof e == "function" ? s !== e || !r : !e.has(s)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return n === "a" ? r.call(s, t) : r ? r.value = t : e.set(s, t), t;
}, x = function(s, e, t, n) {
  if (t === "a" && !n) throw new TypeError("Private accessor was defined without a getter");
  if (typeof e == "function" ? s !== e || !n : !e.has(s)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return t === "m" ? n : t === "a" ? n.call(s) : n ? n.value : e.get(s);
}, v, ee, le, ne, ft, Be, ht, dt, mt, Le, pt, hn;
class ve extends is {
  constructor(e) {
    super(), v.add(this), ee.set(this, void 0), le.set(this, void 0), ne.set(this, void 0), oe(this, ee, e, "f"), oe(this, le, [], "f");
  }
  get currentChatCompletionSnapshot() {
    return x(this, ne, "f");
  }
  static fromReadableStream(e) {
    const t = new ve(null);
    return t._run(() => t._fromReadableStream(e)), t;
  }
  static createChatCompletion(e, t, n) {
    const r = new ve(t);
    return r._run(() => r._runChatCompletion(e, { ...t, stream: !0 }, { ...n, headers: { ...n?.headers, "X-Stainless-Helper-Method": "stream" } })), r;
  }
  async _createChatCompletion(e, t, n) {
    super._createChatCompletion;
    const r = n?.signal;
    r && (r.aborted && this.controller.abort(), r.addEventListener("abort", () => this.controller.abort())), x(this, v, "m", ft).call(this);
    const a = await e.chat.completions.create({ ...t, stream: !0 }, { ...n, signal: this.controller.signal });
    this._connected();
    for await (const i of a)
      x(this, v, "m", ht).call(this, i);
    if (a.controller.signal?.aborted)
      throw new K();
    return this._addChatCompletion(x(this, v, "m", Le).call(this));
  }
  async _fromReadableStream(e, t) {
    const n = t?.signal;
    n && (n.aborted && this.controller.abort(), n.addEventListener("abort", () => this.controller.abort())), x(this, v, "m", ft).call(this), this._connected();
    const r = Z.fromReadableStream(e, this.controller);
    let a;
    for await (const i of r)
      a && a !== i.id && this._addChatCompletion(x(this, v, "m", Le).call(this)), x(this, v, "m", ht).call(this, i), a = i.id;
    if (r.controller.signal?.aborted)
      throw new K();
    return this._addChatCompletion(x(this, v, "m", Le).call(this));
  }
  [(ee = /* @__PURE__ */ new WeakMap(), le = /* @__PURE__ */ new WeakMap(), ne = /* @__PURE__ */ new WeakMap(), v = /* @__PURE__ */ new WeakSet(), ft = function() {
    this.ended || oe(this, ne, void 0, "f");
  }, Be = function(t) {
    let n = x(this, le, "f")[t.index];
    return n || (n = {
      content_done: !1,
      refusal_done: !1,
      logprobs_content_done: !1,
      logprobs_refusal_done: !1,
      done_tool_calls: /* @__PURE__ */ new Set(),
      current_tool_call_index: null
    }, x(this, le, "f")[t.index] = n, n);
  }, ht = function(t) {
    if (this.ended)
      return;
    const n = x(this, v, "m", hn).call(this, t);
    this._emit("chunk", t, n);
    for (const r of t.choices) {
      const a = n.choices[r.index];
      r.delta.content != null && a.message?.role === "assistant" && a.message?.content && (this._emit("content", r.delta.content, a.message.content), this._emit("content.delta", {
        delta: r.delta.content,
        snapshot: a.message.content,
        parsed: a.message.parsed
      })), r.delta.refusal != null && a.message?.role === "assistant" && a.message?.refusal && this._emit("refusal.delta", {
        delta: r.delta.refusal,
        snapshot: a.message.refusal
      }), r.logprobs?.content != null && a.message?.role === "assistant" && this._emit("logprobs.content.delta", {
        content: r.logprobs?.content,
        snapshot: a.logprobs?.content ?? []
      }), r.logprobs?.refusal != null && a.message?.role === "assistant" && this._emit("logprobs.refusal.delta", {
        refusal: r.logprobs?.refusal,
        snapshot: a.logprobs?.refusal ?? []
      });
      const i = x(this, v, "m", Be).call(this, a);
      a.finish_reason && (x(this, v, "m", mt).call(this, a), i.current_tool_call_index != null && x(this, v, "m", dt).call(this, a, i.current_tool_call_index));
      for (const o of r.delta.tool_calls ?? [])
        i.current_tool_call_index !== o.index && (x(this, v, "m", mt).call(this, a), i.current_tool_call_index != null && x(this, v, "m", dt).call(this, a, i.current_tool_call_index)), i.current_tool_call_index = o.index;
      for (const o of r.delta.tool_calls ?? []) {
        const c = a.message.tool_calls?.[o.index];
        c?.type && (c?.type === "function" ? this._emit("tool_calls.function.arguments.delta", {
          name: c.function?.name,
          index: o.index,
          arguments: c.function.arguments,
          parsed_arguments: c.function.parsed_arguments,
          arguments_delta: o.function?.arguments ?? ""
        }) : (c?.type, void 0));
      }
    }
  }, dt = function(t, n) {
    if (x(this, v, "m", Be).call(this, t).done_tool_calls.has(n))
      return;
    const a = t.message.tool_calls?.[n];
    if (!a)
      throw new Error("no tool call snapshot");
    if (!a.type)
      throw new Error("tool call snapshot missing `type`");
    if (a.type === "function") {
      const i = x(this, ee, "f")?.tools?.find((o) => o.type === "function" && o.function.name === a.function.name);
      this._emit("tool_calls.function.arguments.done", {
        name: a.function.name,
        index: n,
        arguments: a.function.arguments,
        parsed_arguments: $e(i) ? i.$parseRaw(a.function.arguments) : i?.function.strict ? JSON.parse(a.function.arguments) : null
      });
    } else
      a.type;
  }, mt = function(t) {
    const n = x(this, v, "m", Be).call(this, t);
    if (t.message.content && !n.content_done) {
      n.content_done = !0;
      const r = x(this, v, "m", pt).call(this);
      this._emit("content.done", {
        content: t.message.content,
        parsed: r ? r.$parseRaw(t.message.content) : null
      });
    }
    t.message.refusal && !n.refusal_done && (n.refusal_done = !0, this._emit("refusal.done", { refusal: t.message.refusal })), t.logprobs?.content && !n.logprobs_content_done && (n.logprobs_content_done = !0, this._emit("logprobs.content.done", { content: t.logprobs.content })), t.logprobs?.refusal && !n.logprobs_refusal_done && (n.logprobs_refusal_done = !0, this._emit("logprobs.refusal.done", { refusal: t.logprobs.refusal }));
  }, Le = function() {
    if (this.ended)
      throw new _("stream has ended, this shouldn't happen");
    const t = x(this, ne, "f");
    if (!t)
      throw new _("request ended without sending any chunks");
    return oe(this, ne, void 0, "f"), oe(this, le, [], "f"), vr(t, x(this, ee, "f"));
  }, pt = function() {
    const t = x(this, ee, "f")?.response_format;
    return ss(t) ? t : null;
  }, hn = function(t) {
    var n, r, a, i;
    let o = x(this, ne, "f");
    const { choices: c, ...u } = t;
    o ? Object.assign(o, u) : o = oe(this, ne, {
      ...u,
      choices: []
    }, "f");
    for (const { delta: g, finish_reason: l, index: h, logprobs: f = null, ...b } of t.choices) {
      let m = o.choices[h];
      if (m || (m = o.choices[h] = { finish_reason: l, index: h, message: {}, logprobs: f, ...b }), f)
        if (!m.logprobs)
          m.logprobs = Object.assign({}, f);
        else {
          const { content: D, refusal: B, ...k } = f;
          Object.assign(m.logprobs, k), D && ((n = m.logprobs).content ?? (n.content = []), m.logprobs.content.push(...D)), B && ((r = m.logprobs).refusal ?? (r.refusal = []), m.logprobs.refusal.push(...B));
        }
      if (l && (m.finish_reason = l, x(this, ee, "f") && rs(x(this, ee, "f")))) {
        if (l === "length")
          throw new Bn();
        if (l === "content_filter")
          throw new Ln();
      }
      if (Object.assign(m, b), !g)
        continue;
      const { content: A, refusal: E, function_call: p, role: F, tool_calls: C, ...T } = g;
      if (Object.assign(m.message, T), E && (m.message.refusal = (m.message.refusal || "") + E), F && (m.message.role = F), p && (m.message.function_call ? (p.name && (m.message.function_call.name = p.name), p.arguments && ((a = m.message.function_call).arguments ?? (a.arguments = ""), m.message.function_call.arguments += p.arguments)) : m.message.function_call = p), A && (m.message.content = (m.message.content || "") + A, !m.message.refusal && x(this, v, "m", pt).call(this) && (m.message.parsed = fn(m.message.content))), C) {
        m.message.tool_calls || (m.message.tool_calls = []);
        for (const { index: D, id: B, type: k, function: O, ...P } of C) {
          const R = (i = m.message.tool_calls)[D] ?? (i[D] = {});
          Object.assign(R, P), B && (R.id = B), k && (R.type = k), O && (R.function ?? (R.function = { name: O.name ?? "", arguments: "" })), O?.name && (R.function.name = O.name), O?.arguments && (R.function.arguments += O.arguments, xr(x(this, ee, "f"), R) && (R.function.parsed_arguments = fn(R.function.arguments)));
        }
      }
    }
    return o;
  }, Symbol.asyncIterator)]() {
    const e = [], t = [];
    let n = !1;
    return this.on("chunk", (r) => {
      const a = t.shift();
      a ? a.resolve(r) : e.push(r);
    }), this.on("end", () => {
      n = !0;
      for (const r of t)
        r.resolve(void 0);
      t.length = 0;
    }), this.on("abort", (r) => {
      n = !0;
      for (const a of t)
        a.reject(r);
      t.length = 0;
    }), this.on("error", (r) => {
      n = !0;
      for (const a of t)
        a.reject(r);
      t.length = 0;
    }), {
      next: async () => e.length ? { value: e.shift(), done: !1 } : n ? { value: void 0, done: !0 } : new Promise((a, i) => t.push({ resolve: a, reject: i })).then((a) => a ? { value: a, done: !1 } : { value: void 0, done: !0 }),
      return: async () => (this.abort(), { value: void 0, done: !0 })
    };
  }
  toReadableStream() {
    return new Z(this[Symbol.asyncIterator].bind(this), this.controller).toReadableStream();
  }
}
function vr(s, e) {
  const { id: t, choices: n, created: r, model: a, system_fingerprint: i, ...o } = s, c = {
    ...o,
    id: t,
    choices: n.map(({ message: u, finish_reason: g, index: l, logprobs: h, ...f }) => {
      if (!g)
        throw new _(`missing finish_reason for choice ${l}`);
      const { content: b = null, function_call: m, tool_calls: A, ...E } = u, p = u.role;
      if (!p)
        throw new _(`missing role for choice ${l}`);
      if (m) {
        const { arguments: F, name: C } = m;
        if (F == null)
          throw new _(`missing function_call.arguments for choice ${l}`);
        if (!C)
          throw new _(`missing function_call.name for choice ${l}`);
        return {
          ...f,
          message: {
            content: b,
            function_call: { arguments: F, name: C },
            role: p,
            refusal: u.refusal ?? null
          },
          finish_reason: g,
          index: l,
          logprobs: h
        };
      }
      return A ? {
        ...f,
        index: l,
        finish_reason: g,
        logprobs: h,
        message: {
          ...E,
          role: p,
          content: b,
          refusal: u.refusal ?? null,
          tool_calls: A.map((F, C) => {
            const { function: T, type: D, id: B, ...k } = F, { arguments: O, name: P, ...R } = T || {};
            if (B == null)
              throw new _(`missing choices[${l}].tool_calls[${C}].id
${Ue(s)}`);
            if (D == null)
              throw new _(`missing choices[${l}].tool_calls[${C}].type
${Ue(s)}`);
            if (P == null)
              throw new _(`missing choices[${l}].tool_calls[${C}].function.name
${Ue(s)}`);
            if (O == null)
              throw new _(`missing choices[${l}].tool_calls[${C}].function.arguments
${Ue(s)}`);
            return { ...k, id: B, type: D, function: { ...R, name: P, arguments: O } };
          })
        }
      } : {
        ...f,
        message: { ...E, content: b, role: p, refusal: u.refusal ?? null },
        finish_reason: g,
        index: l,
        logprobs: h
      };
    }),
    created: r,
    model: a,
    object: "chat.completion",
    ...i ? { system_fingerprint: i } : {}
  };
  return wr(c, e);
}
function Ue(s) {
  return JSON.stringify(s);
}
class me extends ve {
  static fromReadableStream(e) {
    const t = new me(null);
    return t._run(() => t._fromReadableStream(e)), t;
  }
  static runFunctions(e, t, n) {
    const r = new me(null), a = {
      ...n,
      headers: { ...n?.headers, "X-Stainless-Helper-Method": "runFunctions" }
    };
    return r._run(() => r._runFunctions(e, t, a)), r;
  }
  static runTools(e, t, n) {
    const r = new me(
      t
    ), a = {
      ...n,
      headers: { ...n?.headers, "X-Stainless-Helper-Method": "runTools" }
    };
    return r._run(() => r._runTools(e, t, a)), r;
  }
}
let bs = class extends y {
  parse(e, t) {
    return Sr(e.tools), this._client.chat.completions.create(e, {
      ...t,
      headers: {
        ...t?.headers,
        "X-Stainless-Helper-Method": "beta.chat.completions.parse"
      }
    })._thenUnwrap((n) => kt(n, e));
  }
  runFunctions(e, t) {
    return e.stream ? me.runFunctions(this._client, e, t) : Re.runFunctions(this._client, e, t);
  }
  runTools(e, t) {
    return e.stream ? me.runTools(this._client, e, t) : Re.runTools(this._client, e, t);
  }
  stream(e, t) {
    return ve.createChatCompletion(this._client, e, t);
  }
};
class Rt extends y {
  constructor() {
    super(...arguments), this.completions = new bs(this._client);
  }
}
(function(s) {
  s.Completions = bs;
})(Rt || (Rt = {}));
class xs extends y {
  create(e, t) {
    return this._client.post("/realtime/sessions", {
      body: e,
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
}
class Mt extends y {
  constructor() {
    super(...arguments), this.sessions = new xs(this._client);
  }
}
Mt.Sessions = xs;
var d = function(s, e, t, n) {
  if (t === "a" && !n) throw new TypeError("Private accessor was defined without a getter");
  if (typeof e == "function" ? s !== e || !n : !e.has(s)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return t === "m" ? n : t === "a" ? n.call(s) : n ? n.value : e.get(s);
}, W = function(s, e, t, n, r) {
  if (n === "m") throw new TypeError("Private method is not writable");
  if (n === "a" && !r) throw new TypeError("Private accessor was defined without a setter");
  if (typeof e == "function" ? s !== e || !r : !e.has(s)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return n === "a" ? r.call(s, t) : r ? r.value = t : e.set(s, t), t;
}, M, vt, Y, He, V, ie, fe, ae, Qe, H, Xe, qe, Ee, Ce, Ae, dn, mn, pn, gn, _n, wn, yn;
class G extends ns {
  constructor() {
    super(...arguments), M.add(this), vt.set(this, []), Y.set(this, {}), He.set(this, {}), V.set(this, void 0), ie.set(this, void 0), fe.set(this, void 0), ae.set(this, void 0), Qe.set(this, void 0), H.set(this, void 0), Xe.set(this, void 0), qe.set(this, void 0), Ee.set(this, void 0);
  }
  [(vt = /* @__PURE__ */ new WeakMap(), Y = /* @__PURE__ */ new WeakMap(), He = /* @__PURE__ */ new WeakMap(), V = /* @__PURE__ */ new WeakMap(), ie = /* @__PURE__ */ new WeakMap(), fe = /* @__PURE__ */ new WeakMap(), ae = /* @__PURE__ */ new WeakMap(), Qe = /* @__PURE__ */ new WeakMap(), H = /* @__PURE__ */ new WeakMap(), Xe = /* @__PURE__ */ new WeakMap(), qe = /* @__PURE__ */ new WeakMap(), Ee = /* @__PURE__ */ new WeakMap(), M = /* @__PURE__ */ new WeakSet(), Symbol.asyncIterator)]() {
    const e = [], t = [];
    let n = !1;
    return this.on("event", (r) => {
      const a = t.shift();
      a ? a.resolve(r) : e.push(r);
    }), this.on("end", () => {
      n = !0;
      for (const r of t)
        r.resolve(void 0);
      t.length = 0;
    }), this.on("abort", (r) => {
      n = !0;
      for (const a of t)
        a.reject(r);
      t.length = 0;
    }), this.on("error", (r) => {
      n = !0;
      for (const a of t)
        a.reject(r);
      t.length = 0;
    }), {
      next: async () => e.length ? { value: e.shift(), done: !1 } : n ? { value: void 0, done: !0 } : new Promise((a, i) => t.push({ resolve: a, reject: i })).then((a) => a ? { value: a, done: !1 } : { value: void 0, done: !0 }),
      return: async () => (this.abort(), { value: void 0, done: !0 })
    };
  }
  static fromReadableStream(e) {
    const t = new G();
    return t._run(() => t._fromReadableStream(e)), t;
  }
  async _fromReadableStream(e, t) {
    const n = t?.signal;
    n && (n.aborted && this.controller.abort(), n.addEventListener("abort", () => this.controller.abort())), this._connected();
    const r = Z.fromReadableStream(e, this.controller);
    for await (const a of r)
      d(this, M, "m", Ce).call(this, a);
    if (r.controller.signal?.aborted)
      throw new K();
    return this._addRun(d(this, M, "m", Ae).call(this));
  }
  toReadableStream() {
    return new Z(this[Symbol.asyncIterator].bind(this), this.controller).toReadableStream();
  }
  static createToolAssistantStream(e, t, n, r, a) {
    const i = new G();
    return i._run(() => i._runToolAssistantStream(e, t, n, r, {
      ...a,
      headers: { ...a?.headers, "X-Stainless-Helper-Method": "stream" }
    })), i;
  }
  async _createToolAssistantStream(e, t, n, r, a) {
    const i = a?.signal;
    i && (i.aborted && this.controller.abort(), i.addEventListener("abort", () => this.controller.abort()));
    const o = { ...r, stream: !0 }, c = await e.submitToolOutputs(t, n, o, {
      ...a,
      signal: this.controller.signal
    });
    this._connected();
    for await (const u of c)
      d(this, M, "m", Ce).call(this, u);
    if (c.controller.signal?.aborted)
      throw new K();
    return this._addRun(d(this, M, "m", Ae).call(this));
  }
  static createThreadAssistantStream(e, t, n) {
    const r = new G();
    return r._run(() => r._threadAssistantStream(e, t, {
      ...n,
      headers: { ...n?.headers, "X-Stainless-Helper-Method": "stream" }
    })), r;
  }
  static createAssistantStream(e, t, n, r) {
    const a = new G();
    return a._run(() => a._runAssistantStream(e, t, n, {
      ...r,
      headers: { ...r?.headers, "X-Stainless-Helper-Method": "stream" }
    })), a;
  }
  currentEvent() {
    return d(this, Xe, "f");
  }
  currentRun() {
    return d(this, qe, "f");
  }
  currentMessageSnapshot() {
    return d(this, V, "f");
  }
  currentRunStepSnapshot() {
    return d(this, Ee, "f");
  }
  async finalRunSteps() {
    return await this.done(), Object.values(d(this, Y, "f"));
  }
  async finalMessages() {
    return await this.done(), Object.values(d(this, He, "f"));
  }
  async finalRun() {
    if (await this.done(), !d(this, ie, "f"))
      throw Error("Final run was not received.");
    return d(this, ie, "f");
  }
  async _createThreadAssistantStream(e, t, n) {
    const r = n?.signal;
    r && (r.aborted && this.controller.abort(), r.addEventListener("abort", () => this.controller.abort()));
    const a = { ...t, stream: !0 }, i = await e.createAndRun(a, { ...n, signal: this.controller.signal });
    this._connected();
    for await (const o of i)
      d(this, M, "m", Ce).call(this, o);
    if (i.controller.signal?.aborted)
      throw new K();
    return this._addRun(d(this, M, "m", Ae).call(this));
  }
  async _createAssistantStream(e, t, n, r) {
    const a = r?.signal;
    a && (a.aborted && this.controller.abort(), a.addEventListener("abort", () => this.controller.abort()));
    const i = { ...n, stream: !0 }, o = await e.create(t, i, { ...r, signal: this.controller.signal });
    this._connected();
    for await (const c of o)
      d(this, M, "m", Ce).call(this, c);
    if (o.controller.signal?.aborted)
      throw new K();
    return this._addRun(d(this, M, "m", Ae).call(this));
  }
  static accumulateDelta(e, t) {
    for (const [n, r] of Object.entries(t)) {
      if (!e.hasOwnProperty(n)) {
        e[n] = r;
        continue;
      }
      let a = e[n];
      if (a == null) {
        e[n] = r;
        continue;
      }
      if (n === "index" || n === "type") {
        e[n] = r;
        continue;
      }
      if (typeof a == "string" && typeof r == "string")
        a += r;
      else if (typeof a == "number" && typeof r == "number")
        a += r;
      else if (ut(a) && ut(r))
        a = this.accumulateDelta(a, r);
      else if (Array.isArray(a) && Array.isArray(r)) {
        if (a.every((i) => typeof i == "string" || typeof i == "number")) {
          a.push(...r);
          continue;
        }
        for (const i of r) {
          if (!ut(i))
            throw new Error(`Expected array delta entry to be an object but got: ${i}`);
          const o = i.index;
          if (o == null)
            throw console.error(i), new Error("Expected array delta entry to have an `index` property");
          if (typeof o != "number")
            throw new Error(`Expected array delta entry \`index\` property to be a number but got ${o}`);
          const c = a[o];
          c == null ? a.push(i) : a[o] = this.accumulateDelta(c, i);
        }
        continue;
      } else
        throw Error(`Unhandled record type: ${n}, deltaValue: ${r}, accValue: ${a}`);
      e[n] = a;
    }
    return e;
  }
  _addRun(e) {
    return e;
  }
  async _threadAssistantStream(e, t, n) {
    return await this._createThreadAssistantStream(t, e, n);
  }
  async _runAssistantStream(e, t, n, r) {
    return await this._createAssistantStream(t, e, n, r);
  }
  async _runToolAssistantStream(e, t, n, r, a) {
    return await this._createToolAssistantStream(n, e, t, r, a);
  }
}
Ce = function(e) {
  if (!this.ended)
    switch (W(this, Xe, e, "f"), d(this, M, "m", pn).call(this, e), e.event) {
      case "thread.created":
        break;
      case "thread.run.created":
      case "thread.run.queued":
      case "thread.run.in_progress":
      case "thread.run.requires_action":
      case "thread.run.completed":
      case "thread.run.incomplete":
      case "thread.run.failed":
      case "thread.run.cancelling":
      case "thread.run.cancelled":
      case "thread.run.expired":
        d(this, M, "m", yn).call(this, e);
        break;
      case "thread.run.step.created":
      case "thread.run.step.in_progress":
      case "thread.run.step.delta":
      case "thread.run.step.completed":
      case "thread.run.step.failed":
      case "thread.run.step.cancelled":
      case "thread.run.step.expired":
        d(this, M, "m", mn).call(this, e);
        break;
      case "thread.message.created":
      case "thread.message.in_progress":
      case "thread.message.delta":
      case "thread.message.completed":
      case "thread.message.incomplete":
        d(this, M, "m", dn).call(this, e);
        break;
      case "error":
        throw new Error("Encountered an error event in event processing - errors should be processed earlier");
    }
}, Ae = function() {
  if (this.ended)
    throw new _("stream has ended, this shouldn't happen");
  if (!d(this, ie, "f"))
    throw Error("Final run has not been received");
  return d(this, ie, "f");
}, dn = function(e) {
  const [t, n] = d(this, M, "m", _n).call(this, e, d(this, V, "f"));
  W(this, V, t, "f"), d(this, He, "f")[t.id] = t;
  for (const r of n) {
    const a = t.content[r.index];
    a?.type == "text" && this._emit("textCreated", a.text);
  }
  switch (e.event) {
    case "thread.message.created":
      this._emit("messageCreated", e.data);
      break;
    case "thread.message.in_progress":
      break;
    case "thread.message.delta":
      if (this._emit("messageDelta", e.data.delta, t), e.data.delta.content)
        for (const r of e.data.delta.content) {
          if (r.type == "text" && r.text) {
            let a = r.text, i = t.content[r.index];
            if (i && i.type == "text")
              this._emit("textDelta", a, i.text);
            else
              throw Error("The snapshot associated with this text delta is not text or missing");
          }
          if (r.index != d(this, fe, "f")) {
            if (d(this, ae, "f"))
              switch (d(this, ae, "f").type) {
                case "text":
                  this._emit("textDone", d(this, ae, "f").text, d(this, V, "f"));
                  break;
                case "image_file":
                  this._emit("imageFileDone", d(this, ae, "f").image_file, d(this, V, "f"));
                  break;
              }
            W(this, fe, r.index, "f");
          }
          W(this, ae, t.content[r.index], "f");
        }
      break;
    case "thread.message.completed":
    case "thread.message.incomplete":
      if (d(this, fe, "f") !== void 0) {
        const r = e.data.content[d(this, fe, "f")];
        if (r)
          switch (r.type) {
            case "image_file":
              this._emit("imageFileDone", r.image_file, d(this, V, "f"));
              break;
            case "text":
              this._emit("textDone", r.text, d(this, V, "f"));
              break;
          }
      }
      d(this, V, "f") && this._emit("messageDone", e.data), W(this, V, void 0, "f");
  }
}, mn = function(e) {
  const t = d(this, M, "m", gn).call(this, e);
  switch (W(this, Ee, t, "f"), e.event) {
    case "thread.run.step.created":
      this._emit("runStepCreated", e.data);
      break;
    case "thread.run.step.delta":
      const n = e.data.delta;
      if (n.step_details && n.step_details.type == "tool_calls" && n.step_details.tool_calls && t.step_details.type == "tool_calls")
        for (const a of n.step_details.tool_calls)
          a.index == d(this, Qe, "f") ? this._emit("toolCallDelta", a, t.step_details.tool_calls[a.index]) : (d(this, H, "f") && this._emit("toolCallDone", d(this, H, "f")), W(this, Qe, a.index, "f"), W(this, H, t.step_details.tool_calls[a.index], "f"), d(this, H, "f") && this._emit("toolCallCreated", d(this, H, "f")));
      this._emit("runStepDelta", e.data.delta, t);
      break;
    case "thread.run.step.completed":
    case "thread.run.step.failed":
    case "thread.run.step.cancelled":
    case "thread.run.step.expired":
      W(this, Ee, void 0, "f"), e.data.step_details.type == "tool_calls" && d(this, H, "f") && (this._emit("toolCallDone", d(this, H, "f")), W(this, H, void 0, "f")), this._emit("runStepDone", e.data, t);
      break;
  }
}, pn = function(e) {
  d(this, vt, "f").push(e), this._emit("event", e);
}, gn = function(e) {
  switch (e.event) {
    case "thread.run.step.created":
      return d(this, Y, "f")[e.data.id] = e.data, e.data;
    case "thread.run.step.delta":
      let t = d(this, Y, "f")[e.data.id];
      if (!t)
        throw Error("Received a RunStepDelta before creation of a snapshot");
      let n = e.data;
      if (n.delta) {
        const r = G.accumulateDelta(t, n.delta);
        d(this, Y, "f")[e.data.id] = r;
      }
      return d(this, Y, "f")[e.data.id];
    case "thread.run.step.completed":
    case "thread.run.step.failed":
    case "thread.run.step.cancelled":
    case "thread.run.step.expired":
    case "thread.run.step.in_progress":
      d(this, Y, "f")[e.data.id] = e.data;
      break;
  }
  if (d(this, Y, "f")[e.data.id])
    return d(this, Y, "f")[e.data.id];
  throw new Error("No snapshot available");
}, _n = function(e, t) {
  let n = [];
  switch (e.event) {
    case "thread.message.created":
      return [e.data, n];
    case "thread.message.delta":
      if (!t)
        throw Error("Received a delta with no existing snapshot (there should be one from message creation)");
      let r = e.data;
      if (r.delta.content)
        for (const a of r.delta.content)
          if (a.index in t.content) {
            let i = t.content[a.index];
            t.content[a.index] = d(this, M, "m", wn).call(this, a, i);
          } else
            t.content[a.index] = a, n.push(a);
      return [t, n];
    case "thread.message.in_progress":
    case "thread.message.completed":
    case "thread.message.incomplete":
      if (t)
        return [t, n];
      throw Error("Received thread message event with no existing snapshot");
  }
  throw Error("Tried to accumulate a non-message event");
}, wn = function(e, t) {
  return G.accumulateDelta(t, e);
}, yn = function(e) {
  switch (W(this, qe, e.data, "f"), e.event) {
    case "thread.run.created":
      break;
    case "thread.run.queued":
      break;
    case "thread.run.in_progress":
      break;
    case "thread.run.requires_action":
    case "thread.run.cancelled":
    case "thread.run.failed":
    case "thread.run.completed":
    case "thread.run.expired":
      W(this, ie, e.data, "f"), d(this, H, "f") && (this._emit("toolCallDone", d(this, H, "f")), W(this, H, void 0, "f"));
      break;
  }
};
class Nt extends y {
  create(e, t, n) {
    return this._client.post(`/threads/${e}/messages`, {
      body: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  retrieve(e, t, n) {
    return this._client.get(`/threads/${e}/messages/${t}`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  update(e, t, n, r) {
    return this._client.post(`/threads/${e}/messages/${t}`, {
      body: n,
      ...r,
      headers: { "OpenAI-Beta": "assistants=v2", ...r?.headers }
    });
  }
  list(e, t = {}, n) {
    return U(t) ? this.list(e, {}, t) : this._client.getAPIList(`/threads/${e}/messages`, Dt, {
      query: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  del(e, t, n) {
    return this._client.delete(`/threads/${e}/messages/${t}`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
}
class Dt extends X {
}
Nt.MessagesPage = Dt;
class Bt extends y {
  retrieve(e, t, n, r = {}, a) {
    return U(r) ? this.retrieve(e, t, n, {}, r) : this._client.get(`/threads/${e}/runs/${t}/steps/${n}`, {
      query: r,
      ...a,
      headers: { "OpenAI-Beta": "assistants=v2", ...a?.headers }
    });
  }
  list(e, t, n = {}, r) {
    return U(n) ? this.list(e, t, {}, n) : this._client.getAPIList(`/threads/${e}/runs/${t}/steps`, Lt, {
      query: n,
      ...r,
      headers: { "OpenAI-Beta": "assistants=v2", ...r?.headers }
    });
  }
}
class Lt extends X {
}
Bt.RunStepsPage = Lt;
class Fe extends y {
  constructor() {
    super(...arguments), this.steps = new Bt(this._client);
  }
  create(e, t, n) {
    const { include: r, ...a } = t;
    return this._client.post(`/threads/${e}/runs`, {
      query: { include: r },
      body: a,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers },
      stream: t.stream ?? !1
    });
  }
  retrieve(e, t, n) {
    return this._client.get(`/threads/${e}/runs/${t}`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  update(e, t, n, r) {
    return this._client.post(`/threads/${e}/runs/${t}`, {
      body: n,
      ...r,
      headers: { "OpenAI-Beta": "assistants=v2", ...r?.headers }
    });
  }
  list(e, t = {}, n) {
    return U(t) ? this.list(e, {}, t) : this._client.getAPIList(`/threads/${e}/runs`, Ut, {
      query: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  cancel(e, t, n) {
    return this._client.post(`/threads/${e}/runs/${t}/cancel`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  async createAndPoll(e, t, n) {
    const r = await this.create(e, t, n);
    return await this.poll(e, r.id, n);
  }
  createAndStream(e, t, n) {
    return G.createAssistantStream(e, this._client.beta.threads.runs, t, n);
  }
  async poll(e, t, n) {
    const r = { ...n?.headers, "X-Stainless-Poll-Helper": "true" };
    for (n?.pollIntervalMs && (r["X-Stainless-Custom-Poll-Interval"] = n.pollIntervalMs.toString()); ; ) {
      const { data: a, response: i } = await this.retrieve(e, t, {
        ...n,
        headers: { ...n?.headers, ...r }
      }).withResponse();
      switch (a.status) {
        case "queued":
        case "in_progress":
        case "cancelling":
          let o = 5e3;
          if (n?.pollIntervalMs)
            o = n.pollIntervalMs;
          else {
            const c = i.headers.get("openai-poll-after-ms");
            if (c) {
              const u = parseInt(c);
              isNaN(u) || (o = u);
            }
          }
          await Ie(o);
          break;
        case "requires_action":
        case "incomplete":
        case "cancelled":
        case "completed":
        case "failed":
        case "expired":
          return a;
      }
    }
  }
  stream(e, t, n) {
    return G.createAssistantStream(e, this._client.beta.threads.runs, t, n);
  }
  submitToolOutputs(e, t, n, r) {
    return this._client.post(`/threads/${e}/runs/${t}/submit_tool_outputs`, {
      body: n,
      ...r,
      headers: { "OpenAI-Beta": "assistants=v2", ...r?.headers },
      stream: n.stream ?? !1
    });
  }
  async submitToolOutputsAndPoll(e, t, n, r) {
    const a = await this.submitToolOutputs(e, t, n, r);
    return await this.poll(e, a.id, r);
  }
  submitToolOutputsStream(e, t, n, r) {
    return G.createToolAssistantStream(e, t, this._client.beta.threads.runs, n, r);
  }
}
class Ut extends X {
}
Fe.RunsPage = Ut;
Fe.Steps = Bt;
Fe.RunStepsPage = Lt;
class ge extends y {
  constructor() {
    super(...arguments), this.runs = new Fe(this._client), this.messages = new Nt(this._client);
  }
  create(e = {}, t) {
    return U(e) ? this.create({}, e) : this._client.post("/threads", {
      body: e,
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  retrieve(e, t) {
    return this._client.get(`/threads/${e}`, {
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  update(e, t, n) {
    return this._client.post(`/threads/${e}`, {
      body: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  del(e, t) {
    return this._client.delete(`/threads/${e}`, {
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  createAndRun(e, t) {
    return this._client.post("/threads/runs", {
      body: e,
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers },
      stream: e.stream ?? !1
    });
  }
  async createAndRunPoll(e, t) {
    const n = await this.createAndRun(e, t);
    return await this.runs.poll(n.thread_id, n.id, t);
  }
  createAndRunStream(e, t) {
    return G.createThreadAssistantStream(e, this._client.beta.threads, t);
  }
}
ge.Runs = Fe;
ge.RunsPage = Ut;
ge.Messages = Nt;
ge.MessagesPage = Dt;
const Ir = async (s) => {
  const e = await Promise.allSettled(s), t = e.filter((r) => r.status === "rejected");
  if (t.length) {
    for (const r of t)
      console.error(r.reason);
    throw new Error(`${t.length} promise(s) failed - see the above errors`);
  }
  const n = [];
  for (const r of e)
    r.status === "fulfilled" && n.push(r.value);
  return n;
};
let jt = class extends y {
  create(e, t, n) {
    return this._client.post(`/vector_stores/${e}/files`, {
      body: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  retrieve(e, t, n) {
    return this._client.get(`/vector_stores/${e}/files/${t}`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  list(e, t = {}, n) {
    return U(t) ? this.list(e, {}, t) : this._client.getAPIList(`/vector_stores/${e}/files`, rt, {
      query: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  del(e, t, n) {
    return this._client.delete(`/vector_stores/${e}/files/${t}`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  async createAndPoll(e, t, n) {
    const r = await this.create(e, t, n);
    return await this.poll(e, r.id, n);
  }
  async poll(e, t, n) {
    const r = { ...n?.headers, "X-Stainless-Poll-Helper": "true" };
    for (n?.pollIntervalMs && (r["X-Stainless-Custom-Poll-Interval"] = n.pollIntervalMs.toString()); ; ) {
      const a = await this.retrieve(e, t, {
        ...n,
        headers: r
      }).withResponse(), i = a.data;
      switch (i.status) {
        case "in_progress":
          let o = 5e3;
          if (n?.pollIntervalMs)
            o = n.pollIntervalMs;
          else {
            const c = a.response.headers.get("openai-poll-after-ms");
            if (c) {
              const u = parseInt(c);
              isNaN(u) || (o = u);
            }
          }
          await Ie(o);
          break;
        case "failed":
        case "completed":
          return i;
      }
    }
  }
  async upload(e, t, n) {
    const r = await this._client.files.create({ file: t, purpose: "assistants" }, n);
    return this.create(e, { file_id: r.id }, n);
  }
  async uploadAndPoll(e, t, n) {
    const r = await this.upload(e, t, n);
    return await this.poll(e, r.id, n);
  }
};
class rt extends X {
}
jt.VectorStoreFilesPage = rt;
class Ss extends y {
  create(e, t, n) {
    return this._client.post(`/vector_stores/${e}/file_batches`, {
      body: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  retrieve(e, t, n) {
    return this._client.get(`/vector_stores/${e}/file_batches/${t}`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  cancel(e, t, n) {
    return this._client.post(`/vector_stores/${e}/file_batches/${t}/cancel`, {
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  async createAndPoll(e, t, n) {
    const r = await this.create(e, t);
    return await this.poll(e, r.id, n);
  }
  listFiles(e, t, n = {}, r) {
    return U(n) ? this.listFiles(e, t, {}, n) : this._client.getAPIList(`/vector_stores/${e}/file_batches/${t}/files`, rt, { query: n, ...r, headers: { "OpenAI-Beta": "assistants=v2", ...r?.headers } });
  }
  async poll(e, t, n) {
    const r = { ...n?.headers, "X-Stainless-Poll-Helper": "true" };
    for (n?.pollIntervalMs && (r["X-Stainless-Custom-Poll-Interval"] = n.pollIntervalMs.toString()); ; ) {
      const { data: a, response: i } = await this.retrieve(e, t, {
        ...n,
        headers: r
      }).withResponse();
      switch (a.status) {
        case "in_progress":
          let o = 5e3;
          if (n?.pollIntervalMs)
            o = n.pollIntervalMs;
          else {
            const c = i.headers.get("openai-poll-after-ms");
            if (c) {
              const u = parseInt(c);
              isNaN(u) || (o = u);
            }
          }
          await Ie(o);
          break;
        case "failed":
        case "cancelled":
        case "completed":
          return a;
      }
    }
  }
  async uploadAndPoll(e, { files: t, fileIds: n = [] }, r) {
    if (t == null || t.length == 0)
      throw new Error("No `files` provided to process. If you've already uploaded files you should use `.createAndPoll()` instead");
    const a = r?.maxConcurrency ?? 5, i = Math.min(a, t.length), o = this._client, c = t.values(), u = [...n];
    async function g(h) {
      for (let f of h) {
        const b = await o.files.create({ file: f, purpose: "assistants" }, r);
        u.push(b.id);
      }
    }
    const l = Array(i).fill(c).map(g);
    return await Ir(l), await this.createAndPoll(e, {
      file_ids: u
    });
  }
}
class _e extends y {
  constructor() {
    super(...arguments), this.files = new jt(this._client), this.fileBatches = new Ss(this._client);
  }
  create(e, t) {
    return this._client.post("/vector_stores", {
      body: e,
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  retrieve(e, t) {
    return this._client.get(`/vector_stores/${e}`, {
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  update(e, t, n) {
    return this._client.post(`/vector_stores/${e}`, {
      body: t,
      ...n,
      headers: { "OpenAI-Beta": "assistants=v2", ...n?.headers }
    });
  }
  list(e = {}, t) {
    return U(e) ? this.list({}, e) : this._client.getAPIList("/vector_stores", Wt, {
      query: e,
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
  del(e, t) {
    return this._client.delete(`/vector_stores/${e}`, {
      ...t,
      headers: { "OpenAI-Beta": "assistants=v2", ...t?.headers }
    });
  }
}
class Wt extends X {
}
_e.VectorStoresPage = Wt;
_e.Files = jt;
_e.VectorStoreFilesPage = rt;
_e.FileBatches = Ss;
class se extends y {
  constructor() {
    super(...arguments), this.realtime = new Mt(this._client), this.vectorStores = new _e(this._client), this.chat = new Rt(this._client), this.assistants = new Ft(this._client), this.threads = new ge(this._client);
  }
}
se.Realtime = Mt;
se.VectorStores = _e;
se.VectorStoresPage = Wt;
se.Assistants = Ft;
se.AssistantsPage = Tt;
se.Threads = ge;
class Cs extends y {
  create(e, t) {
    return this._client.post("/completions", { body: e, ...t, stream: e.stream ?? !1 });
  }
}
class As extends y {
  create(e, t) {
    return this._client.post("/embeddings", { body: e, ...t });
  }
}
class Jt extends y {
  create(e, t) {
    return this._client.post("/files", pe({ body: e, ...t }));
  }
  retrieve(e, t) {
    return this._client.get(`/files/${e}`, t);
  }
  list(e = {}, t) {
    return U(e) ? this.list({}, e) : this._client.getAPIList("/files", Ht, { query: e, ...t });
  }
  del(e, t) {
    return this._client.delete(`/files/${e}`, t);
  }
  content(e, t) {
    return this._client.get(`/files/${e}/content`, {
      ...t,
      headers: { Accept: "application/binary", ...t?.headers },
      __binaryResponse: !0
    });
  }
  retrieveContent(e, t) {
    return this._client.get(`/files/${e}/content`, t);
  }
  async waitForProcessing(e, { pollInterval: t = 5e3, maxWait: n = 30 * 60 * 1e3 } = {}) {
    const r = /* @__PURE__ */ new Set(["processed", "error", "deleted"]), a = Date.now();
    let i = await this.retrieve(e);
    for (; !i.status || !r.has(i.status); )
      if (await Ie(t), i = await this.retrieve(e), Date.now() - a > n)
        throw new It({
          message: `Giving up on waiting for file ${e} to finish processing after ${n} milliseconds.`
        });
    return i;
  }
}
class Ht extends X {
}
Jt.FileObjectsPage = Ht;
class Xt extends y {
  list(e, t = {}, n) {
    return U(t) ? this.list(e, {}, t) : this._client.getAPIList(`/fine_tuning/jobs/${e}/checkpoints`, qt, { query: t, ...n });
  }
}
class qt extends X {
}
Xt.FineTuningJobCheckpointsPage = qt;
class we extends y {
  constructor() {
    super(...arguments), this.checkpoints = new Xt(this._client);
  }
  create(e, t) {
    return this._client.post("/fine_tuning/jobs", { body: e, ...t });
  }
  retrieve(e, t) {
    return this._client.get(`/fine_tuning/jobs/${e}`, t);
  }
  list(e = {}, t) {
    return U(e) ? this.list({}, e) : this._client.getAPIList("/fine_tuning/jobs", Vt, { query: e, ...t });
  }
  cancel(e, t) {
    return this._client.post(`/fine_tuning/jobs/${e}/cancel`, t);
  }
  listEvents(e, t = {}, n) {
    return U(t) ? this.listEvents(e, {}, t) : this._client.getAPIList(`/fine_tuning/jobs/${e}/events`, Kt, {
      query: t,
      ...n
    });
  }
}
class Vt extends X {
}
class Kt extends X {
}
we.FineTuningJobsPage = Vt;
we.FineTuningJobEventsPage = Kt;
we.Checkpoints = Xt;
we.FineTuningJobCheckpointsPage = qt;
class Te extends y {
  constructor() {
    super(...arguments), this.jobs = new we(this._client);
  }
}
Te.Jobs = we;
Te.FineTuningJobsPage = Vt;
Te.FineTuningJobEventsPage = Kt;
class Ps extends y {
  createVariation(e, t) {
    return this._client.post("/images/variations", pe({ body: e, ...t }));
  }
  edit(e, t) {
    return this._client.post("/images/edits", pe({ body: e, ...t }));
  }
  generate(e, t) {
    return this._client.post("/images/generations", { body: e, ...t });
  }
}
class Gt extends y {
  retrieve(e, t) {
    return this._client.get(`/models/${e}`, t);
  }
  list(e) {
    return this._client.getAPIList("/models", Qt, e);
  }
  del(e, t) {
    return this._client.delete(`/models/${e}`, t);
  }
}
class Qt extends gr {
}
Gt.ModelsPage = Qt;
class Es extends y {
  create(e, t) {
    return this._client.post("/moderations", { body: e, ...t });
  }
}
class Rs extends y {
  create(e, t, n) {
    return this._client.post(`/uploads/${e}/parts`, pe({ body: t, ...n }));
  }
}
class zt extends y {
  constructor() {
    super(...arguments), this.parts = new Rs(this._client);
  }
  create(e, t) {
    return this._client.post("/uploads", { body: e, ...t });
  }
  cancel(e, t) {
    return this._client.post(`/uploads/${e}/cancel`, t);
  }
  complete(e, t, n) {
    return this._client.post(`/uploads/${e}/complete`, { body: t, ...n });
  }
}
zt.Parts = Rs;
var vs;
class w extends sr {
  constructor({ baseURL: e = Ne("OPENAI_BASE_URL"), apiKey: t = Ne("OPENAI_API_KEY"), organization: n = Ne("OPENAI_ORG_ID") ?? null, project: r = Ne("OPENAI_PROJECT_ID") ?? null, ...a } = {}) {
    if (t === void 0)
      throw new _("The OPENAI_API_KEY environment variable is missing or empty; either provide it, or instantiate the OpenAI client with an apiKey option, like new OpenAI({ apiKey: 'My API Key' }).");
    const i = {
      apiKey: t,
      organization: n,
      project: r,
      ...a,
      baseURL: e || "https://api.openai.com/v1"
    };
    if (!i.dangerouslyAllowBrowser && mr())
      throw new _(`It looks like you're running in a browser-like environment.

This is disabled by default, as it risks exposing your secret API credentials to attackers.
If you understand the risks and have appropriate mitigations in place,
you can set the \`dangerouslyAllowBrowser\` option to \`true\`, e.g.,

new OpenAI({ apiKey, dangerouslyAllowBrowser: true });

https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety
`);
    super({
      baseURL: i.baseURL,
      timeout: i.timeout ?? 6e5,
      httpAgent: i.httpAgent,
      maxRetries: i.maxRetries,
      fetch: i.fetch
    }), this.completions = new Cs(this), this.chat = new st(this), this.embeddings = new As(this), this.files = new Jt(this), this.images = new Ps(this), this.audio = new Oe(this), this.moderations = new Es(this), this.models = new Gt(this), this.fineTuning = new Te(this), this.beta = new se(this), this.batches = new Ot(this), this.uploads = new zt(this), this._options = i, this.apiKey = t, this.organization = n, this.project = r;
  }
  defaultQuery() {
    return this._options.defaultQuery;
  }
  defaultHeaders(e) {
    return {
      ...super.defaultHeaders(e),
      "OpenAI-Organization": this.organization,
      "OpenAI-Project": this.project,
      ...this._options.defaultHeaders
    };
  }
  authHeaders(e) {
    return { Authorization: `Bearer ${this.apiKey}` };
  }
  stringifyQuery(e) {
    return Ls(e, { arrayFormat: "brackets" });
  }
}
vs = w;
w.OpenAI = vs;
w.DEFAULT_TIMEOUT = 6e5;
w.OpenAIError = _;
w.APIError = N;
w.APIConnectionError = ze;
w.APIConnectionTimeoutError = It;
w.APIUserAbortError = K;
w.NotFoundError = Tn;
w.ConflictError = kn;
w.RateLimitError = Nn;
w.BadRequestError = On;
w.AuthenticationError = $n;
w.InternalServerError = Dn;
w.PermissionDeniedError = Fn;
w.UnprocessableEntityError = Mn;
w.toFile = Jn;
w.fileFromPath = vn;
w.Completions = Cs;
w.Chat = st;
w.ChatCompletionsPage = nt;
w.Embeddings = As;
w.Files = Jt;
w.FileObjectsPage = Ht;
w.Images = Ps;
w.Audio = Oe;
w.Moderations = Es;
w.Models = Gt;
w.ModelsPage = Qt;
w.FineTuning = Te;
w.Beta = se;
w.Batches = Ot;
w.BatchesPage = $t;
w.Uploads = zt;
async function Or(s, e, t) {
  const r = await new w({
    apiKey: s
  }).chat.completions.create({
    messages: [{ role: "user", content: t }],
    model: e,
    stream: !0
  }), a = [];
  for await (const i of r)
    a.push(i.choices[0]?.delta?.content || "");
  return a.join("");
}
const Ur = { getResponse: Or };
export {
  Ur as default
};
