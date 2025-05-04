import { examplePage } from "../example.ts";

if (Deno.args[0] === "supports") {
  Deno.exit(Deno.args[1] === "html" ? 0 : 1);
}

await import("./build.ts");

const [, book] = await read(Deno.stdin.readable)
  // <br>
  .then((data): [unknown, Book] => JSON.parse(data));

for (const section of book.sections) {
  if ("Chapter" in section) {
    section.Chapter.content = examplePage(section.Chapter.content);
  }
}

console.log(JSON.stringify(book));

async function read(r: ReadableStream): Promise<string> {
  const reader = r.getReader();
  const decoder = new TextDecoder();
  let result = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    result += decoder.decode(value);
  }
  return result;
}

type Book = {
  sections: ({ PartTitle: string } | { Chapter: { content: string } })[];
};
