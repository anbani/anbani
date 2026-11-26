/*
 * anbani.h: C ABI of anbani.rs (crate anbani-cabi, library libanbani_c). GPL-3.0-or-later.
 *
 * Strings in: NUL-terminated UTF-8, borrowed for the duration of the call.
 * Strings out: heap-allocated NUL-terminated UTF-8 owned by the caller; release each with anbani_free().
 * No function aborts on bad input: anbani_call reports errors in its JSON envelope, the typed shortcuts return NULL.
 */
#ifndef ANBANI_H
#define ANBANI_H

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Generic entry point. id: an anbani_api id ("core.convert", "nlp.georgianise", "numbers.words", ...; the ids and
 * argument shapes of spec/golden*.json). args_json: a JSON array of arguments.
 * Returns {"ok":true,"result":...} or {"ok":false,"error":"..."}.
 */
char *anbani_call(const char *id, const char *args_json);

/* Convert between scripts ("mkhedruli" -> "asomtavruli", schemes like "iso_9984:1996"); NULL on error. */
char *anbani_convert(const char *text, const char *from, const char *to);

/* The script of a text: "mkhedruli", "mtavruli", "asomtavruli", "nuskhuri", "khutsuri", "latin", ..., "unknown". */
char *anbani_classify(const char *text);

/* The library version, e.g. "0.7.0". */
char *anbani_version(void);

/* Release a string returned by this library. NULL is ignored. */
void anbani_free(char *s);

#ifdef __cplusplus
}
#endif

#endif /* ANBANI_H */
