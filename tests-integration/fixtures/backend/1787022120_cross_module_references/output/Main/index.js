import * as Library from "../Library/index.js";

export function unbox(argument0) {
  if (Array.isArray(argument0) && argument0[0] === "Box") {
    return argument0[1];
  } else {
    throw new Error("Pattern match failure");
  }
}

export const fromLibrary = unbox(Library.box);

export const directReference = Library.libraryValue;
