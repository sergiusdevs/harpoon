// Preludio de Harpoon: define `harpoon`, `console`, `test` y `expect`.
// Todo el intercambio con Rust va como JSON (harpoon.__init / harpoon.__state).
"use strict";

var __h = {
  req: null,
  res: null,
  vars: {},
  env: {},
  tests: [],
  logs: [],
};

function __str(v) {
  if (typeof v === "string") return v;
  try {
    return JSON.stringify(v);
  } catch (e) {
    return String(v);
  }
}

function __fmt(v) {
  if (typeof v === "string") return JSON.stringify(v);
  if (v === undefined) return "undefined";
  try {
    return JSON.stringify(v);
  } catch (e) {
    return String(v);
  }
}

// `console.log` guarda el texto tal cual; el resto de niveles lo prefija con
// `[nivel] ` para que la consola de la app (y la CLI) puedan distinguirlos.
function __logger(level) {
  return function () {
    var text = Array.prototype.map.call(arguments, __str).join(" ");
    __h.logs.push(level ? "[" + level + "] " + text : text);
  };
}
var console = {
  log: __logger(""),
  info: __logger("info"),
  warn: __logger("warn"),
  error: __logger("error"),
  debug: __logger("debug"),
};

var harpoon = {
  __init: function (json) {
    var s = JSON.parse(json);
    __h.req = s.req || null;
    __h.res = s.res || null;
    __h.vars = s.vars || {};
    __h.env = s.env || {};
    __h.tests = [];
    __h.logs = [];
    harpoon.req = __h.req;
    harpoon.res = __h.res;
    if (harpoon.res) {
      var cache;
      harpoon.res.json = function () {
        if (cache === undefined) cache = JSON.parse(harpoon.res.body);
        return cache;
      };
      harpoon.res.text = function () {
        return harpoon.res.body;
      };
      harpoon.res.header = function (name) {
        var n = String(name).toLowerCase();
        for (var k in harpoon.res.headers) if (k.toLowerCase() === n) return harpoon.res.headers[k];
        return undefined;
      };
    }
  },
  __state: function () {
    return JSON.stringify({ req: harpoon.req, vars: __h.vars, tests: __h.tests, logs: __h.logs });
  },
  req: null,
  res: null,
  vars: {
    get: function (k) {
      return __h.vars[k];
    },
    set: function (k, v) {
      if (v === undefined || v === null) delete __h.vars[k];
      else __h.vars[k] = __str(v);
    },
    has: function (k) {
      return Object.prototype.hasOwnProperty.call(__h.vars, k);
    },
    unset: function (k) {
      delete __h.vars[k];
    },
    all: function () {
      return Object.assign({}, __h.vars);
    },
  },
  env: {
    get: function (k) {
      return __h.env[k];
    },
  },
};

function test(name, fn) {
  try {
    fn();
    __h.tests.push({ name: String(name), passed: true });
  } catch (e) {
    __h.tests.push({ name: String(name), passed: false, detail: e && e.message ? e.message : String(e) });
  }
}

function AssertionError(msg) {
  this.name = "AssertionError";
  this.message = msg;
}
AssertionError.prototype = Object.create(Error.prototype);

function __deepEq(a, b) {
  if (a === b) return true;
  if (typeof a !== typeof b || a === null || b === null) return false;
  if (typeof a !== "object") return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  var ka = Object.keys(a),
    kb = Object.keys(b);
  if (ka.length !== kb.length) return false;
  for (var i = 0; i < ka.length; i++) if (!__deepEq(a[ka[i]], b[ka[i]])) return false;
  return true;
}

function __type(v) {
  if (v === null) return "null";
  if (Array.isArray(v)) return "array";
  return typeof v;
}

function Expectation(actual, negate) {
  this.actual = actual;
  this.negate = !!negate;
  var self = this;
  // encadenables sin efecto
  ["to", "be", "been", "is", "that", "which", "and", "has", "have", "with", "at", "of", "same"].forEach(function (w) {
    Object.defineProperty(self, w, { get: function () { return self; } });
  });
  Object.defineProperty(self, "not", { get: function () { return new Expectation(self.actual, !self.negate); } });
  Object.defineProperty(self, "ok", { get: function () { return self.__check(!!self.actual, "esperaba un valor verdadero, recibido " + __fmt(self.actual)); } });
  Object.defineProperty(self, "true", { get: function () { return self.__check(self.actual === true, "esperaba true, recibido " + __fmt(self.actual)); } });
  Object.defineProperty(self, "false", { get: function () { return self.__check(self.actual === false, "esperaba false, recibido " + __fmt(self.actual)); } });
  Object.defineProperty(self, "null", { get: function () { return self.__check(self.actual === null, "esperaba null, recibido " + __fmt(self.actual)); } });
  Object.defineProperty(self, "undefined", { get: function () { return self.__check(self.actual === undefined, "esperaba undefined, recibido " + __fmt(self.actual)); } });
  Object.defineProperty(self, "empty", {
    get: function () {
      var a = self.actual;
      var n = a == null ? 0 : typeof a === "object" ? Object.keys(a).length : a.length;
      return self.__check(n === 0, "esperaba vacío, recibido " + __fmt(a));
    },
  });
  Object.defineProperty(self, "exist", { get: function () { return self.__check(self.actual != null, "esperaba que existiera, recibido " + __fmt(self.actual)); } });
}
Expectation.prototype.__check = function (cond, msg) {
  if (this.negate ? cond : !cond) throw new AssertionError(this.negate ? "NOT: " + msg : msg);
  return this;
};
Expectation.prototype.equal = function (v) {
  return this.__check(this.actual === v, "esperaba " + __fmt(v) + ", recibido " + __fmt(this.actual));
};
Expectation.prototype.equals = Expectation.prototype.equal;
Expectation.prototype.eq = Expectation.prototype.equal;
Expectation.prototype.eql = function (v) {
  return this.__check(__deepEq(this.actual, v), "esperaba (deep) " + __fmt(v) + ", recibido " + __fmt(this.actual));
};
Expectation.prototype.deep = Expectation.prototype; // expect(x).to.deep.equal → usar eql
Object.defineProperty(Expectation.prototype, "deep", {
  get: function () {
    var self = this;
    return { equal: function (v) { return self.eql(v); }, equals: function (v) { return self.eql(v); }, include: function (v) { return self.include(v); } };
  },
});
Expectation.prototype.include = function (v) {
  var a = this.actual;
  var ok = false;
  if (typeof a === "string") ok = a.indexOf(String(v)) !== -1;
  else if (Array.isArray(a)) ok = a.some(function (x) { return __deepEq(x, v); });
  else if (a && typeof a === "object" && v && typeof v === "object")
    ok = Object.keys(v).every(function (k) { return __deepEq(a[k], v[k]); });
  return this.__check(ok, "esperaba que " + __fmt(a) + " incluyera " + __fmt(v));
};
Expectation.prototype.includes = Expectation.prototype.include;
Expectation.prototype.contain = Expectation.prototype.include;
Expectation.prototype.contains = Expectation.prototype.include;
Expectation.prototype.property = function (p, v) {
  var a = this.actual;
  var has = a != null && p in Object(a);
  if (arguments.length > 1) return this.__check(has && __deepEq(a[p], v), "esperaba propiedad " + __fmt(p) + " = " + __fmt(v) + ", recibido " + __fmt(has ? a[p] : undefined));
  return this.__check(has, "esperaba propiedad " + __fmt(p) + " en " + __fmt(a));
};
Expectation.prototype.keys = function () {
  var ks = Array.prototype.slice.call(arguments);
  if (ks.length === 1 && Array.isArray(ks[0])) ks = ks[0];
  var a = this.actual || {};
  var ok = ks.every(function (k) { return k in a; });
  return this.__check(ok, "esperaba claves " + __fmt(ks) + " en " + __fmt(Object.keys(a)));
};
Expectation.prototype.length = function (n) {
  var a = this.actual;
  var len = a == null ? undefined : typeof a === "object" && !Array.isArray(a) ? Object.keys(a).length : a.length;
  return this.__check(len === n, "esperaba longitud " + n + ", recibido " + len);
};
Expectation.prototype.lengthOf = Expectation.prototype.length;
Expectation.prototype.above = function (n) { return this.__check(this.actual > n, "esperaba > " + n + ", recibido " + __fmt(this.actual)); };
Expectation.prototype.greaterThan = Expectation.prototype.above;
Expectation.prototype.gt = Expectation.prototype.above;
Expectation.prototype.below = function (n) { return this.__check(this.actual < n, "esperaba < " + n + ", recibido " + __fmt(this.actual)); };
Expectation.prototype.lessThan = Expectation.prototype.below;
Expectation.prototype.lt = Expectation.prototype.below;
Expectation.prototype.least = function (n) { return this.__check(this.actual >= n, "esperaba >= " + n + ", recibido " + __fmt(this.actual)); };
Expectation.prototype.gte = Expectation.prototype.least;
Expectation.prototype.most = function (n) { return this.__check(this.actual <= n, "esperaba <= " + n + ", recibido " + __fmt(this.actual)); };
Expectation.prototype.lte = Expectation.prototype.most;
Expectation.prototype.within = function (lo, hi) { return this.__check(this.actual >= lo && this.actual <= hi, "esperaba entre " + lo + " y " + hi + ", recibido " + __fmt(this.actual)); };
Expectation.prototype.match = function (re) { return this.__check(re.test(String(this.actual)), "esperaba que " + __fmt(this.actual) + " cumpliera " + String(re)); };
Expectation.prototype.matches = Expectation.prototype.match;
Expectation.prototype.string = function (s) { return this.__check(typeof this.actual === "string" && this.actual.indexOf(s) !== -1, "esperaba que " + __fmt(this.actual) + " contuviera " + __fmt(s)); };
Expectation.prototype.a = function (t) { return this.__check(__type(this.actual) === t, "esperaba tipo " + t + ", recibido " + __type(this.actual)); };
Expectation.prototype.an = Expectation.prototype.a;
Expectation.prototype.oneOf = function (list) { return this.__check(list.indexOf(this.actual) !== -1, "esperaba uno de " + __fmt(list) + ", recibido " + __fmt(this.actual)); };
Expectation.prototype.instanceOf = function (c) { return this.__check(this.actual instanceof c, "esperaba instancia de " + (c && c.name)); };
Expectation.prototype.throw = function () {
  var threw = false;
  try { this.actual(); } catch (e) { threw = true; }
  return this.__check(threw, "esperaba que lanzara una excepción");
};

function expect(v) {
  return new Expectation(v, false);
}

var assert = {
  ok: function (v, m) { if (!v) throw new AssertionError(m || "assert.ok falló: " + __fmt(v)); },
  equal: function (a, b, m) { if (a !== b) throw new AssertionError(m || "esperaba " + __fmt(b) + ", recibido " + __fmt(a)); },
  deepEqual: function (a, b, m) { if (!__deepEq(a, b)) throw new AssertionError(m || "esperaba (deep) " + __fmt(b) + ", recibido " + __fmt(a)); },
  fail: function (m) { throw new AssertionError(m || "assert.fail"); },
};

// Alias para quien venga de Postman.
var pm = {
  test: test,
  expect: expect,
  variables: harpoon.vars,
  environment: harpoon.vars,
  get response() {
    return harpoon.res;
  },
  get request() {
    return harpoon.req;
  },
};
