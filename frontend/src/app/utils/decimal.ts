/**
 * Exact arithmetic on signed decimal strings (e.g. "-12.50"). Monetary values
 * are never parsed into floats anywhere in the app.
 */

/** Adds two signed decimal strings exactly, without ever using floats. */
export function addDecimals(a: string, b: string): string {
  const parts = (value: string) => {
    const negative = value.startsWith('-');
    const [int = '0', frac = ''] = (negative ? value.slice(1) : value).split('.');
    return { negative, int, frac };
  };
  const x = parts(a);
  const y = parts(b);
  const scale = Math.max(x.frac.length, y.frac.length);
  const toScaledInt = (p: { negative: boolean; int: string; frac: string }) =>
    BigInt(p.int + p.frac.padEnd(scale, '0')) * (p.negative ? -1n : 1n);
  const sum = toScaledInt(x) + toScaledInt(y);
  const negative = sum < 0n;
  const digits = (negative ? -sum : sum).toString().padStart(scale + 1, '0');
  const body = scale ? `${digits.slice(0, -scale)}.${digits.slice(-scale)}` : digits;
  return (negative ? '-' : '') + body;
}

/** Flips the sign of a decimal string. */
export function negateDecimal(value: string): string {
  return value.startsWith('-') ? value.slice(1) : `-${value}`;
}

/** Absolute value of a decimal string. */
export function absDecimal(value: string): string {
  return value.startsWith('-') ? value.slice(1) : value;
}

/** Sign of a decimal string: -1, 0, or 1. "0.00" is 0. */
export function decimalSign(value: string): -1 | 0 | 1 {
  if (!/[1-9]/.test(value)) {
    return 0;
  }
  return value.startsWith('-') ? -1 : 1;
}

/**
 * Normalizes user input into a positive decimal string, or `null` when the
 * input is not a positive amount. Validated without ever parsing a float.
 */
export function parseAmountInput(value: string): string | null {
  const trimmed = value.trim();
  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    return null;
  }
  if (!/[1-9]/.test(trimmed)) {
    return null; // zero is not a valid amount
  }
  return trimmed;
}

/** Pads a decimal string to at least two fraction digits for display. */
export function formatAmount(value: string): string {
  const negative = value.startsWith('-');
  const body = negative ? value.slice(1) : value;
  const dot = body.indexOf('.');
  const sign = negative ? '-' : '';
  if (dot === -1) {
    return `${sign}${body}.00`;
  }
  return `${sign}${body.slice(0, dot)}.${body.slice(dot + 1).padEnd(2, '0')}`;
}
