/** @type {import("eslint").Rule.RuleModule} */
export default {
  meta: {
    type: "problem",
    messages: {
      missingTooltip: "IconButton must be wrapped in a Tooltip.",
    },
  },
  create(context) {
    const isNamed = (element, name) =>
      element?.type === "JSXElement" &&
      element.openingElement?.name?.type === "JSXIdentifier" &&
      element.openingElement?.name?.name === name;

    return {
      JSXOpeningElement(node) {
        if (node.name.type !== "JSXIdentifier" || node.name.name !== "IconButton") return;

        const parent = node.parent.parent;
        if (isNamed(parent, "Tooltip")) return;
        // MUI's documented pattern for disabled buttons: Tooltip > span > IconButton
        // (a disabled button swallows the events the Tooltip listens for, so a span
        // must carry them). The span satisfies the rule only when the Tooltip is
        // directly above it.
        if (isNamed(parent, "span") && isNamed(parent.parent, "Tooltip")) return;

        context.report({ node, messageId: "missingTooltip" });
      },
    };
  },
};
