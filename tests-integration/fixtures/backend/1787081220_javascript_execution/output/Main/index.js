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
  function capture$closure(captured) {
    return argument0 => {
      return captured;
    };
  }
  return capture$closure(captured);
}

export function apply($function) {
  return value => {
    return $function(value);
  };
}

export function addCaptured(amount) {
  function addCaptured$closure(amount) {
    return value => {
      return addInt(amount)(value);
    };
  }
  return addCaptured$closure(amount);
}

export function nestedJoin(outer) {
  return inner => {
    function if$join$1(result$1, foreignValue$1) {
      return addInt(foreignValue$1)(result$1);
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
    function localSecond$function(captured, localFirst) {
      return argument0 => {
        if (argument0 === true) {
          return captured;
        } else {
          if (argument0 === false) {
            return localFirst(true);
          } else {
            throw new Error("Pattern match failure");
          }
        }
      };
    }
    function localFirst$function(captured, localSecond) {
      return argument0 => {
        if (argument0 === true) {
          return captured;
        } else {
          if (argument0 === false) {
            return localSecond(true);
          } else {
            throw new Error("Pattern match failure");
          }
        }
      };
    }
    const localSecond = argument0 => localSecond$function(captured, localFirst)(argument0);
    const localFirst = argument0 => localFirst$function(captured, localSecond)(argument0);
    return localFirst(condition);
  };
}

export function first(choice) {
  if (Array.isArray(choice) && choice[0] === "None") {
    return 0 | 0;
  } else {
    if (Array.isArray(choice) && choice[0] === "Pair") {
      const left = choice[1];
      const argument = choice[2];
      return left;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function partialPattern(argument0) {
  if (Array.isArray(argument0) && argument0[0] === "Pair") {
    const left = argument0[1];
    const argument = argument0[2];
    return left;
  } else {
    throw new Error("Pattern match failure");
  }
}

export function unwrapWrapped(argument0) {
  if (Array.isArray(argument0) && argument0[0] === "Wrapped") {
    return argument0[1];
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
  return { ...model, count: 1 | 0, nested: { ...model.nested, enabled: false } };
})();

export const curried = apply(addCaptured(2 | 0))(40 | 0);

export const pair = Pair(7 | 0)(8 | 0);

export const crossModule = unwrapWrapped(Library.wrapped);

export const forwardReference = Library.forward;

export const measureInt = { measure: addInt(1 | 0) };

export const evidenceValue = measureInt.measure(41 | 0);

export { $await as "await" };
