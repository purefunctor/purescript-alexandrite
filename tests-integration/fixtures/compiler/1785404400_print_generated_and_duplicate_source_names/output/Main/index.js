export const Proxy = "Proxy";
export function generatedIdentity(value) {
  return value;
}
export function generatedConst(first) {
  return ($argument1) => {
    return first;
  };
}
export function generatedApply($function) {
  return (value) => {
    return $function(value);
  };
}
export function sourceIdentity(value) {
  return value;
}
export function sourceConst(first) {
  return ($second) => {
    return first;
  };
}
export function sourceNumeric(first) {
  return ($t1) => {
    return first;
  };
}
export function sourceShadowed(value) {
  return ($function) => {
    return value;
  };
}
