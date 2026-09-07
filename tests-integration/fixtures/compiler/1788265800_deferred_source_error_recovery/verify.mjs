import * as Main from "./output/Main/index.js";

if (Main.usable !== 42) {
  throw new Error(`unexpected usable value: ${Main.usable}`);
}

let deferredFailure;
try {
  Main.deferred(0);
} catch (error) {
  deferredFailure = error;
}
if (
  !(deferredFailure instanceof Error) ||
  deferredFailure.message !== "Generated code reached a source error"
) {
  throw new Error(`unexpected deferred source error: ${deferredFailure}`);
}

const partiallyApplied = Main.nested(0);
if (typeof partiallyApplied !== "function") {
  throw new Error(`unexpected partial application: ${partiallyApplied}`);
}

let nestedFailure;
try {
  partiallyApplied(0);
} catch (error) {
  nestedFailure = error;
}
if (!(nestedFailure instanceof Error) || nestedFailure.message !== "Generated code reached a source error") {
  throw new Error(`unexpected nested source error: ${nestedFailure}`);
}
