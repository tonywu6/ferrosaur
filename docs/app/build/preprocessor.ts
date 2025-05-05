import { examplePage } from "../example.ts";

if (Deno.args[0] === "supports") {
  Deno.exit(Deno.args[1] === "html" ? 0 : 1);
}

await import("./build.ts");

const [, book] = await read(Deno.stdin.readable)
  // <br>
  .then((data): [unknown, Book] => JSON.parse(data));

for (const chapter of iterChapters(book.sections)) {
  chapter.content = examplePage(chapter.content);
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

function* iterChapters(sections: Item[]): Generator<Chapter> {
  for (const section of sections) {
    if ("Chapter" in section) {
      yield section.Chapter;
      yield* iterChapters(section.Chapter.sub_items);
    }
  }
}

type Book = {
  sections: Item[];
};

type Item = { PartTitle: string } | { Chapter: { content: string; sub_items: Item[] } };

type Chapter = { content: string; sub_items: Item[] };
