/* Utility library (C) — part of multi-language build example. */

#include <string.h>
#include <ctype.h>

/* Convert string to uppercase in-place. */
void to_upper(char *s) {
    while (*s) {
        *s = toupper((unsigned char)*s);
        s++;
    }
}

/* Reverse string in-place. */
void str_reverse(char *s) {
    size_t len = strlen(s);
    for (size_t i = 0; i < len / 2; i++) {
        char tmp = s[i];
        s[i] = s[len - 1 - i];
        s[len - 1 - i] = tmp;
    }
}

/* Count occurrences of character in string. */
int count_char(const char *s, char c) {
    int count = 0;
    while (*s) {
        if (*s == c) count++;
        s++;
    }
    return count;
}
