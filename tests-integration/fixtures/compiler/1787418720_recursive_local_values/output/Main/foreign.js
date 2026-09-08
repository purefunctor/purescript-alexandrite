let trace = [];

export const same = first => second => first === second;

export const observe = name => value => {
  trace.push(name);
  return value;
};

export const readTrace = () => trace.slice();

export const resetTrace = () => {
  trace = [];
};
