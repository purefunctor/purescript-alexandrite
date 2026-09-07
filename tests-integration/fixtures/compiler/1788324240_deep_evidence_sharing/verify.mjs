import { strictEqual } from "node:assert/strict";
import { constructionCount } from "./output/Main/foreign.js";

const Main = await import("./output/Main/index.js");

strictEqual(Main.result, 0);
strictEqual(constructionCount(), 40);
