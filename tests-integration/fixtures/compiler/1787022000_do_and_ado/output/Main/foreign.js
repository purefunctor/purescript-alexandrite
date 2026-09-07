const trace = [];

export const resetTrace = () => {
  trace.length = 0;
};
export const readTrace = () => trace.slice();

export const firstAction = () => {
  trace.push("first");
  return 20;
};
export const secondAction = value => () => {
  trace.push("second");
  return `value:${value}`;
};
export const independentAction = () => {
  trace.push("independent");
  return true;
};
