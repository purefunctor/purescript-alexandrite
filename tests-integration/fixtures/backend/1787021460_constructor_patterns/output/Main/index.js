export const Empty = ["Empty"];
export const One = $value0 => ["One", $value0];
export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];
export const Outer = $value0 => ["Outer", $value0];

export function first($choice) {
  function case$join$1(result$1) {
    return result$1;
  }

  if (Array.isArray($choice) && $choice[0] === "Empty") {
    return Empty;
  } else {
    if (Array.isArray($choice) && $choice[0] === "One") {
      return One($choice[1]);
    } else {
      if (Array.isArray($choice) && $choice[0] === "Pair") {
        const left = $choice[1];
        const argument = $choice[2];
        if (Array.isArray($choice) && $choice[0] === "Pair") {
          const argument$1 = $choice[1];
          const argument$2 = $choice[2];
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

export function nested($nested) {
  function case$1() {
    return Empty;
  }

  if (Array.isArray($nested) && $nested[0] === "Outer") {
    const argument = $nested[1];
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
  return bind(identity)(value => value);
}
