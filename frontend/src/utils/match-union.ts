/**
 * Exhaustive pattern matching for single-key object unions.
 * Provides Rust-like match expressions with compile-time exhaustiveness.
 *
 * @example
 * type Event = { NodeAdded: { id: string } } | { NodeDeleted: { id: string } };
 *
 * matchUnion<Event, void>(event, {
 *   NodeAdded: (payload) => console.log('Added:', payload.id),
 *   NodeDeleted: (payload) => console.log('Deleted:', payload.id),
 * });
 */

// Extract all possible keys from a union of single-key objects
export type Tags<U> = U extends unknown ? keyof U : never;

// Extract the variant that has a specific key
export type Variant<U, K extends PropertyKey> = Extract<U, Record<K, unknown>>;

// Extract the payload type for a specific key
export type Payload<U, K extends PropertyKey> = Variant<U, K>[K];

// Handler map: each key must have a handler that receives the payload
export type MatchHandlers<U, R> = {
  [K in Tags<U>]: (payload: Payload<U, K>, whole: Variant<U, K>) => R;
};

/**
 * Match a discriminated union value against handlers.
 * TypeScript ensures all variants are handled at compile time.
 */
export function matchUnion<U, R>(value: U, handlers: MatchHandlers<U, R>): R {
  const handlersObj = handlers as Record<string, (p: unknown, w: unknown) => R>;
  const valueObj = value as Record<string, unknown>;

  for (const k in handlersObj) {
    if (Object.prototype.hasOwnProperty.call(valueObj, k)) {
      const payload = valueObj[k];
      return handlersObj[k](payload, value);
    }
  }
  throw new Error('No matching handler for value');
}

/**
 * Partial match - allows handling only some variants.
 * Unhandled variants return undefined.
 */
export type PartialMatchHandlers<U, R> = {
  [K in Tags<U>]?: (payload: Payload<U, K>, whole: Variant<U, K>) => R;
};

export function matchUnionPartial<U, R>(
  value: U,
  handlers: PartialMatchHandlers<U, R>
): R | undefined {
  const handlersObj = handlers as Record<string, ((p: unknown, w: unknown) => R) | undefined>;
  const valueObj = value as Record<string, unknown>;

  for (const k in handlersObj) {
    if (Object.prototype.hasOwnProperty.call(valueObj, k)) {
      const handler = handlersObj[k];
      if (handler) {
        const payload = valueObj[k];
        return handler(payload, value);
      }
    }
  }
  return undefined;
}
