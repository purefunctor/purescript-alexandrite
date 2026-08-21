export const Empty = ["Empty"];
export const One = $value0 => ["One", $value0];
export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];
export const Outer = $value0 => ["Outer", $value0];

export function first(argument0) {
  function case$join$1(result$1) {
    return result$1;
  }

  if (Array.isArray(argument0) && argument0[0] === "Empty") {
    return Empty;
  } else {
    if (Array.isArray(argument0) && argument0[0] === "One") {
      return One(argument0[1]);
    } else {
      if (Array.isArray(argument0) && argument0[0] === "Pair") {
        const left = argument0[1];
        const argument = argument0[2];
        if (Array.isArray(argument0) && argument0[0] === "Pair") {
          const argument$1 = argument0[1];
          const argument$2 = argument0[2];
          return case$join$1(One(left));
        } else {
          return case$join$1(Empty);
        }
      } else {
        throw new Error("Pattern match failure");
      }
    }
  }
}

export function unwrap(value) {
  return value;
}

export function nested(argument0) {
  function case$1() {
    return Empty;
  }

  if (Array.isArray(argument0) && argument0[0] === "Outer") {
    const argument = argument0[1];
    if (Array.isArray(argument) && argument[0] === "One") {
      return One(argument[1]);
    } else {
      return case$1();
    }
  } else {
    return case$1();
  }
}

export function bind(value) {
  return continuation => {
    return continuation(value);
  };
}

export function ordinaryBind(identity) {
  function ordinaryBind$closure(value) {
    return value;
  }
  return bind(identity)(ordinaryBind$closure);
}
