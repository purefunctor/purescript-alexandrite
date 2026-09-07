import { deepStrictEqual } from "node:assert/strict";
import * as Library from "./output/Library/index.js";
import * as Main from "./output/Main/index.js";

let patternFailure = false;
try {
  Main.partialPattern(Main.None);
} catch (error) {
  patternFailure = error.message === "Pattern match failure";
}

const actual = {
  integer: Main.integer,
  number: Main.number,
  string: Main.string,
  array: Main.array,
  recordCount: Main.model.count,
  updatedCount: Main.updated.count,
  updatedNested: Main.updated.nested.enabled,
  originalNested: Main.model.nested.enabled,
  hostileProperty: Main.readHostile(Main.model),
  hostileExport: Main.await,
  protoProperty: Main.readProto(Main.model),
  recordPrototype: Object.getPrototypeOf(Main.model) === Object.prototype,
  closureCapture: Main.capture(42)(0),
  curriedApplication: Main.curried,
  sharedJoinCapture: Main.nestedJoin(true)(true),
  nestedBranch: Main.nestedJoin(false)(true),
  recursion: Main.countdown(5),
  mutualRecursion: Main.isEven(6) && Main.isOdd(5),
  recursivePeerAndFreeCapture: Main.capturedMutual(42)(false),
  nullaryConstructor: Main.None,
  constructorTag: Main.pair.tag,
  constructorArguments: [Main.pair._1, Main.pair._2],
  constructorPattern: Main.first(Main.pair),
  importedConstructorPattern: Main.unwrapWrapped(Library.Wrapped(34)),
  zeroArgumentConstructorPattern: Main.first(Main.None),
  curriedConstructorPattern: Main.first(Main.Pair(11)(12)),
  crossModuleConstructor: Main.crossModule,
  forwardReference: Main.forwardReference,
  foreignValue: Main.foreignValue,
  effectThunk: Main.effectValue(),
  evidence: Main.evidenceValue,
  patternFailure,
};

const expected = {
  integer: 42,
  number: 1.5,
  string: "alexandrite",
  array: [1, 2, 3],
  recordCount: 0,
  updatedCount: 1,
  updatedNested: false,
  originalNested: true,
  hostileProperty: 17,
  hostileExport: 17,
  protoProperty: "data, not a prototype",
  recordPrototype: true,
  closureCapture: 42,
  curriedApplication: 42,
  sharedJoinCapture: 10,
  nestedBranch: 0,
  recursion: 5,
  mutualRecursion: true,
  recursivePeerAndFreeCapture: 42,
  nullaryConstructor: "None",
  constructorTag: "Pair",
  constructorArguments: [7, 8],
  constructorPattern: 7,
  importedConstructorPattern: 34,
  zeroArgumentConstructorPattern: 0,
  curriedConstructorPattern: 11,
  crossModuleConstructor: 21,
  forwardReference: 13,
  foreignValue: 9,
  effectThunk: 41,
  evidence: 42,
  patternFailure: true,
};

deepStrictEqual(actual, expected, "unexpected output");
