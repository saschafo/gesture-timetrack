/**
 * Kleine Farbhelfer für die Projektfarben.
 *
 * Projektfarben wählt der Nutzer frei - eine Schaltfläche in Projektfarbe
 * braucht deshalb eine mitgerechnete Textfarbe, sonst steht irgendwann weißer
 * Text auf Gelb.
 */

/** Zerlegt `#rgb` oder `#rrggbb`. Gibt `null` bei allem anderen zurück. */
export function parseHex(color: string | null | undefined): [number, number, number] | null {
  if (!color) return null
  const value = color.trim()
  const short = /^#([0-9a-f])([0-9a-f])([0-9a-f])$/i.exec(value)
  if (short) {
    return [short[1], short[2], short[3]].map((part) => parseInt(part + part, 16)) as [
      number,
      number,
      number,
    ]
  }
  const long = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(value)
  if (long) {
    return [long[1], long[2], long[3]].map((part) => parseInt(part, 16)) as [number, number, number]
  }
  return null
}

/**
 * Relative Helligkeit nach WCAG - Grundlage für die Entscheidung
 * heller oder dunkler Text.
 */
export function luminance(color: string): number | null {
  const rgb = parseHex(color)
  if (!rgb) return null
  const [r, g, b] = rgb.map((channel) => {
    const value = channel / 255
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

const LIGHT_TEXT = '#ffffff'
const DARK_TEXT = '#14161f'

/** Gut lesbare Textfarbe auf der angegebenen Fläche. */
export function readableTextOn(color: string): string {
  const value = luminance(color)
  if (value === null) return LIGHT_TEXT
  return value > 0.45 ? DARK_TEXT : LIGHT_TEXT
}

/**
 * Stil für eine Schaltfläche in Projektfarbe. `null`, wenn keine verwertbare
 * Farbe vorliegt - dann bleibt es beim Standardanstrich.
 */
export function projectButtonStyle(
  color: string | null | undefined,
): { background: string; borderColor: string; color: string } | null {
  if (!color || !parseHex(color)) return null
  return { background: color, borderColor: color, color: readableTextOn(color) }
}
