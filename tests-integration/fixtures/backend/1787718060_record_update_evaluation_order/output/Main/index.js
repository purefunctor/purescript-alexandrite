import * as $foreign from "./foreign.js";

export function updateLocal(source) {
  const $record = source;
  const $field = observe("local-first")(10 | 0);
  const $field$1 = observe("local-nested")(20 | 0);
  const $field$2 = observe("local-last")(30 | 0);
  return {
    ...$record,
    first: $field,
    nested: { ...$record.nested, value: $field$1 },
    last: $field$2
  };
}

export function updateCall(shouldThrow) {
  const $record = makeModel("call");
  const $field = observe("call-first")(10 | 0);
  const $field$1 = failAt("call-nested")(shouldThrow)(20 | 0);
  const $field$2 = observe("call-last")(30 | 0);
  return {
    ...$record,
    first: $field,
    nested: { ...$record.nested, value: $field$1 },
    last: $field$2
  };
}

export function updateControl(condition) {
  return source => {
    const $record = source;
    const $field = observe("control-first")(10 | 0);
    let $result;
    if (condition) {
      $result = observe("control-then")(30 | 0);
    } else {
      $result = observe("control-else")(31 | 0);
    }
    const $field$1 = $result;
    return { ...$record, first: $field, last: $field$1 };
  };
}

export function updateOpen(source) {
  const $record = source;
  const $field = observe("open-first")(40 | 0);
  return { ...$record, first: $field };
}

export const failAt = $foreign["failAt"];
export const makeModel = $foreign["makeModel"];
export const observe = $foreign["observe"];
