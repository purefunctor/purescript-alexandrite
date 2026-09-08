export const Proxy = "Proxy";
export function tail($proxy) {
  return 42 | 0;
}
export function nonTail($proxy) {
  return 42 | 0;
}
export const asTail = [tail, nonTail];
export const asNonTail = [tail, nonTail];
