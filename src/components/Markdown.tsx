import React from "react";

// A small, self-contained Markdown renderer — enough of the syntax the model
// actually emits (headings, bold/italic, inline code, bullet & numbered lists,
// blockquotes, fenced code) to turn a wall of text into something skimmable.
// Deliberately dependency-free and built from React elements (never
// dangerouslySetInnerHTML), so there's no injection surface from model output.

// Inline spans: links, **bold**, `code`, *italic* / _italic_.
function inline(text: string, keyBase: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  const re = /\[([^\]]+)\]\(([^)]+)\)|\*\*([^*]+)\*\*|`([^`]+)`|\*([^*]+)\*|_([^_]+)_/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let i = 0;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) nodes.push(text.slice(last, m.index));
    const key = `${keyBase}-${i}`;
    if (m[1] !== undefined) {
      // Link: show the text, keep the URL only as a tooltip — never navigable,
      // so model output can't send the user anywhere.
      nodes.push(<span className="md-link" title={m[2]} key={key}>{m[1]}</span>);
    } else if (m[3] !== undefined) {
      nodes.push(<strong key={key}>{m[3]}</strong>);
    } else if (m[4] !== undefined) {
      nodes.push(<code key={key}>{m[4]}</code>);
    } else if (m[5] !== undefined) {
      nodes.push(<em key={key}>{m[5]}</em>);
    } else if (m[6] !== undefined) {
      nodes.push(<em key={key}>{m[6]}</em>);
    }
    last = m.index + m[0].length;
    i++;
  }
  if (last < text.length) nodes.push(text.slice(last));
  return nodes;
}

export default function Markdown({ text }: { text: string }) {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const blocks: React.ReactNode[] = [];
  let para: string[] = [];
  let i = 0;

  const flushPara = () => {
    if (para.length) {
      blocks.push(<p key={`p-${blocks.length}`}>{inline(para.join(" "), `p${blocks.length}`)}</p>);
      para = [];
    }
  };

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (trimmed === "") {
      flushPara();
      i++;
      continue;
    }

    // Fenced code block ```
    if (trimmed.startsWith("```")) {
      flushPara();
      i++;
      const code: string[] = [];
      while (i < lines.length && !lines[i].trim().startsWith("```")) {
        code.push(lines[i]);
        i++;
      }
      i++; // consume closing fence
      blocks.push(
        <pre className="md-pre" key={`pre-${blocks.length}`}>
          <code>{code.join("\n")}</code>
        </pre>,
      );
      continue;
    }

    // Headings # .. ####  → h3..h6 (kept modest so they don't shout)
    const h = /^(#{1,4})\s+(.*)$/.exec(trimmed);
    if (h) {
      flushPara();
      const tag = `h${Math.min(h[1].length + 2, 6)}`;
      blocks.push(
        React.createElement(tag, { key: `h-${blocks.length}`, className: "md-h" }, inline(h[2], `h${blocks.length}`)),
      );
      i++;
      continue;
    }

    // Unordered list (-, *, •)
    if (/^[-*•]\s+/.test(trimmed)) {
      flushPara();
      const items: string[] = [];
      while (i < lines.length && /^\s*[-*•]\s+/.test(lines[i])) {
        items.push(lines[i].trim().replace(/^[-*•]\s+/, ""));
        i++;
      }
      blocks.push(
        <ul className="md-ul" key={`ul-${blocks.length}`}>
          {items.map((it, k) => <li key={k}>{inline(it, `uli-${blocks.length}-${k}`)}</li>)}
        </ul>,
      );
      continue;
    }

    // Ordered list (1. 2. 3.)
    if (/^\d+\.\s+/.test(trimmed)) {
      flushPara();
      const items: string[] = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].trim().replace(/^\d+\.\s+/, ""));
        i++;
      }
      blocks.push(
        <ol className="md-ol" key={`ol-${blocks.length}`}>
          {items.map((it, k) => <li key={k}>{inline(it, `oli-${blocks.length}-${k}`)}</li>)}
        </ol>,
      );
      continue;
    }

    // Blockquote
    if (/^>\s?/.test(trimmed)) {
      flushPara();
      const quote: string[] = [];
      while (i < lines.length && /^\s*>\s?/.test(lines[i])) {
        quote.push(lines[i].trim().replace(/^>\s?/, ""));
        i++;
      }
      blocks.push(
        <blockquote className="md-quote" key={`bq-${blocks.length}`}>{inline(quote.join(" "), `bq-${blocks.length}`)}</blockquote>,
      );
      continue;
    }

    // Plain text — accumulate into the current paragraph.
    para.push(trimmed);
    i++;
  }
  flushPara();

  return <div className="md">{blocks}</div>;
}
