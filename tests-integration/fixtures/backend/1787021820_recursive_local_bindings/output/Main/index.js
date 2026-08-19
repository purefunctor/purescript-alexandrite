function second$function(first) {
  return argument0 => {
    const matches = argument0 === true;
    if (matches) {
      const literal = 2 | 0;
      return literal;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const literal$1 = true;
        const call = first(literal$1);
        return call;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

function first$function(second) {
  return argument0 => {
    const matches = argument0 === true;
    if (matches) {
      const literal = 1 | 0;
      return literal;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const literal$1 = true;
        const call = second(literal$1);
        return call;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

function second$function$1(captured, first) {
  return argument0 => {
    const matches = argument0 === true;
    if (matches) {
      return captured;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const literal = true;
        const call = first(literal);
        return call;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

function first$function$1(captured, second) {
  return argument0 => {
    const matches = argument0 === true;
    if (matches) {
      return captured;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const literal = true;
        const call = second(literal);
        return call;
      } else {
        throw new Error("Pattern match failure");
      }
    }
  };
}

export function mutual(condition) {
  const second = argument0 => second$function(first)(argument0);
  const first = argument0 => first$function(second)(argument0);
  const call = first(condition);
  return call;
}

export function capturedMutual(captured) {
  return condition => {
    const second = argument0 => second$function$1(captured, first)(argument0);
    const first = argument0 => first$function$1(captured, second)(argument0);
    const call = first(condition);
    return call;
  };
}
