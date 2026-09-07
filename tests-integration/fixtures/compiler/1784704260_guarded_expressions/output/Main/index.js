export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function recover(fallback) {
  return (input) => {
    if (input.tag === "Just") {
      const { _1: value } = input;
      return value;
    }
    if (true) {
      return fallback;
    }
    throw new Error("Pattern match failure");
  };
}
