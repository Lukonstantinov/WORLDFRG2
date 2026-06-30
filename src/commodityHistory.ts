/** #36 · Real-world commodity history cards. Keyed by good id (matching the Rust
 *  GOOD_NAMES / GoodSpec.id). Pure static content — short, evocative notes on how
 *  each commodity actually shaped real trade, shown in the Goods Codex panel and
 *  the Good Detail panel. Goods without an entry fall back to a generic line. */

export interface CommodityHistory {
  /** Short era / region tag, e.g. "Silk Road · 200 BCE–1400 CE". */
  era: string;
  /** One or two sentences of real-world trade history. */
  note: string;
}

export const COMMODITY_HISTORY: Record<string, CommodityHistory> = {
  silk: {
    era: "Silk Road · 200 BCE–1400 CE",
    note: "Chinese sericulture stayed a guarded secret for centuries; bolts of silk crossed Asia hand to hand at such value that the Romans drained gold to buy it, and whole caravan cities rose to tax the trade.",
  },
  wine: {
    era: "Mediterranean · since 1500 BCE",
    note: "Phoenician and Greek ships carried wine in stamped amphorae the way modern wine carries labels — Falernian and Chian named their vintages, and the cargo doubled as the era's safest drinking water.",
  },
  oliveoil: {
    era: "Classical Mediterranean",
    note: "Olive oil lit lamps, fed bodies, and anointed athletes and kings. Athens built an export economy on it; amphora sherds across the sea floor still trace the old oil routes.",
  },
  sugar: {
    era: "Plantation Atlantic · 1500–1800",
    note: "Once a medieval apothecary's luxury, sugar became the engine of the plantation economy — its appetite drove the darkest currents of the Atlantic trade and made island colonies fabulously rich.",
  },
  spices: {
    era: "Spice Trade · to 1600",
    note: "Pepper, nutmeg and clove were worth their weight in silver; the search for a direct route to the Spice Islands launched the Age of Discovery and turned tiny Banda islets into geopolitical prizes.",
  },
  pepper: {
    era: "Malabar Coast",
    note: "Black pepper — 'black gold' — was demanded as ransom and hoarded as wealth; a sack of it could buy a town's freedom, and Venice grew fat as Europe's pepper gateway.",
  },
  tea: {
    era: "Canton & the Clippers · 1700s–1800s",
    note: "China's tea monopoly bent empires: it bankrolled the East India Company, triggered the Opium Wars, and the race to land the first crop bred the fastest sailing ships ever built.",
  },
  coffee: {
    era: "Mocha to Java · 1500–1800",
    note: "From Yemeni Sufi rituals coffee spread through Ottoman coffeehouses to Europe; nations smuggled live seedlings to break Arabia's monopoly and plant it across their tropical colonies.",
  },
  frankincense: {
    era: "Incense Road · antiquity",
    note: "Tapped from desert trees in Arabia and the Horn, frankincense burned on every altar from Egypt to Rome — the caravan trade in it made Petra and the kingdoms of the south wealthy.",
  },
  incense: {
    era: "Incense Road · antiquity",
    note: "Aromatics for temple and tomb crossed the Arabian deserts by camel; the cities that watered the caravans grew rich taxing every fragrant load.",
  },
  myrrh: {
    era: "Incense Road · antiquity",
    note: "Worth more than gold to embalmers and physicians, myrrh travelled the same desert caravan roads as frankincense and was prized from Egyptian tombs to Roman markets.",
  },
  salt: {
    era: "Universal · all eras",
    note: "The one indispensable preservative: Roman soldiers were paid in it (salarium → 'salary'), Saharan caravans traded it slab-for-gold, and salt taxes toppled and built governments alike.",
  },
  stockfish: {
    era: "North Atlantic · medieval",
    note: "Wind-dried cod kept for years and fed armies, navies and Lent. The Hanseatic League and later Atlantic powers built fortunes on the Lofoten and Newfoundland fisheries.",
  },
  herring: {
    era: "Hanseatic Baltic",
    note: "The 'silver of the sea' underwrote the Hanseatic League; whole cities lived on the salted herring trade, and the fish's migrations could shift the balance of Baltic power.",
  },
  whaling: {
    era: "Basque to Nantucket · 1100–1900",
    note: "Whale oil lit the lamps of the world before petroleum; Basque, Dutch and Yankee whalers chased the trade across every ocean, and a single good voyage could make a crew's fortune.",
  },
  furs: {
    era: "Fur Trade · 1500–1800",
    note: "Beaver felt hats drove exploration of Siberia and North America; the fur companies mapped continents, founded cities, and made the pelt a frontier currency.",
  },
  timber: {
    era: "Naval Stores · all eras",
    note: "Ship-timber and masts were strategic war materiel; powers fought over Baltic forests and tall pines, and a navy without good timber was a navy in decline.",
  },
  amber: {
    era: "Amber Road · antiquity",
    note: "Baltic amber travelled south to the Mediterranean along one of Europe's oldest trade roads, exchanged for bronze and gold a thousand years before Rome.",
  },
  dyes: {
    era: "Tyrian purple & beyond",
    note: "Color was wealth: Tyrian purple from sea-snails was reserved for emperors, and later indigo and cochineal became colonial cash crops fought over by trading empires.",
  },
  pearls: {
    era: "Persian Gulf & Indian Ocean",
    note: "Before cultured pearls, natural pearls from the Gulf and Mannar fisheries were among the densest portable wealth on earth, financing coastal sheikhdoms and tempting every passing empire.",
  },
  cotton: {
    era: "Indian calico to mill towns",
    note: "Indian cotton cloth clothed the medieval world; later the raw fiber fed the mills of the Industrial Revolution and reshaped the economies of three continents.",
  },
  wheat: {
    era: "Granary of empires",
    note: "Grain fleets from Egypt and the Black Sea fed Rome and Constantinople; control of the wheat supply was control of the cities, and a failed harvest meant riots.",
  },
  iron: {
    era: "From bloomery to blast furnace",
    note: "Iron made tools, plows and weapons; regions with ore and fuel — Sweden, the Basque country, the Weald — became arsenals, and the metal underpinned every army and economy.",
  },
  gemstones: {
    era: "Golconda & beyond",
    note: "Diamonds from Golconda and gems from Ceylon were the most concentrated wealth a merchant could carry; they financed wars, sealed alliances, and crowned dynasties.",
  },
  // Manufactured chain goods.
  cloth: {
    era: "Flanders & Florence · medieval",
    note: "Finished woolen and linen cloth was medieval Europe's great manufacture; the looms of Flanders and Florence turned imported wool into the wealth that built banking dynasties.",
  },
  metalware: {
    era: "Guild workshops",
    note: "Worked metal — tools, arms, fine goods — carried the value added by skilled urban guilds, concentrating wealth and craft in the great manufacturing cities.",
  },
};

/** A generic fallback line for goods without a curated card. */
export function commodityHistory(id: string): CommodityHistory | null {
  return COMMODITY_HISTORY[id] ?? null;
}
