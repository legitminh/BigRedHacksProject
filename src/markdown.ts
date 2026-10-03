import { Marked } from "marked";
import DOMPurify from "dompurify";
import katex from "katex";

/** Treat model output as untrusted, even when it quotes a connected document. */
export function renderMarkdown(host: HTMLElement, text: string): void {
  const equations = new Map<string, { source: string; display: boolean }>();
  const marked = new Marked({ extensions: [{
    name: "latex",
    level: "inline",
    start: source => source.search(/\$|\\[\[(]/),
    tokenizer(source) {
      const patterns = [
        { regex: /^\$\$([\s\S]+?)\$\$/, display: true },
        { regex: /^\\\[([\s\S]+?)\\\]/, display: true },
        { regex: /^\\\(([\s\S]+?)\\\)/, display: false },
        { regex: /^\$(?!\s)([^$\n]*?[^\s\\])\$(?!\d)/, display: false },
      ];
      for (const { regex, display } of patterns) {
        const match = regex.exec(source);
        if (match) return { type: "latex", raw: match[0], source: match[1], display };
      }
      return undefined;
    },
    renderer(token) {
      const id = crypto.randomUUID();
      equations.set(id, { source: token.source, display: token.display });
      return `<span data-equation="${id}"></span>`;
    },
  }] });
  host.innerHTML = DOMPurify.sanitize(marked.parse(text, { async: false, gfm: true }), {
    ALLOWED_TAGS: ["span", "p", "br", "hr", "h1", "h2", "h3", "h4", "h5", "h6", "strong", "em", "del", "s", "blockquote", "ul", "ol", "li", "pre", "code", "table", "thead", "tbody", "tr", "th", "td", "a"],
    ALLOWED_ATTR: ["href", "title", "start", "data-equation"],
    ALLOW_DATA_ATTR: false,
    ALLOW_ARIA_ATTR: false,
  });
  host.querySelectorAll<HTMLElement>("[data-equation]").forEach((element) => {
    const equation = equations.get(element.dataset.equation || "");
    element.removeAttribute("data-equation");
    if (!equation) return;
    // Generate math only after sanitization; never enable trusted HTML commands.
    katex.render(equation.source, element, {
      displayMode: equation.display, throwOnError: false, trust: false,
      maxExpand: 500, maxSize: 20, strict: "ignore",
    });
  });
  host.querySelectorAll("a").forEach((link) => {
    const href = link.getAttribute("href") || "";
    if (!/^https?:\/\//i.test(href)) {
      link.removeAttribute("href");
    } else {
      link.rel = "noopener noreferrer";
      link.target = "_blank";
    }
  });
  host.querySelectorAll("table").forEach((table) => {
    const wrap = document.createElement("div");
    wrap.className = "table-scroll";
    wrap.tabIndex = 0;
    wrap.setAttribute("role", "region");
    wrap.setAttribute("aria-label", "Table; scroll horizontally if needed");
    table.replaceWith(wrap);
    wrap.append(table);
  });
}
