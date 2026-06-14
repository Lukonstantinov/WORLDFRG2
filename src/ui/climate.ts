// Shared Köppen-code → human phrase helpers, used by both the hub window
// (settlement character blurb) and the route window (merchant narrative).

/** [short climate name, a real-world city analogue by climate]. */
const KOPPEN: Record<number, [string, string]> = {
  1: ["steamy equatorial rainforest", "Singapore"], 2: ["monsoon-drenched tropics", "Mumbai"],
  3: ["tropical savanna", "Bangkok"], 23: ["dry-season savanna", "Khartoum"],
  4: ["scorching hot desert", "Cairo"], 5: ["cold high desert", "Kashgar"],
  6: ["hot semi-arid steppe", "Baghdad"], 7: ["cool grassland steppe", "Astana"],
  8: ["sun-baked Mediterranean", "Athens"], 9: ["mild Mediterranean", "Lisbon"], 10: ["cool Mediterranean", "Santiago"],
  11: ["humid subtropical", "Shanghai"], 12: ["temperate oceanic", "London"], 13: ["cool subpolar oceanic", "Reykjavík"],
  14: ["warm humid-continental", "Chicago"], 15: ["cold humid-continental", "Moscow"],
  16: ["harsh subarctic", "Yakutsk"], 17: ["bitter subarctic", "Verkhoyansk"],
  18: ["dry-summer continental", "Tehran"], 19: ["dry-summer continental", "Erzurum"], 20: ["dry-summer subarctic", "Ulaanbaatar"],
  21: ["frozen tundra", "Murmansk"], 22: ["polar ice", "an ice-bound shore"],
  24: ["monsoon humid-subtropical", "Guangzhou"], 25: ["mild subtropical highland", "Kunming"], 26: ["cold highland", "Lhasa"],
  27: ["dry-winter continental", "Beijing"], 28: ["dry-winter continental", "Harbin"], 29: ["dry-winter subarctic", "Chita"],
  30: ["dry-winter subarctic", "Oymyakon"], 31: ["dry-summer subarctic", "the cold steppe"], 32: ["alpine highland", "an alpine vale"],
};

/** Short climate name only (e.g. "humid subtropical"). */
export function koppenName(k: number): string {
  return (KOPPEN[k] ?? ["temperate", "a familiar shore"])[0];
}

/** Köppen code → [r,g,b], MIRRORING the backend `render/tile_image.rs
 *  koppen_color` so a good's seeding heatmap is tinted the same colour the
 *  climate layer paints that zone. */
const KOPPEN_RGB: Record<number, [number, number, number]> = {
  1: [0, 0, 255], 2: [0, 120, 255], 3: [70, 170, 250], 4: [255, 0, 0],
  5: [255, 150, 150], 6: [245, 165, 0], 7: [255, 220, 100], 8: [255, 255, 0],
  9: [200, 200, 0], 10: [150, 150, 0], 11: [200, 255, 80], 12: [100, 200, 100],
  13: [50, 150, 50], 14: [0, 255, 150], 15: [55, 200, 255], 16: [0, 125, 125],
  17: [0, 70, 95], 18: [255, 0, 255], 19: [200, 0, 200], 20: [150, 50, 150],
  21: [200, 205, 210], 22: [240, 246, 252], 23: [110, 200, 230], 24: [170, 235, 120],
  25: [120, 205, 120], 26: [80, 165, 95], 27: [150, 130, 225], 28: [120, 100, 205],
  29: [95, 85, 170], 30: [70, 60, 130], 31: [150, 50, 150], 32: [185, 165, 185],
};

export function koppenRgb(k: number): [number, number, number] {
  return KOPPEN_RGB[k] ?? [128, 128, 128];
}

export function koppenHex(k: number): string {
  const [r, g, b] = koppenRgb(k);
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

/** A short climate/terrain phrase + a real-world city analogue, for a settlement. */
export function climatePhrase(k: number, elev: number, coastal: boolean): { clim: string; analogue: string } {
  const hilly = elev > 0.45 ? "mountain " : elev > 0.30 ? "upland " : "";
  const seat = coastal ? "port" : "town";
  const [name, analogue] = KOPPEN[k] ?? ["temperate", "a familiar shore"];
  return { clim: `a ${name} ${hilly}${seat}`, analogue };
}
