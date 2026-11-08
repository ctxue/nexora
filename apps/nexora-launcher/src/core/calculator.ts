export interface SmartResult {
  value: string;
  title: string;
  subtitle: string;
  copyText: string;
}

type ArithmeticToken =
  | { type: "number"; value: number }
  | { type: "operator"; value: string }
  | { type: "left" }
  | { type: "right" };

class ArithmeticError extends Error {}

function tokenizeArithmetic(source: string): ArithmeticToken[] {
  const tokens: ArithmeticToken[] = [];
  let index = 0;

  while (index < source.length) {
    const character = source[index];
    if (/\s/u.test(character)) {
      index += 1;
      continue;
    }

    if (/[\d.]/u.test(character)) {
      const match = /^(?:\d+(?:\.\d*)?|\.\d+)/u.exec(source.slice(index));
      if (!match) throw new ArithmeticError("Invalid number");
      const value = Number(match[0]);
      if (!Number.isFinite(value)) throw new ArithmeticError("Invalid number");
      tokens.push({ type: "number", value });
      index += match[0].length;
      continue;
    }

    if ("+-*/%^".includes(character)) {
      tokens.push({ type: "operator", value: character });
      index += 1;
      continue;
    }

    if (character === "(") {
      tokens.push({ type: "left" });
      index += 1;
      continue;
    }

    if (character === ")") {
      tokens.push({ type: "right" });
      index += 1;
      continue;
    }

    throw new ArithmeticError("Unsupported character");
  }

  return tokens;
}

function evaluateArithmetic(source: string): number | null {
  const tokens = tokenizeArithmetic(source);
  let index = 0;

  const peek = () => tokens[index];
  const consume = () => tokens[index++];

  const parsePrimary = (): number => {
    const token = consume();
    if (!token) throw new ArithmeticError("Unexpected end");
    if (token.type === "number") return token.value;
    if (token.type === "operator" && (token.value === "+" || token.value === "-")) {
      const value = parsePrimary();
      return token.value === "-" ? -value : value;
    }
    if (token.type === "left") {
      const value = parseExpression();
      const close = consume();
      if (close?.type !== "right") throw new ArithmeticError("Missing parenthesis");
      return value;
    }
    throw new ArithmeticError("Expected a number");
  };

  const parsePower = (): number => {
    let value = parsePrimary();
    const token = peek();
    if (token?.type === "operator" && token.value === "^") {
      consume();
      value **= parsePower();
    }
    return value;
  };

  const parseTerm = (): number => {
    let value = parsePower();
    while (true) {
      const token = peek();
      if (
        token?.type !== "operator" ||
        !["*", "/", "%"].includes(token.value)
      ) {
        return value;
      }
      consume();
      const right = parsePower();
      if ((token.value === "/" || token.value === "%") && right === 0) {
        throw new ArithmeticError("Division by zero");
      }
      if (token.value === "*") value *= right;
      if (token.value === "/") value /= right;
      if (token.value === "%") value %= right;
    }
  };

  const parseExpression = (): number => {
    let value = parseTerm();
    while (true) {
      const token = peek();
      if (
        token?.type !== "operator" ||
        !["+", "-"].includes(token.value)
      ) {
        return value;
      }
      consume();
      const right = parseTerm();
      value = token.value === "+" ? value + right : value - right;
    }
  };

  try {
    if (tokens.length === 0) return null;
    const value = parseExpression();
    if (index !== tokens.length || !Number.isFinite(value)) return null;
    return Object.is(value, -0) ? 0 : value;
  } catch {
    return null;
  }
}

function formatNumber(value: number, maximumFractionDigits = 12): string {
  return new Intl.NumberFormat("en-US", {
    maximumFractionDigits,
    useGrouping: true,
  }).format(value);
}

const CURRENCY_UNITS: Record<string, { name: string; ratePerUsd: number }> = {
  usd: { name: "USD", ratePerUsd: 1 },
  cny: { name: "CNY", ratePerUsd: 7.24 },
  eur: { name: "EUR", ratePerUsd: 0.92 },
  gbp: { name: "GBP", ratePerUsd: 0.79 },
  jpy: { name: "JPY", ratePerUsd: 151 },
  hkd: { name: "HKD", ratePerUsd: 7.82 },
};

interface LinearUnit {
  symbol: string;
  factorToBase: number;
}

const LINEAR_UNITS: Record<string, LinearUnit> = {
  m: { symbol: "m", factorToBase: 1 },
  km: { symbol: "km", factorToBase: 1000 },
  cm: { symbol: "cm", factorToBase: 0.01 },
  mm: { symbol: "mm", factorToBase: 0.001 },
  mi: { symbol: "mi", factorToBase: 1609.344 },
  ft: { symbol: "ft", factorToBase: 0.3048 },
  in: { symbol: "in", factorToBase: 0.0254 },
  kg: { symbol: "kg", factorToBase: 1 },
  g: { symbol: "g", factorToBase: 0.001 },
  lb: { symbol: "lb", factorToBase: 0.45359237 },
  oz: { symbol: "oz", factorToBase: 0.028349523125 },
  l: { symbol: "L", factorToBase: 1 },
  ml: { symbol: "mL", factorToBase: 0.001 },
  b: { symbol: "B", factorToBase: 1 },
  kb: { symbol: "KB", factorToBase: 1024 },
  mb: { symbol: "MB", factorToBase: 1024 ** 2 },
  gb: { symbol: "GB", factorToBase: 1024 ** 3 },
  tb: { symbol: "TB", factorToBase: 1024 ** 4 },
};

const TEMPERATURE_UNITS = new Set(["c", "f", "k"]);

function convertTemperature(value: number, from: string, to: string): number {
  const celsius =
    from === "c" ? value : from === "f" ? ((value - 32) * 5) / 9 : value - 273.15;
  if (to === "c") return celsius;
  if (to === "f") return (celsius * 9) / 5 + 32;
  return celsius + 273.15;
}

function evaluateConversion(source: string): SmartResult | null {
  const match =
    /^([+-]?(?:\d+(?:\.\d*)?|\.\d+))\s*([a-z]+)\s+(?:to\s+|in\s+|->\s*)?([a-z]+)$/iu.exec(
      source.trim(),
    );
  if (!match) return null;

  const amount = Number(match[1]);
  const fromKey = match[2].toLowerCase();
  const toKey = match[3].toLowerCase();
  if (!Number.isFinite(amount)) return null;

  const fromCurrency = CURRENCY_UNITS[fromKey];
  const toCurrency = CURRENCY_UNITS[toKey];
  if (fromCurrency && toCurrency) {
    const converted = (amount / fromCurrency.ratePerUsd) * toCurrency.ratePerUsd;
    const value = `${formatNumber(converted, 2)} ${toCurrency.name}`;
    return {
      value,
      title: value,
      subtitle: "Currency conversion · offline reference rate",
      copyText: String(converted),
    };
  }

  if (TEMPERATURE_UNITS.has(fromKey) && TEMPERATURE_UNITS.has(toKey)) {
    const converted = convertTemperature(amount, fromKey, toKey);
    const symbol = toKey === "c" ? "°C" : toKey === "f" ? "°F" : "K";
    const value = `${formatNumber(converted, 6)} ${symbol}`;
    return {
      value,
      title: value,
      subtitle: "Unit conversion · temperature",
      copyText: String(converted),
    };
  }

  const from = LINEAR_UNITS[fromKey];
  const to = LINEAR_UNITS[toKey];
  if (!from || !to) return null;

  // Only convert within compatible dimensions.
  const length = new Set(["m", "km", "cm", "mm", "mi", "ft", "in"]);
  const mass = new Set(["kg", "g", "lb", "oz"]);
  const volume = new Set(["l", "ml"]);
  const data = new Set(["b", "kb", "mb", "gb", "tb"]);
  const compatible = [length, mass, volume, data].some(
    (units) => units.has(fromKey) && units.has(toKey),
  );
  if (!compatible) return null;

  const converted = (amount * from.factorToBase) / to.factorToBase;
  const value = `${formatNumber(converted, 10)} ${to.symbol}`;
  return {
    value,
    title: value,
    subtitle: "Unit conversion",
    copyText: String(converted),
  };
}

function evaluateTime(source: string): SmartResult | null {
  const match =
    /^(\d{1,2}):(\d{2})\s*([+-])\s*(\d+(?:\.\d+)?)\s*(h|hr|hour|hours|m|min|minute|minutes)$/iu.exec(
      source.trim(),
    );
  if (!match) return null;

  const hour = Number(match[1]);
  const minute = Number(match[2]);
  const duration = Number(match[4]);
  if (hour > 23 || minute > 59 || !Number.isFinite(duration)) return null;

  const unit = match[5].toLowerCase();
  const minutes = unit.startsWith("h") ? duration * 60 : duration;
  const signedMinutes = match[3] === "-" ? -minutes : minutes;
  const total =
    ((hour * 60 + minute + signedMinutes) % (24 * 60) + 24 * 60) % (24 * 60);
  const roundedTotal = Math.round(total);
  const result = `${String(Math.floor(roundedTotal / 60) % 24).padStart(
    2,
    "0",
  )}:${String(roundedTotal % 60).padStart(2, "0")}`;

  return {
    value: result,
    title: result,
    subtitle: "Time calculation",
    copyText: result,
  };
}

/** Returns a result only for input that is unambiguously a calculator/unit query. */
export function evaluateSmartQuery(source: string): SmartResult | null {
  const trimmed = source.trim();
  if (!trimmed) return null;

  const timed = evaluateTime(trimmed);
  if (timed) return timed;

  const converted = evaluateConversion(trimmed);
  if (converted) return converted;

  if (!/[+\-*/%^()]/u.test(trimmed) || !/\d/u.test(trimmed)) return null;
  const arithmetic = evaluateArithmetic(trimmed.replaceAll(",", ""));
  if (arithmetic === null) return null;

  const value = formatNumber(arithmetic);
  return {
    value,
    title: value,
    subtitle: "Calculation · Enter to copy",
    copyText: String(arithmetic),
  };
}
