export const Unit = "Unit";
export const First = "First";
export const Second = "Second";
export const Third = "Third";
export const SpecT = "SpecT";
export function identity(x) {
  return x;
}
export function apply(x) {
  return ($function) => {
    return $function(x);
  };
}
export function describe($string) {
  return (specification) => {
    return specification;
  };
}
export function it($string) {
  return ($effect) => {
    return "SpecT";
  };
}
export function hoistSpec($naturalTransformation) {
  return ($function) => {
    return ($specT) => {
      return "SpecT";
    };
  };
}
export function catchFirst($first) {
  return "Second";
}
export function catchSecond($second) {
  return "Third";
}
export const runExample = "First";
export const spec = apply(describe("outer")(it("inner")(runExample)))(hoistSpec(identity)(($unit) => (value) => apply(apply(value)(catchFirst))(catchSecond)));
