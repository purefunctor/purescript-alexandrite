import * as Data_Eq from "../Data.Eq/index.js";

function compareTwice$closure(dictionary0) {
  return left => {
    return right => {
      const eq = dictionary0.eq;
      const call = eq(left);
      const call$1 = call(right);
      if (call$1) {
        const eq$1 = dictionary0.eq;
        const call$2 = eq$1(right);
        const call$3 = call$2(left);
        return call$3;
      } else {
        const literal = false;
        return literal;
      }
    };
  };
}

function compareGenericArraysTwice$closure(dictionary1) {
  return left => {
    return right => {
      const eqArray = Data_Eq.eqArray;
      const call = eqArray(dictionary1);
      const eq = call.eq;
      const call$1 = eq(left);
      const call$2 = call$1(right);
      if (call$2) {
        const eqArray$1 = Data_Eq.eqArray;
        const call$3 = eqArray$1(dictionary1);
        const eq$1 = call$3.eq;
        const call$4 = eq$1(right);
        const call$5 = call$4(left);
        return call$5;
      } else {
        const literal = false;
        return literal;
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

          const eqArray = Data_Eq.eqArray;
          const call = eqArray(dictionary2);
          const eq = call.eq;
          const call$1 = eq(leftA);
          const call$2 = call$1(rightA);
          if (call$2) {
            const eqArray$1 = Data_Eq.eqArray;
            const call$3 = eqArray$1(dictionary2);
            const eq$1 = call$3.eq;
            const call$4 = eq$1(rightA);
            const call$5 = call$4(leftA);
            return call$5;
          } else {
            const eqArray$2 = Data_Eq.eqArray;
            const call$6 = eqArray$2(dictionary3);
            const eq$2 = call$6.eq;
            const call$7 = eq$2(leftB);
            const call$8 = call$7(rightB);
            if (call$8) {
              const eqArray$3 = Data_Eq.eqArray;
              const call$9 = eqArray$3(dictionary3);
              const eq$3 = call$9.eq;
              const call$10 = eq$3(rightB);
              const call$11 = call$10(leftB);
              return if$join$1(call$11);
            } else {
              const literal = false;
              return if$join$1(literal);
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
      const eqArray = Data_Eq.eqArray;
      const superclass62 = dictionary4.superclass62;
      const call = eqArray(superclass62);
      const eq = call.eq;
      const call$1 = eq(left);
      const call$2 = call$1(right);
      if (call$2) {
        const eqArray$1 = Data_Eq.eqArray;
        const superclass62$1 = dictionary4.superclass62;
        const call$3 = eqArray$1(superclass62$1);
        const eq$1 = call$3.eq;
        const call$4 = eq$1(right);
        const call$5 = call$4(left);
        return call$5;
      } else {
        const literal = false;
        return literal;
      }
    };
  };
}

function compareSuperclassTwice$closure(dictionary5) {
  return left => {
    return right => {
      const superclass62 = dictionary5.superclass62;
      const eq = superclass62.eq;
      const call = eq(left);
      const call$1 = call(right);
      if (call$1) {
        const superclass62$1 = dictionary5.superclass62;
        const eq$1 = superclass62$1.eq;
        const call$2 = eq$1(right);
        const call$3 = call$2(left);
        return call$3;
      } else {
        const literal = false;
        return literal;
      }
    };
  };
}

function lambdaScope$closure(lambdaLeft) {
  return lambdaRight => {
    const eqArray = Data_Eq.eqArray;
    const eqInt = Data_Eq.eqInt;
    const call = eqArray(eqInt);
    const eq = call.eq;
    const call$1 = eq(lambdaLeft);
    const call$2 = call$1(lambdaRight);
    if (call$2) {
      const eqArray$1 = Data_Eq.eqArray;
      const eqInt$1 = Data_Eq.eqInt;
      const call$3 = eqArray$1(eqInt$1);
      const eq$1 = call$3.eq;
      const call$4 = eq$1(lambdaRight);
      const call$5 = call$4(lambdaLeft);
      return call$5;
    } else {
      const literal = false;
      return literal;
    }
  };
}

function whereIsolation$closure(helperLeft) {
  return helperRight => {
    const eqArray = Data_Eq.eqArray;
    const eqInt = Data_Eq.eqInt;
    const call = eqArray(eqInt);
    const eq = call.eq;
    const call$1 = eq(helperLeft);
    const call$2 = call$1(helperRight);
    return call$2;
  };
}

export function compareTwice(dictionary0) {
  const closure = compareTwice$closure(dictionary0);
  return closure;
}

export function compareIntsTwice(left) {
  return right => {
    const eqInt = Data_Eq.eqInt;
    const eq = eqInt.eq;
    const call = eq(left);
    const call$1 = call(right);
    if (call$1) {
      const eqInt$1 = Data_Eq.eqInt;
      const eq$1 = eqInt$1.eq;
      const call$2 = eq$1(right);
      const call$3 = call$2(left);
      return call$3;
    } else {
      const literal = false;
      return literal;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    const eqArray = Data_Eq.eqArray;
    const eqInt = Data_Eq.eqInt;
    const call = eqArray(eqInt);
    const eq = call.eq;
    const call$1 = eq(left);
    const call$2 = call$1(right);
    if (call$2) {
      const eqArray$1 = Data_Eq.eqArray;
      const eqInt$1 = Data_Eq.eqInt;
      const call$3 = eqArray$1(eqInt$1);
      const eq$1 = call$3.eq;
      const call$4 = eq$1(right);
      const call$5 = call$4(left);
      return call$5;
    } else {
      const literal = false;
      return literal;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    const eqArray = Data_Eq.eqArray;
    const eqInt = Data_Eq.eqInt;
    const call = eqArray(eqInt);
    const eq = call.eq;
    const call$1 = eq(left);
    const call$2 = call$1(right);
    return call$2;
  };
}

export function compareGenericArraysTwice(dictionary1) {
  const closure = compareGenericArraysTwice$closure(dictionary1);
  return closure;
}

export function compareNestedArraysTwice(left) {
  return right => {
    return nestedLeft => {
      return nestedRight => {
        const eqArray = Data_Eq.eqArray;
        const eqArray$1 = Data_Eq.eqArray;
        const eqInt = Data_Eq.eqInt;
        const call = eqArray$1(eqInt);
        const call$1 = eqArray(call);
        const eq = call$1.eq;
        const call$2 = eq(nestedLeft);
        const call$3 = call$2(nestedRight);
        if (call$3) {
          const eqArray$2 = Data_Eq.eqArray;
          const eqArray$3 = Data_Eq.eqArray;
          const eqInt$1 = Data_Eq.eqInt;
          const call$4 = eqArray$3(eqInt$1);
          const call$5 = eqArray$2(call$4);
          const eq$1 = call$5.eq;
          const call$6 = eq$1(nestedRight);
          const call$7 = call$6(nestedLeft);
          return call$7;
        } else {
          const eqArray$4 = Data_Eq.eqArray;
          const eqInt$2 = Data_Eq.eqInt;
          const call$8 = eqArray$4(eqInt$2);
          const eq$2 = call$8.eq;
          const call$9 = eq$2(left);
          const call$10 = call$9(right);
          return call$10;
        }
      };
    };
  };
}

export function distinctGivens(dictionary2) {
  return dictionary3 => {
    const closure = distinctGivens$closure(dictionary2, dictionary3);
    return closure;
  };
}

export function distinctSubgoals(leftInt) {
  return rightInt => {
    return leftBoolean => {
      return rightBoolean => {
        function if$join$1(result$1) {
          return result$1;
        }

        const eqArray = Data_Eq.eqArray;
        const eqInt = Data_Eq.eqInt;
        const call = eqArray(eqInt);
        const eq = call.eq;
        const call$1 = eq(leftInt);
        const call$2 = call$1(rightInt);
        if (call$2) {
          const eqArray$1 = Data_Eq.eqArray;
          const eqInt$1 = Data_Eq.eqInt;
          const call$3 = eqArray$1(eqInt$1);
          const eq$1 = call$3.eq;
          const call$4 = eq$1(rightInt);
          const call$5 = call$4(leftInt);
          return call$5;
        } else {
          const eqArray$2 = Data_Eq.eqArray;
          const eqBoolean = Data_Eq.eqBoolean;
          const call$6 = eqArray$2(eqBoolean);
          const eq$2 = call$6.eq;
          const call$7 = eq$2(leftBoolean);
          const call$8 = call$7(rightBoolean);
          if (call$8) {
            const eqArray$3 = Data_Eq.eqArray;
            const eqBoolean$1 = Data_Eq.eqBoolean;
            const call$9 = eqArray$3(eqBoolean$1);
            const eq$3 = call$9.eq;
            const call$10 = eq$3(rightBoolean);
            const call$11 = call$10(leftBoolean);
            return if$join$1(call$11);
          } else {
            const literal = false;
            return if$join$1(literal);
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

    const eqArray = Data_Eq.eqArray;
    const eqInt = Data_Eq.eqInt;
    const call = eqArray(eqInt);
    const eq = call.eq;
    const call$1 = eq(left);
    const call$2 = call$1(right);
    if (call$2) {
      const eqArray$1 = Data_Eq.eqArray;
      const eqInt$1 = Data_Eq.eqInt;
      const call$3 = eqArray$1(eqInt$1);
      const eq$1 = call$3.eq;
      const call$4 = eq$1(right);
      const call$5 = call$4(left);
      if (call$5) {
        const eqArray$2 = Data_Eq.eqArray;
        const eqInt$2 = Data_Eq.eqInt;
        const call$6 = eqArray$2(eqInt$2);
        const eq$2 = call$6.eq;
        const call$7 = eq$2(left);
        const call$8 = call$7(right);
        return if$join$1(call$8);
      } else {
        const literal = false;
        return if$join$1(literal);
      }
    } else {
      const literal$1 = false;
      return literal$1;
    }
  };
}

export function compareNestedArraysWhole(left) {
  return right => {
    const eqArray = Data_Eq.eqArray;
    const eqArray$1 = Data_Eq.eqArray;
    const eqInt = Data_Eq.eqInt;
    const call = eqArray$1(eqInt);
    const call$1 = eqArray(call);
    const eq = call$1.eq;
    const call$2 = eq(left);
    const call$3 = call$2(right);
    if (call$3) {
      const eqArray$2 = Data_Eq.eqArray;
      const eqArray$3 = Data_Eq.eqArray;
      const eqInt$1 = Data_Eq.eqInt;
      const call$4 = eqArray$3(eqInt$1);
      const call$5 = eqArray$2(call$4);
      const eq$1 = call$5.eq;
      const call$6 = eq$1(right);
      const call$7 = call$6(left);
      return call$7;
    } else {
      const literal = false;
      return literal;
    }
  };
}

export function compareSuperclassArraysTwice(dictionary4) {
  const closure = compareSuperclassArraysTwice$closure(dictionary4);
  return closure;
}

export function compareSuperclassTwice(dictionary5) {
  const closure = compareSuperclassTwice$closure(dictionary5);
  return closure;
}

export function lambdaScope(left) {
  return right => {
    const eqArray = Data_Eq.eqArray;
    const eqInt = Data_Eq.eqInt;
    const call = eqArray(eqInt);
    const eq = call.eq;
    const call$1 = eq(left);
    const call$2 = call$1(right);
    if (call$2) {
      const closure = lambdaScope$closure;
      const call$3 = closure(left);
      const call$4 = call$3(right);
      return call$4;
    } else {
      const literal = false;
      return literal;
    }
  };
}

export function whereIsolation(left) {
  return right => {
    const closure = whereIsolation$closure;
    const call = closure(left);
    const call$1 = call(right);
    if (call$1) {
      const eqArray = Data_Eq.eqArray;
      const eqInt = Data_Eq.eqInt;
      const call$2 = eqArray(eqInt);
      const eq = call$2.eq;
      const call$3 = eq(left);
      const call$4 = call$3(right);
      return call$4;
    } else {
      const literal = false;
      return literal;
    }
  };
}

export function equationScope(argument0) {
  return argument1 => {
    return argument2 => {
      const matches = argument0 === true;
      if (matches) {
        const eqArray = Data_Eq.eqArray;
        const eqInt = Data_Eq.eqInt;
        const call = eqArray(eqInt);
        const eq = call.eq;
        const call$1 = eq(argument1);
        const call$2 = call$1(argument2);
        return call$2;
      } else {
        const matches$1 = argument0 === false;
        if (matches$1) {
          const eqArray$1 = Data_Eq.eqArray;
          const eqInt$1 = Data_Eq.eqInt;
          const call$3 = eqArray$1(eqInt$1);
          const eq$1 = call$3.eq;
          const call$4 = eq$1(argument2);
          const call$5 = call$4(argument1);
          return call$5;
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  };
}

export const firstComparison = Data_Eq.eqInt.eq(1 | 0)(2 | 0);

export const secondComparison = Data_Eq.eqInt.eq(3 | 0)(4 | 0);
