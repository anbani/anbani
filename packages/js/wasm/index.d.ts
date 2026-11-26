export function load(source?: string | URL | Response | BufferSource | WebAssembly.Module): Promise<unknown>;
export const core: {
  /** `verse`: bicameral targets also capitalise the first letter of every line. */
  convert(text: string, from: string, to: string, options?: { verse?: boolean }): string;
  interpret(text: string, to: string): string;
  classify(text: string): string;
};
export const morse: {
  encode(text: string, options?: { profile?: 'mkhedruli' | 'laz_latin' }): string;
  decode(text: string, options?: { profile?: 'mkhedruli' | 'laz_latin' }): string;
};
export function call(id: string, ...args: unknown[]): unknown;
export function version(): string;
declare const _default: { load: typeof load; core: typeof core; morse: typeof morse; call: typeof call; version: typeof version };
export default _default;
