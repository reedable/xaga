import path from "node:path";

// TypeScript currently uses relative imports (no path aliases). Resolve before
// checking so traversal and explicit extensions cannot bypass the boundary.
function internalPath(filename) {
  const normalized = filename.split(path.sep).join("/");
  return (
    normalized.includes("/src/platform/native/") ||
    normalized.endsWith("/src/platform/native")
  );
}

export default {
  meta: {
    type: "problem",
    messages: {
      boundary:
        "Native payload internals are accessible only to the native adapter and its tests.",
      reexport:
        "Do not re-export native payload internals outside their directory.",
    },
  },
  create(context) {
    const filename = path.resolve(context.filename ?? context.getFilename());
    const normalized = filename.split(path.sep).join("/");
    const internal = internalPath(filename);
    const allowed =
      internal ||
      /\/src\/platform\/TauriPlatform(?:\.test)?\.ts$/.test(normalized);
    const imported = new Set();
    function resolvesInternal(source) {
      const value =
        source?.value ??
        (source?.type === "TemplateLiteral" && source.expressions.length === 0
          ? source.quasis[0].value.cooked
          : undefined);
      if (typeof value !== "string") return false;
      if (!value.startsWith(".") && !path.isAbsolute(value)) return false;
      return internalPath(path.resolve(path.dirname(filename), value));
    }
    function check(node, source, reexport = false) {
      if (!resolvesInternal(source)) return;
      if (!allowed || (reexport && !internal)) {
        context.report({ node, messageId: reexport ? "reexport" : "boundary" });
      }
    }
    return {
      Program(node) {
        // Imports are hoisted: an export may appear before its import in source.
        for (const statement of node.body) {
          if (
            statement.type === "ImportDeclaration" &&
            resolvesInternal(statement.source)
          ) {
            for (const specifier of statement.specifiers)
              imported.add(specifier.local.name);
          }
        }
      },
      ImportDeclaration(node) {
        check(node, node.source);
        if (resolvesInternal(node.source)) {
          for (const specifier of node.specifiers)
            imported.add(specifier.local.name);
        }
      },
      ImportExpression(node) {
        check(node, node.source);
      },
      TSImportType(node) {
        check(node, node.source ?? node.argument ?? node.parameter);
      },
      ExportAllDeclaration(node) {
        check(node, node.source, true);
      },
      ExportNamedDeclaration(node) {
        if (node.source) check(node, node.source, true);
        else if (!internal) {
          for (const specifier of node.specifiers) {
            if (imported.has(specifier.local.name))
              context.report({ node: specifier, messageId: "reexport" });
          }
        }
      },
      ExportDefaultDeclaration(node) {
        if (!internal && imported.has(node.declaration.name))
          context.report({ node, messageId: "reexport" });
      },
    };
  },
};
