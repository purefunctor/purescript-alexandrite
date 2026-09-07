import { result } from "./output/Main/index.js";

if (result !== 42) {
  throw new Error(`expected cyclic instance dictionaries to produce 42, received ${result}`);
}
