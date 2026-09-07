import assert from "node:assert/strict";

import { result } from "./output/Main/index.js";
import * as Library from "./output/Library/index.js";

assert.deepEqual(result, {
  third: 3,
  first: 1,
  second: 2,
  named: 4,
  repeated: 3,
  local: 10,
});
assert.equal(Library.pick1, 90);
for (const name of ["pick2", "pick3", "pick4", "pick5"]) {
  assert.ok(Object.hasOwn(Library, name), `missing dictionary ${name}`);
}
