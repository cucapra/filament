hljs.registerLanguage("filament", (hljs) => ({
    name: "filament",
    keywords: {
        keyword: "bundle in import extern for if else new where comp",
    },
    contains: [
        hljs.QUOTE_STRING_MODE,
        {
            className: 'literal',
            scope: 'number',
            begin: '[0-9]+'
        },
        {
            className: 'operator',
            begin: '[-+*<>=]'
        },
        {
            className: 'variable',
            begin: '#[A-Za-z][0-9A-Za-z]*'
        },
        hljs.COMMENT(
            '/\\*', // begin
            '\\*/', // end
            {}
        ),
        hljs.C_LINE_COMMENT_MODE,
    ],
}));

hljs.registerLanguage("verilog", (hljs) => ({
    name: "verilog",
    keywords: {
        keyword: "module endmodule input output"
    },
    contains: [
        {
            className: 'literal',
            scope: 'number',
            begin: '[0-9]+'
        },
    ]
}));

hljs.initHighlightingOnLoad();