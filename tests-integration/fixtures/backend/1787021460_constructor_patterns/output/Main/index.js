export const Empty = ["Empty"];
export const One = $value0 => ["One", $value0];
export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];
export const Outer = $value0 => ["Outer", $value0];

function ordinaryBind$closure(value) {
  return value;
}

export function first(argument0) {
  function case$join$1(result$1) {
    return result$1;
  }

  const matches = Array.isArray(argument0) && argument0[0] === "Empty";
  if (matches) {
    return Empty;
  } else {
    const matches$1 = Array.isArray(argument0) && argument0[0] === "One";
    if (matches$1) {
      const value = argument0[1];
      const call = One(value);
      return call;
    } else {
      const matches$2 = Array.isArray(argument0) && argument0[0] === "Pair";
      if (matches$2) {
        const left = argument0[1];
        const argument = argument0[2];
        const matches$3 = Array.isArray(argument0) && argument0[0] === "Pair";
        if (matches$3) {
          const argument$1 = argument0[1];
          const argument$2 = argument0[2];
          const call$1 = One(left);
          return case$join$1(call$1);
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

  const matches = Array.isArray(argument0) && argument0[0] === "Outer";
  if (matches) {
    const argument = argument0[1];
    const matches$1 = Array.isArray(argument) && argument[0] === "One";
    if (matches$1) {
      const value = argument[1];
      const call = One(value);
      return call;
    } else {
      return case$1();
    }
  } else {
    return case$1();
  }
}

export function bind(value) {
  return continuation => {
    const call = continuation(value);
    return call;
  };
}

export function ordinaryBind(identity) {
  const call = bind(identity);
  const call$1 = call(ordinaryBind$closure);
  return call$1;
}
