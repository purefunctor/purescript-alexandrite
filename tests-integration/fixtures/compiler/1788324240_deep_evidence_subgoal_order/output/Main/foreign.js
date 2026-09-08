const trace = [];

export const observe = label => {
  trace.push(label);
  return 0;
};

export const readTrace = () => trace.slice();
