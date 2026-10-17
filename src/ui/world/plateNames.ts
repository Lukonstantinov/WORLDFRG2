/** Generated tectonic-plate names. A plate has no stored name, so each one gets a
 *  stable invented name from (world, plate id): the same world always calls the same
 *  plate the same thing, and two plates never share a stem by accident of order. */

const HEAD = ["Ar", "Bel", "Cor", "Dra", "El", "Fen", "Gal", "Hes", "Ir", "Jor", "Kal", "Lun", "Mor", "Nor",
  "Oss", "Pel", "Quen", "Ras", "Sar", "Tor", "Ul", "Val", "Wes", "Xan", "Yor", "Zen", "Ath", "Bry", "Cae", "Dov"];
const MID = ["a", "e", "i", "o", "u", "ia", "en", "or", "an", "el"];
const TAIL = ["dor", "mar", "thos", "vik", "lan", "gar", "ria", "nis", "pen", "ron", "sel", "tal", "vor", "zin", "dal", "mere"];

function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
  return h >>> 0;
}

/** "Belandor Plate" / "Corvik Basin Plate". Oceanic plates read as basins. */
export function plateName(id: number, oceanic: boolean, worldKey: string): string {
  const h = hash(`${worldKey}#plate${id}`);
  const stem = HEAD[h % HEAD.length] + MID[(h >>> 8) % MID.length] + TAIL[(h >>> 16) % TAIL.length];
  return `${stem}${oceanic ? " Basin Plate" : " Plate"}`;
}
