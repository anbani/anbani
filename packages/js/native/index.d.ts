/** Independent opt-in Rust bindings; library failures throw Error. */
/** `verse`: bicameral targets also capitalise the first letter of every line. */
export function convert(text: string, from: string, to: string, options?: { verse?: boolean }): string;
export function interpret(text: string, to: string): string;
export function classify(text: string): string;
/** The default mode is balanced. */
export function georgianise(text: string, mode?: string): string;
export function latinise(text: string): string;
/** Profiles: clean (default), search, modern. */
export function normalize(text: string, profile?: string): string;
/** Kartvelian Morse; profile mkhedruli (default) or laz_latin. */
export function morseEncode(text: string, profile?: string): string;
export function morseDecode(text: string, profile?: string): string;
/** Dispatch an API operation with JSON-compatible arguments. */
export function call(id: string, args: unknown[]): unknown;
export function ids(): string[];
export function version(): string;
