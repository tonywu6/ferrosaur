// @ts-check

import { buildExample, relpath } from "../_runtime/src/lib.js";

await buildExample([relpath("src/main.ts", import.meta)]);
