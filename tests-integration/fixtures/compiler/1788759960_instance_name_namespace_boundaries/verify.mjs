import assert from "node:assert/strict";

import * as main from "./output/Main/index.js";

assert.equal(main.test, 42);
assert.equal(main.shadow(7), 7);
assert.equal(main.record.local, 42);
assert.equal(main.classString, 43);
