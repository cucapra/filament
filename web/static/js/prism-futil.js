Prism.languages.filament = {
    'diff-addition': {
        pattern: /^\+.*$/m
    },
    'diff-deletion': {
        pattern: /^-.*$/m
    },
    'comment': Prism.languages.clike.comment,
    'regex': {
        pattern: /'[a-zA-Z]+/,
        greedy: true
    },
    'string': {
        pattern: /(["])(?:\\(?:\r\n|[\s\S])|(?!\1)[^\\\r\n])*\1/,
        greedy: true
    },
    'class-name': {
        pattern: /(\b(?:comp|new)\s+)\w+/i,
        lookbehind: true
    },
    'namespace': {
        pattern: /\b(?:extern|comp)\b/,
        lookbehind: true,
    },
    'keyword': {
        pattern: /\b(?:new|bundle|for|if)\b/,
        lookbehind: true,
    },
    'number': [
        {
            pattern: /\b[0-9]+'b[0-1]+\b/
        },
        {
            pattern: /\b[0-9]+'d[0-9]+\b/
        },
        {
            pattern: /\b[0-9]+'x[0-9A-Fa-f]+\b/
        },
        {
            pattern: /\b[0-9]+'o[0-7]+\b/
        },
        {
            pattern: /\b(?:[0-9]+)(?!')\b/
        }
    ],
};
