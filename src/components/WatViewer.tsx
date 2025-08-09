import * as React from "react";
import { useState, useEffect, useRef } from "react";
import { Module } from "../Module.js";
import type {
  PrintPart,
  Item,
} from "../../component-built/interfaces/local-module-module.d.ts";

const MaxBytesForRich = 100 * 1024;

function renderRichPrint(parts: PrintPart[]) {
  let root = document.createElement("span");
  let offsets = document.createElement("div");

  let stack = [root];
  for (let part of parts) {
    switch (part.tag) {
      case "str": {
        stack[0].append(part.val);
        break;
      }
      case "new-line": {
        stack[0].append("\n");

        let offset = document.createElement("div");
        offset.innerText = `0x${part.val.toString(16)}`;
        offset.className = "print-newline";
        offset.setAttribute("data-offset", "" + part.val);
        offsets.append(offset);
        break;
      }
      case "name":
      case "literal":
      case "keyword":
      case "type":
      case "comment": {
        let ele = document.createElement("span");
        ele.className = "print-" + part.tag;
        stack[0].append(ele);
        stack.unshift(ele);
        break;
      }
      case "reset": {
        stack.shift();
        break;
      }
      default: {
        // @ts-expect-error the above should be exhaustive
        type remaining = typeof part.tag;
        console.error("got unknown print part", part);
      }
    }
  }

  return [root, offsets];
}

export function WatViewer({
  content,
  item,
  offset,
  setOffset,
  searchXRef,
}: {
  content: Module;
  item: Item | null;
  offset: number | null;
  setOffset: (offset: number) => void;
  searchXRef: (xref: string) => void;
}) {
  let [bodyState, setBodyState] = useState<Promise<PrintPart[]>>(() =>
    Promise.resolve([]),
  );

  useEffect(() => {
    if (!item) {
      return;
    }

    let range = item.range;
    let items = (async () => {
      if (range.end - range.start > MaxBytesForRich) {
        let part: PrintPart = {
          tag: "str",
          val: await content.printPlain(range),
        };
        return [part];
      } else {
        return await content.printRich(range);
      }
    })();
    setBodyState(items);
  }, [content, item]);

  let body = React.use(bodyState);

  let contents = useRef<HTMLPreElement | null>(null);
  let offsets = useRef<HTMLPreElement | null>(null);

  useEffect(() => {
    if (!contents.current || !offsets.current || !body) {
      return;
    }
    let [newContents, newOffsets] = renderRichPrint(body);
    contents.current.innerHTML = "";
    contents.current.appendChild(newContents);
    offsets.current.innerHTML = "";
    offsets.current.appendChild(newOffsets);
  }, [body]);

  useEffect(() => {
    if (!offsets.current) {
      return;
    }

    let targetElement = offsets.current.querySelector(
      `[data-offset="${offset}"]`,
    );
    if (targetElement) {
      targetElement.classList.add("print-newline-selected");
      targetElement.scrollIntoView({ behavior: "smooth", block: "center" });
    }
    return () => {
      targetElement?.classList.remove("print-newline-selected");
    };
  }, [body, offset]);

  let onClickOffsets = (e: React.MouseEvent<HTMLPreElement, MouseEvent>) => {
    if (
      e.target &&
      e.target instanceof HTMLDivElement &&
      e.target.hasAttribute("data-offset")
    ) {
      let offsetString = e.target.getAttribute("data-offset");
      if (offsetString === null) {
        return;
      }
      let offset = parseInt(offsetString);
      if (isNaN(offset)) {
        return;
      }
      setOffset(offset);
    }
  };
  let onClickContents = (e: React.MouseEvent<HTMLPreElement, MouseEvent>) => {
    if (
      e.target &&
      e.target instanceof HTMLSpanElement &&
      e.target.classList.contains("print-name")
    ) {
      let xref = e.target.innerText;

      let xrefIsIndex = !isNaN(parseInt(xref));
      let previousSibling = e.target.previousSibling;

      // Apply heuristics to guess what an integer 'name' refers to
      if (xrefIsIndex && previousSibling) {
        let text = (previousSibling.textContent ?? "").trimEnd();

        let funcPattern = /(call|func)$/;
        let typePattern = /((struct\.\w+)|(array\.\w+)|type)$/;

        if (text.match(funcPattern)) {
          xref = "func " + xref;
        } else if (text.match(typePattern)) {
          xref = "type " + xref;
        }
      } else if (xref.startsWith("$")) {
        xref = xref.substring(1);
      }

      searchXRef(xref);
    }
  };

  if (!item) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        Nothing selected yet.
      </div>
    );
  }
  return (
    <div className="flex-1 overflow-auto bg-white flex min-h-full">
      <pre
        ref={offsets}
        className="wat font-mono text-xs leading-relaxed text-gray-800 whitespace-pre break-words"
        onClick={onClickOffsets}
      ></pre>
      <pre
        ref={contents}
        className="wat flex-1 font-mono text-xs leading-relaxed text-gray-800 whitespace-pre break-words"
        onClick={onClickContents}
      ></pre>
    </div>
  );
}
