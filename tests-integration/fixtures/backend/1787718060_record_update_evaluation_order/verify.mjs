import { deepStrictEqual } from "node:assert/strict";
import * as Main from "./output/Main/index.js";
import {
  makeModel,
  readTrace,
  resetTrace,
  symbolKey,
} from "./output/Main/foreign.js";

resetTrace();
const localSource = makeModel("local");
const local = Main.updateLocal(localSource);
const localTrace = readTrace();

resetTrace();
const call = Main.updateCall(false);
const callTrace = readTrace();

resetTrace();
let callError;
try {
  Main.updateCall(true);
} catch (error) {
  callError = error.message;
}
const throwingCallTrace = readTrace();

resetTrace();
const deep = Main.updateDeep(false);
const deepTrace = readTrace();

resetTrace();
const control = Main.updateControl(true);
const controlConstructionTrace = readTrace();
const controlSource = makeModel("control");
const controlled = control(controlSource);
const controlTrace = readTrace();

resetTrace();
const openSource = makeModel("open");
const open = Main.updateOpen(openSource);
const openTrace = readTrace();

const copyTrace = label => [
  `${label}:keys`,
  `${label}:descriptor:first`,
  `${label}:get:first`,
  `${label}:descriptor:nested`,
  `${label}:get:nested`,
  `${label}:descriptor:last`,
  `${label}:get:last`,
  `${label}:descriptor:untouched`,
  `${label}:get:untouched`,
  `${label}:descriptor:Symbol(marker)`,
  `${label}:get:Symbol(marker)`,
];

const nestedCopyTrace = label => [
  `${label}:keys`,
  `${label}:descriptor:value`,
  `${label}:get:value`,
  `${label}:descriptor:inner`,
  `${label}:get:inner`,
  `${label}:descriptor:untouched`,
  `${label}:get:untouched`,
];

const innerCopyTrace = label => [
  `${label}:keys`,
  `${label}:descriptor:value`,
  `${label}:get:value`,
  `${label}:descriptor:untouched`,
  `${label}:get:untouched`,
];

deepStrictEqual(
  {
    localTrace,
    localValues: [local.first, local.nested.value, local.last],
    callTrace,
    callValues: [call.first, call.nested.value, call.last],
    callError,
    throwingCallTrace,
    deepTrace,
    deepValues: [deep.nested.value, deep.nested.inner.value],
    controlConstructionTrace,
    controlTrace,
    controlValues: [controlled.first, controlled.nested.value, controlled.last],
    openTrace,
    openValues: [open.first, open.untouched, open[symbolKey]],
  },
  {
    localTrace: [
      "make:local",
      ...copyTrace("local"),
      "observe:local-first",
      "local:get:nested",
      ...nestedCopyTrace("local.nested"),
      "observe:local-nested",
      "observe:local-last",
    ],
    localValues: [10, 20, 30],
    callTrace: [
      "make:call",
      ...copyTrace("call"),
      "observe:call-first",
      "call:get:nested",
      ...nestedCopyTrace("call.nested"),
      "fail:call-nested",
      "observe:call-last",
    ],
    callValues: [10, 20, 30],
    callError: "call-nested",
    throwingCallTrace: [
      "make:call",
      ...copyTrace("call"),
      "observe:call-first",
      "call:get:nested",
      ...nestedCopyTrace("call.nested"),
      "fail:call-nested",
    ],
    deepTrace: [
      "make:deep",
      ...copyTrace("deep"),
      "deep:get:nested",
      ...nestedCopyTrace("deep.nested"),
      "observe:deep-nested",
      "deep:get:nested",
      "deep.nested:get:inner",
      ...innerCopyTrace("deep.nested.inner"),
      "observe:deep-inner",
    ],
    deepValues: [50, 60],
    controlConstructionTrace: [],
    controlTrace: [
      "make:control",
      ...copyTrace("control"),
      "observe:control-first",
      "control:get:nested",
      ...nestedCopyTrace("control.nested"),
      "observe:control-then",
      "observe:control-last",
    ],
    controlValues: [10, 30, 32],
    openTrace: ["make:open", ...copyTrace("open"), "observe:open-first"],
    openValues: [40, 4, "symbol-value"],
  },
);
