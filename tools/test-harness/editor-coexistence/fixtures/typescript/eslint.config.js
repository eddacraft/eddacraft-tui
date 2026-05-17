// Minimal flat config — no plugins, only stock JS parsing. Lets us lint
// `index.js` without dragging typescript-eslint into the harness install.
export default [
  {
    files: ["index.js"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
    },
    rules: {},
  },
];
