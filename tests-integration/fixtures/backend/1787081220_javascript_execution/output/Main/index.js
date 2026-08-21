import * as Library from "../Library/index.js";
import * as $foreign from "./foreign.js";

export const None = ["None"];
export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];

function capture$closure(captured) {
  return argument0 => {
    return captured;
  };
}

function addCaptured$closure(amount) {
  return value => {
    const call = addInt(amount);
    const call$1 = call(value);
    return call$1;
  };
}

function localSecond$function(captured, localFirst) {
  return argument0 => {
    const matches = argument0 === true;
    if (matches) {
      return captured;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const call = localFirst(true);
        return call;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

function localFirst$function(captured, localSecond) {
  return argument0 => {
    const matches = argument0 === true;
    if (matches) {
      return captured;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const call = localSecond(true);
        return call;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

export function readHostile(value) {
  return value["hostile-field"];
}

export function readProto(value) {
  return value.__proto__;
}

export function capture(captured) {
  const closure = capture$closure(captured);
  return closure;
}

export function apply($function) {
  return value => {
    const call = $function(value);
    return call;
  };
}

export function addCaptured(amount) {
  const closure = addCaptured$closure(amount);
  return closure;
}

export function nestedJoin(outer) {
  return inner => {
    function if$join$1(result$1, foreignValue$1) {
      const call = addInt(foreignValue$1);
      const call$1 = call(result$1);
      return call$1;
    }

    if (outer) {
      const foreignValue$1 = foreignValue;
      if (inner) {
        return if$join$1(1 | 0, foreignValue$1);
      } else {
        return if$join$1(2 | 0, foreignValue$1);
      }
    } else {
      return 0 | 0;
    }
  };
}

export function countdown(value) {
  const call = equalInt(value);
  const call$1 = call(0 | 0);
  if (call$1) {
    return 0 | 0;
  } else {
    const call$2 = addInt(1 | 0);
    const countdown$1 = countdown;
    const call$3 = decrementInt(value);
    const call$4 = countdown$1(call$3);
    const call$5 = call$2(call$4);
    return call$5;
  }
}

export function isEven(value) {
  const call = equalInt(value);
  const call$1 = call(0 | 0);
  if (call$1) {
    return true;
  } else {
    const isOdd$1 = isOdd;
    const call$2 = decrementInt(value);
    const call$3 = isOdd$1(call$2);
    return call$3;
  }
}

export function isOdd(value) {
  const call = equalInt(value);
  const call$1 = call(0 | 0);
  if (call$1) {
    return false;
  } else {
    const isEven$1 = isEven;
    const call$2 = decrementInt(value);
    const call$3 = isEven$1(call$2);
    return call$3;
  }
}

export function capturedMutual(captured) {
  return condition => {
    const localSecond = argument0 => localSecond$function(captured, localFirst)(argument0);
    const localFirst = argument0 => localFirst$function(captured, localSecond)(argument0);
    const call = localFirst(condition);
    return call;
  };
}

export function first(choice) {
  const matches = Array.isArray(choice) && choice[0] === "None";
  if (matches) {
    return 0 | 0;
  } else {
    const matches$1 = Array.isArray(choice) && choice[0] === "Pair";
    if (matches$1) {
      const left = choice[1];
      const argument = choice[2];
      return left;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function partialPattern(argument0) {
  const matches = Array.isArray(argument0) && argument0[0] === "Pair";
  if (matches) {
    const left = argument0[1];
    const argument = argument0[2];
    return left;
  } else {
    throw new Error("Pattern match failure");
  }
}

export function unwrapWrapped(argument0) {
  const matches = Array.isArray(argument0) && argument0[0] === "Wrapped";
  if (matches) {
    const value = argument0[1];
    return value;
  } else {
    throw new Error("Pattern match failure");
  }
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
  const updated$1 = { ...model, count: 1 | 0, nested: { ...model.nested, enabled: false } };
  return updated$1;
})();

export const curried = apply(addCaptured(2 | 0))(40 | 0);

export const pair = Pair(7 | 0)(8 | 0);

export const crossModule = unwrapWrapped(Library.wrapped);

export const forwardReference = Library.forward;

export const measureInt = { measure: addInt(1 | 0) };

export const evidenceValue = measureInt.measure(41 | 0);

export { $await as "await" };
