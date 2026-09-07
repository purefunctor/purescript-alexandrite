import * as $foreign from "./foreign.js";
export const First = "First";
export const Second = "Second";
export function branchResult(condition) {
  return (seed) => {
    let $result;
    if (condition) {
      $result = constructEffect("branch-then")(seed);
    } else {
      $result = constructEffect("branch-else")(seed);
    }
    return () => {
      const selected = $result();
      return constructEffect("branch-after")(selected)();
    };
  };
}
export function caseResult(choice) {
  return (seed) => {
    let $result;
    $case: {
      if (choice === "First") {
        $result = constructEffect("case-first")(seed);
        break $case;
      }
      if (choice === "Second") {
        $result = constructEffect("case-second")(seed);
        break $case;
      }
      throw new Error("Pattern match failure");
    }
    return () => {
      const selected = $result();
      return constructEffect("case-after")(selected)();
    };
  };
}
export function guardResult(condition) {
  return (seed) => {
    let $result;
    $case: {
      {
        const value = seed;
        if (condition) {
          $result = constructEffect("guard-true")(value);
          break $case;
        }
      }
      const value$1 = seed;
      $result = constructEffect("guard-false")(value$1);
      break $case;
    }
    return () => {
      const selected = $result();
      return constructEffect("guard-after")(selected)();
    };
  };
}
export const constructEffect = $foreign["constructEffect"];
