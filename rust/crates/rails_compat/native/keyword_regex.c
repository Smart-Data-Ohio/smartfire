/* Rust-owned narrow bridge to the pinned, standalone Ruby Onigmo engine. */
#include "onigmo.h"
int campfire_keyword_regex_init(void) { return onig_init(); }
void *campfire_keyword_regex_new(const unsigned char *pattern, size_t length, int *error) {
    OnigRegex regex = NULL;
    *error = onig_new(&regex, pattern, pattern + length, ONIG_OPTION_IGNORECASE,
                      ONIG_ENCODING_UTF_8, ONIG_SYNTAX_RUBY, NULL);
    return regex;
}
int campfire_keyword_regex_match(void *regex, const unsigned char *text, size_t length) {
    OnigPosition result = onig_search(regex, text, text + length, text, text + length,
                                     NULL, ONIG_OPTION_NONE);
    return result >= 0 ? 1 : (int)result;
}
void campfire_keyword_regex_free(void *regex) { onig_free(regex); }
