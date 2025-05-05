import { core } from "ext:core/mod.js";

import "ext:deno_console/01_console.js";
import "ext:deno_url/00_url.js";
import "ext:deno_url/01_urlpattern.js";
import "ext:deno_web/00_infra.js";
import "ext:deno_web/01_dom_exception.js";
import "ext:deno_web/01_mimesniff.js";
import "ext:deno_web/02_event.js";
import "ext:deno_web/02_structured_clone.js";
import "ext:deno_web/02_timers.js";
import "ext:deno_web/03_abort_signal.js";
import "ext:deno_web/04_global_interfaces.js";
import "ext:deno_web/05_base64.js";
import "ext:deno_web/06_streams.js";
import "ext:deno_web/08_text_encoding.js";
import "ext:deno_web/09_file.js";
import "ext:deno_web/10_filereader.js";
import "ext:deno_web/12_location.js";
import "ext:deno_web/13_message_port.js";
import "ext:deno_web/14_compression.js";
import "ext:deno_web/15_performance.js";
import "ext:deno_web/16_image_data.js";
import "ext:deno_webidl/00_webidl.js";

import { Console, setNoColorFns } from "ext:deno_console/01_console.js";
import { URL } from "ext:deno_url/00_url.js";
import { setTimeout } from "ext:deno_web/02_timers.js";
import { atob, btoa } from "ext:deno_web/05_base64.js";
import { TextDecoder } from "ext:deno_web/08_text_encoding.js";

/** @type {Pick<typeof globalThis.Deno, "cwd" | "readTextFileSync">} */
const Deno = {
  cwd: () => core.ops.op_example_cwd(),
  readTextFileSync: (path) =>
    new TextDecoder().decode(core.ops.op_example_read_file(path.toString())),
};

const publicFinal = {
  configurable: false,
  enumerable: true,
};

Object.entries({
  URL,
  Deno,
  atob,
  btoa,
  setTimeout,
}).forEach(([key, value]) =>
  Object.defineProperty(globalThis, key, {
    get: () => value,
    ...publicFinal,
  }),
);

{
  let stdout = "";
  const console = new Console((msg) => (core.print(msg), (stdout += msg)));
  Object.defineProperties(globalThis, {
    console: {
      get: () => console,
      ...publicFinal,
    },
    __cargo_test_stdout__: {
      get: () => stdout,
      ...publicFinal,
    },
  });
  setNoColorFns(
    () => true,
    () => true,
  );
}
