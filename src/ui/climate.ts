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

/** A short climate/terrain phrase + a real-world city analogue, for a settlement. */
export function climatePhrase(k: number, elev: number, coastal: boolean): { clim: string; analogue: string } {
  const hilly = elev > 0.45 ? "mountain " : elev > 0.30 ? "upland " : "";
  const seat = coastal ? "port" : "town";
  const [name, analogue] = KOPPEN[k] ?? ["temperate", "a familiar shore"];
  return { clim: `a ${name} ${hilly}${seat}`, analogue };
}
