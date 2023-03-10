hljs.registerLanguage("filament", (hljs) => ({
    name: "filament",
    keywords: {
        keyword: "in import extern for if else comp new where",
    },
    contains: [
        hljs.QUOTE_STRING_MODE,
        hljs.C_NUMBER_MODE,
        hljs.COMMENT(
            '/\\*', // begin
            '\\*/', // end
            {}
        ),
        hljs.COMMENT(
            '//', // begin
            '$',  // end
            {}
        ),
    ],
}));

hljs.initHighlightingOnLoad();