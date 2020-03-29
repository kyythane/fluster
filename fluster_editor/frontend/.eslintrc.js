module.exports = {
    root: true,
    parser: "@typescript-eslint/parser",
    plugins: [
        "@typescript-eslint",
    ],
    extends: [
        "plugin:prettier/recommended",
        "plugin:@typescript-eslint/eslint-recommended",
        "plugin:@typescript-eslint/recommended",
    ],
    parserOptions: {
        ecmaVersion: 8,
        ecmaFeatures: {
            impliedStrict: true,
            jsx: true
        }
    }
}