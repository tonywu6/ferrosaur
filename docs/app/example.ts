export function examplePage(page: string): string {
  type State = { state: "fenced"; fence: string; inner: string } | { state: "prose" };

  let output = "";
  let state: State = { state: "prose" };

  for (const line of page.split("\n")) {
    switch (state.state) {
      case "prose": {
        const match = /(`{3,})rs,example/.exec(line);
        if (match) {
          const fence = match[1];
          const inner = "";
          state = { state: "fenced", fence, inner };
        } else {
          output += line;
          output += "\n";
        }
        break;
      }
      case "fenced": {
        if (line === state.fence) {
          output += intoNotebook(state.inner);
          output += "\n";
          state = { state: "prose" };
        } else {
          state.inner += line;
          state.inner += "\n";
        }
      }
    }
  }

  if (state.state === "fenced") {
    output += state.inner;
    output += "\n";
  }

  return output;
}

function intoNotebook(code: string): string {
  const paragraphs: Paragraph[] = [];

  const lastParagraph = (): Paragraph | undefined => paragraphs[paragraphs.length - 1];

  for (const line of code.split("\n")) {
    const last = lastParagraph();
    if (/^\s*[/]{2}(?:[^/!]|$)/.test(line)) {
      switch (last?.type) {
        case "code": {
          const text = last.lines[last.lines.length - 1];
          if (text) {
            last.lines.push(line);
          } else {
            paragraphs.push({ type: "prose", lines: [line] });
          }
          break;
        }
        case "prose":
          last.lines.push(line);
          break;
        default:
          paragraphs.push({ type: "prose", lines: [line] });
          break;
      }
    } else {
      switch (last?.type) {
        case "code":
          last.lines.push(line);
          break;
        case "prose":
          paragraphs.push({ type: "code", lines: [line] });
          break;
      }
    }
  }

  const dedent = (lines: string[]): string[] => {
    const indent = Math.min(
      ...lines.filter(Boolean).map((line) => /^[ /!]*/.exec(line)?.[0].length ?? 0),
    );
    return lines.map((line) => line.slice(indent));
  };

  let output = "";

  for (const para of paragraphs) {
    switch (para.type) {
      case "code": {
        const inner = para.lines.join("\n").trim();
        if (inner) {
          output += "```rs\n";
          output += inner;
          output += "\n```\n\n";
        }
        break;
      }
      case "prose": {
        output += dedent(para.lines).join("\n");
        output += "\n\n";
      }
    }
  }

  return output;
}

type Paragraph = { type: "prose"; lines: string[] } | { type: "code"; lines: string[] };
