const trace = [];

export const resetTrace = () => {
  trace.length = 0;
};

export const readTrace = () => trace.slice();

export const constructEffect = label => value => {
  trace.push(`construct:${label}`);
  return () => {
    trace.push(`run:${label}`);
    return value;
  };
};
