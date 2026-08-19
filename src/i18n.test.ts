import assert from 'node:assert/strict'
import test from 'node:test'

import { dictionaries, locale, setLocale, t } from './i18n.ts'

/** Platzhalter einer Zeichenkette, sortiert. */
function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort()
}

test('beide Sprachen kennen dieselben Kennungen', () => {
  const de = Object.keys(dictionaries.de).sort()
  const en = Object.keys(dictionaries.en).sort()

  const missingEn = de.filter((key) => !dictionaries.en[key])
  const missingDe = en.filter((key) => !dictionaries.de[key])
  assert.deepEqual(missingEn, [], 'englische Fassung fehlt')
  assert.deepEqual(missingDe, [], 'deutsche Fassung fehlt')
  assert.deepEqual(de, en)
})

test('keine Übersetzung ist leer', () => {
  for (const [code, dict] of Object.entries(dictionaries)) {
    for (const [key, value] of Object.entries(dict)) {
      assert.ok(value.trim().length > 0, `${code}/${key} ist leer`)
    }
  }
})

test('Platzhalter stimmen zwischen den Sprachen überein', () => {
  for (const key of Object.keys(dictionaries.de)) {
    assert.deepEqual(
      placeholders(dictionaries.de[key]),
      placeholders(dictionaries.en[key]),
      `${key}: Platzhalter unterscheiden sich`,
    )
  }
})

test('übersetzt und setzt Platzhalter ein', () => {
  setLocale('de')
  assert.equal(t('status.start', { name: 'Kunde Meier' }), 'Start · Kunde Meier')
  assert.equal(t('overview.showMore', { count: 20 }), 'Weitere 20 anzeigen')

  setLocale('en')
  assert.equal(t('overview.showMore', { count: 20 }), 'Show 20 more')
  assert.equal(t('overview.title'), 'Reports')

  // Unbekannte Sprache fällt auf Deutsch zurück, unbekannte Kennung auf sich selbst.
  setLocale('fr')
  assert.equal(locale.value, 'de')
  assert.equal(t('gibt.es.nicht'), 'gibt.es.nicht')
})
