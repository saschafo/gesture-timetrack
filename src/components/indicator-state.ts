/** Zustände des Overlay-Feedbacks (eigene Datei, damit beide Komponenten
 *  denselben Typ importieren können - `<script setup>` kann keine Typen
 *  exportieren). */
export type IndicatorState = 'starting' | 'searching' | 'accepted' | 'rejected'
