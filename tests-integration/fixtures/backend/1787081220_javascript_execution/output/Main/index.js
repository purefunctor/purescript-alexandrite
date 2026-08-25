import * as Library from "../Library/index.js";
import * as $foreign from "./foreign.js";

export const None = ["None"];
export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];

export function readHostile(value) {
  return value["hostile-field"];
}

export function readProto(value) {
  return value.__proto__;
}

export function capture(captured) {
  return $int => captured;
}

export function apply($function) {
  return value => {
    return $function(value);
  };
}

export function addCaptured(amount) {
  return value => addInt(amount)(value);
}

export function nestedJoin(outer) {
  return inner => {
    if (outer) {
      let $result;
      if (inner) {
        $result = 1 | 0;
      } else {
        $result = 2 | 0;
      }
      const result = $result;
      return addInt(foreignValue)(result);
    } else {
      return 0 | 0;
    }
  };
}

export function countdown(value) {
  if (equalInt(value)(0 | 0)) {
    return 0 | 0;
  } else {
    return addInt(1 | 0)(countdown(decrementInt(value)));
  }
}

export function isEven(value) {
  if (equalInt(value)(0 | 0)) {
    return true;
  } else {
    return isOdd(decrementInt(value));
  }
}

export function isOdd(value) {
  if (equalInt(value)(0 | 0)) {
    return false;
  } else {
    return isEven(decrementInt(value));
  }
}

export function capturedMutual(captured) {
  return condition => {
    const localSecond = $boolean => {
      if ($boolean === true) {
        return captured;
      }
      if ($boolean === false) {
        return localFirst(true);
      }
      throw new Error("Pattern match failure");
    };
    const localFirst = $boolean$1 => {
      if ($boolean$1 === true) {
        return captured;
      }
      if ($boolean$1 === false) {
        return localSecond(true);
      }
      throw new Error("Pattern match failure");
    };
    return localFirst(condition);
  };
}

export function first(choice) {
  if (Array.isArray(choice) && choice[0] === "None") {
    return 0 | 0;
  }
  if (Array.isArray(choice) && choice[0] === "Pair") {
    const left = choice[1];
    return left;
  }
  throw new Error("Pattern match failure");
}

export function partialPattern($choice) {
  if (Array.isArray($choice) && $choice[0] === "Pair") {
    const left = $choice[1];
    return left;
  } else {
    throw new Error("Pattern match failure");
  }
}

export function unwrapWrapped($wrapped) {
  if (Array.isArray($wrapped) && $wrapped[0] === "Wrapped") {
    const value = $wrapped[1];
    return value;
  } else {
    throw new Error("Pattern match failure");
  }
}

export function measure(dictionary) {
  return dictionary.measure;
}

export const addInt = $foreign["addInt"];
export const decrementInt = $foreign["decrementInt"];
export const equalInt = $foreign["equalInt"];
export const foreignValue = $foreign["foreignValue"];
export const effectValue = $foreign["effectValue"];
const $await = $foreign["await"];

export const integer = 42 | 0;

export const number = 1.5;

export const string = "alexandrite";

export const array = [1 | 0, 2 | 0, 3 | 0];

export const model = {
  count: 0 | 0,
  nested: { enabled: true },
  "hostile-field": $await,
  ["__proto__"]: "data, not a prototype"
};

export const updated = (() => {
  const $record = model;
  const $field = 1 | 0;
  const $field$1 = false;
  const $update = { ...$record, count: $field, nested: { ...$record.nested, enabled: $field$1 } };
  return $update;
})();

export const curried = apply(addCaptured(2 | 0))(40 | 0);

export const pair = Pair(7 | 0)(8 | 0);

export const crossModule = unwrapWrapped(Library.wrapped);

export const forwardReference = Library.forward;

export const measureInt = { measure: addInt(1 | 0) };

export const evidenceValue = measure(measureInt)(41 | 0);

export { $await as "await" };
