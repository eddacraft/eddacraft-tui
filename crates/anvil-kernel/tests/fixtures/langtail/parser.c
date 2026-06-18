/* LANGTAIL T1 fixture — representative C source. */
#include <stdio.h>
#include <stdlib.h>
#include "parser.h"

struct Token {
    int kind;
    const char *text;
};

enum TokenKind {
    TOKEN_EOF,
    TOKEN_IDENT,
    TOKEN_NUMBER
};

typedef struct Token Token;

static int classify(char c) {
    return c >= '0' && c <= '9' ? TOKEN_NUMBER : TOKEN_IDENT;
}

int parse_line(const char *line) {
    if (line == NULL) {
        return -1;
    }
    return classify(line[0]);
}
