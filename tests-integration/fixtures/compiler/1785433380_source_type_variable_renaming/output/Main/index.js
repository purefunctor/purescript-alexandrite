export const Proxy = "Proxy";
export function identity(value) {
  return value;
}
export function grouped(dictionary) {
  return dictionary.grouped;
}
export function ranked(dictionary) {
  return dictionary.ranked;
}
export const grouped1 = {
  grouped: (value) => ($proxy) => identity(value),
  ranked: ($function) => "Proxy"
};
