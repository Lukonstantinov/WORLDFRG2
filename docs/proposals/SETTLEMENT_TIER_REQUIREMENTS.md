# Settlement Development Tiers — what a city needs (so you can check)

Tiers measure **advancement / sophistication, not size** — a compact, institutionally
deep city-state can outrank a bigger but shallow town. Population is only a **soft floor**.
Each tier = a population floor + a small **required core** + **"at least N of M" supporting
milestones**, so there are several *paths* to a tier and no single rare thing hard-blocks it.

A city holds the **highest tier whose conditions it meets**. (Yearly + hysteresis wrapping
comes next; today the classifier evaluates current state.)

---

## 1 · Outpost
Any living settlement that hasn't reached Market yet. *(A bare founding hamlet.)*

## 2 · Market
**Population ≥ 700**, **and any ONE of:**
- has some trade (traded at all in the last year), **or**
- has a warehouse (at least a Depot), **or**
- is ranked a trade hub.

*Check in-app: the town has ≥700 people and shows any trade or a depot.*

## 3 · Guild Town
**Population ≥ 2,000**, **AND** it has a **government** (a council / officials seated),
**AND at least 2 of these 4:**
- a **guild** seated in the town,
- a **warehouse** (Depot or larger),
- at least **1 civic building**,
- ranked a **trade hub**.

*Check: ≥2,000 people, it has a government, and any two of {guild, warehouse, a civic
building, trade-hub status}.*

## 4 · Free City
**Population ≥ 7,000**, **AND** it is a **trade hub** (`hub_class` ≥ 1), **AND** it has
**finance** (a mint, its own coin, or a bank stake), **AND at least 2 of these 5:**
- a **Warehouse-grade** depot (tier 3+),
- **2+ civic buildings**,
- a **guild**,
- **written laws**,
- **stable** (content + low unrest).

*Check: ≥7,000 people, it's a trade hub, it has a bank/mint/own coin, plus any two of
{big warehouse, 2 civic buildings, a guild, laws, stability}.*

## 5 · Emporium  *(apex — the "Venice" tier)*
**Population ≥ 20,000**, **AND** trade eminence (an **entrepôt** *or* at least a trade hub),
**AND** it has **finance**, **AND at least 3 of these 7:**
- its **own coinage**,
- an **Entrepôt-grade** warehouse (tier 4+),
- **3+ civic buildings**,
- **written laws**,
- **stable** government,
- decent **public health**,
- a **guild**.

*Check: ≥20,000 people, it's an entrepôt (or trade hub), it has finance, and any three of
{own coin, big warehouse, 3 civic buildings, laws, stability, public health, a guild}.*

---

## Where to read each thing in the app
| Requirement | Where you can see it |
|---|---|
| Population | Settlement/Hub panel population number |
| Trade / trade hub / entrepôt | Map hub styling + Hub panel Trade tab; entrepôt/hub badges (from `classify_hubs`, re-ranked twice a year) |
| Warehouse tier | Warehouses panel (Depot < Storehouse < Warehouse < Entrepôt < Grand Entrepôt) |
| Finance (mint / coin / bank) | Coin & Credit / Money panels; a city with its own named coin or a bank |
| Government / laws | Hub panel government section (council, officials, laws) |
| Civic buildings | Hub panel structures / civic works |
| Stability / public health | Hub panel sentiment (stability) + public health |
| Guild | Guilds panel (is a guild seated here) |

## Tuning note
If, on a real run, most cities plateau at Market/Guild Town and few reach Free City /
Emporium, the likely blockers are **finance** (few cities mint/bank) and **big warehouses**
(few reach tier 3–4). Those are the knobs to loosen — tell me what you observe and I'll
adjust the required counts or the finance/warehouse thresholds.
