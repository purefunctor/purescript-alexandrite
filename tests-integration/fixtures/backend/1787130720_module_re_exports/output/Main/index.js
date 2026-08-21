import * as Direct from "../Direct/index.js";
import * as Origin from "../Origin/index.js";
import * as Transitive from "../Transitive/index.js";

export const constructorValue = (0, Origin.Just)(42 | 0);

export const operatorValue = (0, Origin.append)((0, Origin.visible)(23 | 0))(9 | 0);

export const localCollision = Direct.append;

export const foreignResult = Origin.foreignValue;

export const hostileResult = Origin.await;

export const measured = (0, Origin.measureInt.measure)(41 | 0);

export const transitiveMarker = Transitive.marker;

export { append } from "../Direct/index.js";
export { Just, "await", foreignValue, visible } from "../Origin/index.js";
export { marker } from "../Transitive/index.js";
