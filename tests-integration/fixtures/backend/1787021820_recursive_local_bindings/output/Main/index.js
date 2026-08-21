function second$function(first) {
  return argument0 => {
    const matches = argument0 === true;
    if (matches) {
      return 2 | 0;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const call = first(true);
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
      return 1 | 0;
    } else {
      const matches$1 = argument0 === false;
      if (matches$1) {
        const call = second(true);
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
        const call = first(true);
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
        const call = second(true);
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
