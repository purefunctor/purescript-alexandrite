export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function local(partialDict) {
  const $closure = (choice) => {
    const $closure$1 = (wrapped) => {
      if (wrapped.tag === "Just") {
        const { _1: value } = wrapped;
        return value;
      } else {
        throw new Error("Pattern match failure");
      }
    };
    return $closure$1(choice);
  };
  return $closure;
}
