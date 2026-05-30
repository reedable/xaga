/** @type {import("eslint").Rule.RuleModule} */
export default {
  meta: {
    type: "problem",
    messages: {
      missingTooltip: "IconButton must be wrapped in a Tooltip.",
    },
  },
  create(context) {
    return {
      JSXOpeningElement(node) {
        if (node.name.type !== "JSXIdentifier" || node.name.name !== "IconButton") return;

        const parent = node.parent.parent;
        if (
          parent?.type === "JSXElement" &&
          parent.openingElement?.name?.type === "JSXIdentifier" &&
          parent.openingElement?.name?.name === "Tooltip"
        ) return;

        context.report({ node, messageId: "missingTooltip" });
      },
    };
  },
};
