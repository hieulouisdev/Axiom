import js from "@eslint/js";
import tseslint from "typescript-eslint";

// Aegis AI — ESLint 10 flat config.
// Keeps the signal high and the noise low: recommended sets only, with
// pragmatic relaxations for a Tauri codebase that talks to untyped JS edges.
export default tseslint.config(
  {
    ignores: ["dist/**", "node_modules/**", "src-tauri/**"],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "no-empty": "off",
    },
  },
);
