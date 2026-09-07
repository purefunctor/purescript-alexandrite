export const Proxy = "Proxy";
export function grouped(dictionary) {
  return dictionary.grouped;
}
export const grouped1 = { grouped: (value) => ($proxy) => ((localValue) => ($proxy$1) => localValue)(value)("Proxy") };
