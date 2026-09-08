export const Nil = "Nil";
export const Cons = ($value0) => ({
  tag: "Cons",
  _1: $value0
});
export function append(section36) {
  return (section37) => {
    if (section36 === "Nil" && section37 === "Nil") {
      return "Nil";
    }
    if (section36.tag === "Cons" && section37 === "Nil") {
      const { _1: p } = section36;
      return {
        tag: "Cons",
        _1: append(p)("Nil")
      };
    }
    if (section36 === "Nil" && section37.tag === "Cons") {
      const { _1: p$1 } = section37;
      return {
        tag: "Cons",
        _1: append("Nil")(p$1)
      };
    }
    if (section36.tag === "Cons" && section37.tag === "Cons") {
      const { _1: p$2 } = section36;
      const { _1: r } = section37;
      return {
        tag: "Cons",
        _1: append(p$2)(r)
      };
    }
    throw new Error("Pattern match failure");
  };
}
