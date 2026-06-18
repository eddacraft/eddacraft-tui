/* LANGTAIL T1 fixture — representative C header (.h → C grammar). */
#ifndef PARSER_H
#define PARSER_H

#include <stddef.h>

struct ParseResult {
    int code;
    size_t length;
};

typedef struct ParseResult ParseResult;

/* Function prototypes — headers often carry only these. */
int parse_line(const char *line);
ParseResult parse_buffer(const char *buf, size_t len);

#endif /* PARSER_H */
