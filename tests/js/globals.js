// @ts-check

/// <reference no-default-lib="true"/>
/// <reference path="https://raw.githubusercontent.com/denoland/deno_core/refs/tags/0.339.0/core/core.d.ts" />
/// <reference path="https://raw.githubusercontent.com/denoland/deno/refs/tags/v2.2.1/cli/tsc/dts/lib.deno.shared_globals.d.ts" />

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

import { URL } from "ext:deno_url/00_url.js";
import { Console, setNoColorFns } from "ext:deno_console/01_console.js";
import { setTimeout } from "ext:deno_web/02_timers.js";

const { console, __cargo_test_stdout__ } = (() => {
  let stdout = "";

  const console = new Console((msg) => (stdout += msg));

  setNoColorFns(
    () => true,
    () => true,
  );

  const __cargo_test_stdout__ = () => stdout;

  return { console, __cargo_test_stdout__ };
})();

const items = {
  URL,
  setTimeout,
  console,
  __cargo_test_stdout__,
};

Object.entries(items).forEach(([key, value]) =>
  Object.defineProperty(globalThis, key, {
    value,
    configurable: false,
    writable: false,
  }),
);
