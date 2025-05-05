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

  for (const thisLine of code.split("\n")) {
    const last = lastParagraph();
    if (RE_COMMENT.test(thisLine)) {
      switch (last?.type) {
        case "code": {
          const lastLine = last.lines[last.lines.length - 1];
          if (lastLine) {
            last.lines.push(thisLine);
          } else {
            paragraphs.push({ type: "prose", lines: [thisLine] });
          }
          break;
        }
        case "prose":
          last.lines.push(thisLine);
          break;
        default:
          paragraphs.push({ type: "prose", lines: [thisLine] });
          break;
      }
    } else {
      switch (last?.type) {
        case "code":
          last.lines.push(thisLine);
          break;
        case "prose":
          if (thisLine) {
            const lastLine = last.lines[last.lines.length - 1];
            if (lastLine) {
              paragraphs.pop();
              let code = [...last.lines, thisLine];
              let para: Paragraph | undefined;
              while ((para = lastParagraph())) {
                if (para.type === "code") {
                  paragraphs.pop();
                  code = [...para.lines, ...code];
                } else {
                  break;
                }
              }
              paragraphs.push({ type: "code", lines: code });
            } else {
              paragraphs.push({ type: "code", lines: [thisLine] });
            }
          } else {
            paragraphs.push({ type: "code", lines: [thisLine] });
          }
          break;
        default:
          paragraphs.push({ type: "code", lines: [thisLine] });
          break;
      }
    }
  }

  let output = "";

  for (const para of paragraphs) {
    switch (para.type) {
      case "code": {
        const inner = para.lines
          .join("\n")
          .replace(/^\s*\n/, "")
          .trimEnd();
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
        break;
      }
    }
  }

  return output;
}

type Paragraph = { type: "prose"; lines: string[] } | { type: "code"; lines: string[] };

const RE_COMMENT = /^\s*[/]{2}(?:[^/!]|$)/;

const dedent = (lines: string[]): string[] => {
  const indent = Math.min(
    ...lines.filter(Boolean).map((line) => /^[ /!]*/.exec(line)?.[0].length ?? 0),
  );
  return lines.map((line) => line.slice(indent));
};
