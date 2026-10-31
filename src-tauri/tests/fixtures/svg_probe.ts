import { markdownToHtml } from "../../../src/text/markdownPipeline";
import { prepareEntryImages } from "../../../src/text/prepareEntryImages";
import {
  buildEntryHtml,
  FIND_IN_PAGE,
  FIND_RESULT,
} from "../../../src/text/buildEntryHtml";

// This bootstrap is a native initialization script, not a page script granted a
// CSP exception. Attack scripts arrive in the parsed page or SVG resources.
const state = window as unknown as {
  __svgProbe: {
    label: string;
    base: string;
    attacks: string;
    payload: string;
    image: string;
  };
  __svgExecuted: string[];
};
const config = state.__svgProbe;
state.__svgExecuted = [];
const violations: string[] = [];
document.addEventListener("securitypolicyviolation", (event) => {
  violations.push(`${event.effectiveDirective}: ${event.blockedURI}`);
});
window.addEventListener("message", (event) => {
  if (event.data?.type === "svg-attack")
    state.__svgExecuted.push("reader-script");
});

window.addEventListener("load", async () => {
  try {
    document.querySelector("svg")?.dispatchEvent(new Event("load"));
    const data = `data:image/svg+xml;base64,${btoa(config.image)}`;
    const image = new Image();
    const loaded = new Promise<boolean>((resolve) => {
      image.onload = () => resolve(true);
      image.onerror = () => resolve(false);
    });
    image.src = data;
    document.body.append(image);
    const background = document.createElement("div");
    background.style.cssText = `width:32px;height:32px;background-image:url('${data}')`;
    document.body.append(background);
    const sanitized = await markdownToHtml(config.payload);
    const prepared = await prepareEntryImages(
      sanitized,
      "probe",
      async () => []
    );
    const frame = document.createElement("iframe");
    frame.setAttribute("sandbox", "allow-scripts");
    const html = buildEntryHtml(`${prepared}<p>SVG probe marker</p>`, "SVG probe", {
      themeMode: "light",
      fontFamily: "system",
      fontSize: 16,
      lineHeight: 1.5,
    });
    const blob = URL.createObjectURL(new Blob([html], { type: "text/html" }));
    let readerReady = false;
    window.addEventListener("message", (event) => {
      if (
        event.source === frame.contentWindow &&
        event.data?.type === FIND_RESULT
      )
        readerReady = true;
    });
    frame.onload = () =>
      frame.contentWindow?.postMessage(
        { type: FIND_IN_PAGE, query: "SVG probe marker" },
        "*"
      );
    frame.src = blob;
    document.body.append(frame);
    const imageLoaded = await Promise.race([
      loaded,
      new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 3000)),
    ]);
    await new Promise((resolve) => setTimeout(resolve, 3000));
    URL.revokeObjectURL(blob);
    const parsed = new DOMParser().parseFromString(prepared, "text/html");
    const activeMarkup = !!parsed.querySelector("script,[onload],[onerror]");
    document.title =
      "SVG-RESULT:" +
      JSON.stringify({
        label: config.label,
        executed: state.__svgExecuted,
        violations: [...new Set(violations.map((v) => v.split(": ")[0]))],
        imageLoaded,
        readerReady,
        activeMarkup,
        retainedSvgReferences: parsed.querySelectorAll(
          "svg [href],svg [xlink\\:href]"
        ).length,
      });
  } catch (error) {
    document.title = "SVG-ERROR:" + String(error);
  }
});
