import * as Data_Eq from "../Data.Eq/index.js";

function compareTwice$closure(dictionary0) {
  return left => {
    return right => {
      if ((0, dictionary0.eq)(left)(right)) {
        return (0, dictionary0.eq)(right)(left);
      } else {
        return false;
      }
    };
  };
}

function compareGenericArraysTwice$closure(dictionary1) {
  return left => {
    return right => {
      const call = (0, Data_Eq.eqArray)(dictionary1);
      if ((0, call.eq)(left)(right)) {
        return (0, call.eq)(right)(left);
      } else {
        return false;
      }
    };
  };
}

function distinctGivens$closure(dictionary2, dictionary3) {
  return leftA => {
    return rightA => {
      return leftB => {
        return rightB => {
          function if$join$1(result$1) {
            return result$1;
          }

          const call = (0, Data_Eq.eqArray)(dictionary2);
          const call$1 = (0, Data_Eq.eqArray)(dictionary3);
          if ((0, call.eq)(leftA)(rightA)) {
            return (0, call.eq)(rightA)(leftA);
          } else {
            if ((0, call$1.eq)(leftB)(rightB)) {
              return if$join$1((0, call$1.eq)(rightB)(leftB));
            } else {
              return if$join$1(false);
            }
          }
        };
      };
    };
  };
}

function compareSuperclassArraysTwice$closure(dictionary4) {
  return left => {
    return right => {
      const call = (0, Data_Eq.eqArray)(dictionary4.superclass62);
      if ((0, call.eq)(left)(right)) {
        return (0, call.eq)(right)(left);
      } else {
        return false;
      }
    };
  };
}

function compareSuperclassTwice$closure(dictionary5) {
  return left => {
    return right => {
      if ((0, dictionary5.superclass62.eq)(left)(right)) {
        return (0, dictionary5.superclass62.eq)(right)(left);
      } else {
        return false;
      }
    };
  };
}

function lambdaScope$closure(lambdaLeft) {
  return lambdaRight => {
    const call = (0, Data_Eq.eqArray)(Data_Eq.eqInt);
    if ((0, call.eq)(lambdaLeft)(lambdaRight)) {
      return (0, call.eq)(lambdaRight)(lambdaLeft);
    } else {
      return false;
    }
  };
}

function whereIsolation$closure(helperLeft) {
  return helperRight => {
    return (0, ((0, Data_Eq.eqArray)(Data_Eq.eqInt)).eq)(helperLeft)(helperRight);
  };
}

export function compareTwice(dictionary0) {
  return compareTwice$closure(dictionary0);
}

export function compareIntsTwice(left) {
  return right => {
    if ((0, Data_Eq.eqInt.eq)(left)(right)) {
      return (0, Data_Eq.eqInt.eq)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    const call = (0, Data_Eq.eqArray)(Data_Eq.eqInt);
    if ((0, call.eq)(left)(right)) {
      return (0, call.eq)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    return (0, ((0, Data_Eq.eqArray)(Data_Eq.eqInt)).eq)(left)(right);
  };
}

export function compareGenericArraysTwice(dictionary1) {
  return compareGenericArraysTwice$closure(dictionary1);
}

export function compareNestedArraysTwice(left) {
  return right => {
    return nestedLeft => {
      return nestedRight => {
        const call = (0, Data_Eq.eqArray)(Data_Eq.eqInt);
        const call$1 = (0, Data_Eq.eqArray)(call);
        if ((0, call$1.eq)(nestedLeft)(nestedRight)) {
          return (0, call$1.eq)(nestedRight)(nestedLeft);
        } else {
          return (0, call.eq)(left)(right);
        }
      };
    };
  };
}

export function distinctGivens(dictionary2) {
  return dictionary3 => {
    return distinctGivens$closure(dictionary2, dictionary3);
  };
}

export function distinctSubgoals(leftInt) {
  return rightInt => {
    return leftBoolean => {
      return rightBoolean => {
        function if$join$1(result$1) {
          return result$1;
        }

        const call = (0, Data_Eq.eqArray)(Data_Eq.eqInt);
        const call$1 = (0, Data_Eq.eqArray)(Data_Eq.eqBoolean);
        if ((0, call.eq)(leftInt)(rightInt)) {
          return (0, call.eq)(rightInt)(leftInt);
        } else {
          if ((0, call$1.eq)(leftBoolean)(rightBoolean)) {
            return if$join$1((0, call$1.eq)(rightBoolean)(leftBoolean));
          } else {
            return if$join$1(false);
          }
        }
      };
    };
  };
}

export function compareArraysThrice(left) {
  return right => {
    function if$join$1(result$1) {
      return result$1;
    }

    const call = (0, Data_Eq.eqArray)(Data_Eq.eqInt);
    if ((0, call.eq)(left)(right)) {
      if ((0, call.eq)(right)(left)) {
        return if$join$1((0, call.eq)(left)(right));
      } else {
        return if$join$1(false);
      }
    } else {
      return false;
    }
  };
}

export function compareNestedArraysWhole(left) {
  return right => {
    const call$1 = (0, Data_Eq.eqArray)((0, Data_Eq.eqArray)(Data_Eq.eqInt));
    if ((0, call$1.eq)(left)(right)) {
      return (0, call$1.eq)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareSuperclassArraysTwice(dictionary4) {
  return compareSuperclassArraysTwice$closure(dictionary4);
}

export function compareSuperclassTwice(dictionary5) {
  return compareSuperclassTwice$closure(dictionary5);
}

export function lambdaScope(left) {
  return right => {
    if ((0, ((0, Data_Eq.eqArray)(Data_Eq.eqInt)).eq)(left)(right)) {
      return lambdaScope$closure(left)(right);
    } else {
      return false;
    }
  };
}

export function whereIsolation(left) {
  return right => {
    if (whereIsolation$closure(left)(right)) {
      return (0, ((0, Data_Eq.eqArray)(Data_Eq.eqInt)).eq)(left)(right);
    } else {
      return false;
    }
  };
}

export function equationScope(argument0) {
  return argument1 => {
    return argument2 => {
      const call = (0, Data_Eq.eqArray)(Data_Eq.eqInt);
      if (argument0 === true) {
        return (0, call.eq)(argument1)(argument2);
      } else {
        if (argument0 === false) {
          return (0, call.eq)(argument2)(argument1);
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  };
}

export const firstComparison = (0, Data_Eq.eqInt.eq)(1 | 0)(2 | 0);

export const secondComparison = (0, Data_Eq.eqInt.eq)(3 | 0)(4 | 0);
