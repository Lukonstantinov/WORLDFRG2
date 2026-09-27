import { useEffect, useMemo, useState } from "react";
import {
  campaignGetGovernment, campaignGetCityIdeology, campaignGetCityGallery, campaignGetVenues,
  campaignGetCultureAcceptance, campaignGetJournal, campaignGetHub, campaignCityDevelopment,
} from "@bridge";
import type {
  GovernmentBrief, CityIdeologyBrief, MasterworkBrief, VenueBrief, CultureAcceptanceBrief, JournalEntry,
  HubDetail, CityDevelopment,
} from "@types";
import { Section, StatGrid, Stat, DataRow, Badge, FootNote, EmptyNote } from "@ui/kit";
import { useWorldStore } from "@state/worldStore";
import { useCultureKits, cityScene, IsoThumb } from "@ui/campaign/windowKit";

const TRACK_NAMES = ["⚔ Military", "⚖ Trade", "🏛 Civil", "📜 Ideological"] as const;

/** `docs/living_world/10_SETTLEMENT_OVERVIEW.md`, slice 10.1/10.2 (+ Q10.4
 *  this pass) — the first thing a player sees when opening a city:
 *  identity, development, government, ideology, culture tiers, venues/
 *  masterworks, economy at a glance and the last chronicle lines, each
 *  block a summary of a DETAILED window elsewhere.
 *
 *  **Scope cut, recorded (rule 36):** the doc's own 10.1 calls for "one read
 *  query assembling the Overview (no new sim state)". This is built as a
 *  CLIENT-SIDE composition of the rows 04/06/07/08's own already-shipped
 *  read commands instead of a single new Rust aggregator — each of those
 *  commands takes its own `db: State<'_, WorldDb>` and locks the same
 *  `std::sync::Mutex` internally, so calling them FROM one more Rust
 *  command would deadlock on that same lock; composing independent invokes
 *  client-side avoids inventing a second, parallel read path for data these
 *  commands already serve correctly.
 *
 *  **Q10.4, this pass:** identity (population/culture mix), development
 *  (value + this year's breakdown + the four track levels) and economy at
 *  a glance (treasury/trade wealth/food balance) are folded in — each reuses
 *  a command/field this same window's OTHER tabs already read
 *  (`campaignGetHub`, `campaignCityDevelopment`), so this is composition,
 *  not new plumbing.
 *
 *  **10.3, this pass — the city portrait.** Rather than build new
 *  procedural art blind (this environment has no display to visually verify
 *  a new canvas layer against — the exact risk §8.21's own fill-light
 *  regression and row 02's 02.7 face art both name), the Overview's portrait
 *  reuses the EXISTING, already-shipped `cityScene`/`IsoThumb` isometric
 *  renderer verbatim — the same one `CityView.tsx`, `ColonyWindow.tsx` and
 *  `HouseWindow.tsx` already draw a city with, which already derives its
 *  buildings/districts/ships from this hub's own real state
 *  (`deriveSlots`/`deriveDistricts` in `settlementWindowData.ts`). This is
 *  the honest way to get "reflects buildings and venues" without inventing
 *  a second, unverified art path — a genuinely new venue-specific sprite
 *  layer (Q10.1/Q10.2) is real follow-up work, not attempted here.
 *
 *  A full pass tidying the other tabs into pure detail windows behind each
 *  Overview block (Q10.3, the remainder of 10.4) is NOT built this
 *  session — real follow-up work, not a blocker, since Overview summarising
 *  a little of what "summary"/"development" already show is a duplication,
 *  not a broken reading. War/realm/horde-threat (row 09 Parts B/C/E) has no
 *  summary here since those parts remain unbuilt (see `hordes.rs`). */
export function SettlementOverviewTab({ hub }: { hub: number }) {
  const [detail, setDetail] = useState<HubDetail | null>(null);
  const [dev, setDev] = useState<CityDevelopment | null>(null);
  const economy = useWorldStore((s) => s.economy);
  const settlements = useWorldStore((s) => s.settlements);
  const kits = useCultureKits();
  const [gov, setGov] = useState<GovernmentBrief | null>(null);
  const [ideology, setIdeology] = useState<CityIdeologyBrief | null>(null);
  const [gallery, setGallery] = useState<MasterworkBrief[]>([]);
  const [venues, setVenues] = useState<VenueBrief[]>([]);
  const [tiers, setTiers] = useState<CultureAcceptanceBrief[]>([]);
  const [journal, setJournal] = useState<JournalEntry[]>([]);

  useEffect(() => {
    let alive = true;
    campaignGetHub(hub).then((v) => { if (alive) setDetail(v); }).catch(() => { if (alive) setDetail(null); });
    campaignCityDevelopment(hub).then((v) => { if (alive) setDev(v); }).catch(() => { if (alive) setDev(null); });
    campaignGetGovernment(hub).then((v) => { if (alive) setGov(v); }).catch(() => { if (alive) setGov(null); });
    campaignGetCityIdeology(hub).then((v) => { if (alive) setIdeology(v); }).catch(() => { if (alive) setIdeology(null); });
    campaignGetCityGallery(hub).then((v) => { if (alive) setGallery(v); }).catch(() => { if (alive) setGallery([]); });
    campaignGetVenues(hub).then((v) => { if (alive) setVenues(v); }).catch(() => { if (alive) setVenues([]); });
    campaignGetCultureAcceptance(hub).then((v) => { if (alive) setTiers(v); }).catch(() => { if (alive) setTiers([]); });
    campaignGetJournal(hub, -1).then((v) => { if (alive) setJournal(v.slice(-5).reverse()); }).catch(() => { if (alive) setJournal([]); });
    return () => { alive = false; };
  }, [hub]);

  const econHub = economy?.hubs.find((h) => h.id === hub);
  const settlement = settlements.find((s) => s.name === detail?.name);
  const kit = detail?.culture ? kits?.get(detail.culture) : undefined;
  const scene = useMemo(() => detail ? cityScene(detail, {
    kit, seaAccess: econHub?.sea_access, elevation: econHub?.elevation,
    river: settlement?.site === "river" || (detail.vessels?.classes.find((c) => c.kind === "river")?.registered ?? 0) > 0,
  }) : null, [detail, kit, econHub?.sea_access, econHub?.elevation, settlement?.site]);

  return (
    <div>
      {detail && (
        <Section title="Identity">
          {scene && (
            <div style={{ width: "100%", aspectRatio: "2.8 / 1", marginBottom: 6, borderRadius: 4, overflow: "hidden" }}>
              <IsoThumb cfg={scene.cfg} W={280} H={100} />
            </div>
          )}
          <StatGrid cols={3}>
            <Stat label="Population" value={Math.round(detail.population).toLocaleString()} />
            <Stat label="Culture" value={detail.culture || "—"} />
            <Stat label="Minorities" value={String(detail.minorities?.length ?? 0)} />
          </StatGrid>
          {detail.minorities && detail.minorities.length > 0 && (
            <FootNote>
              {detail.minorities.slice(0, 3).map(([c, frac]) => `${c} ${Math.round(frac * 100)}%`).join(" · ")}
            </FootNote>
          )}
        </Section>
      )}

      {dev && (
        <Section title="Development">
          <StatGrid cols={2}>
            <Stat label="Value" value={dev.dev.toFixed(2)} />
            <Stat label="This year" value={`${dev.dev_breakdown.reduce((a, b) => a + b, 0) >= 0 ? "+" : ""}${dev.dev_breakdown.reduce((a, b) => a + b, 0).toFixed(2)}`} />
          </StatGrid>
          <DataRow cols="1fr 1fr 1fr 1fr" zebra>
            {TRACK_NAMES.map((n, i) => (
              <span key={n} style={{ fontSize: 11 }}>{n} L{dev.track_level[i]}</span>
            ))}
          </DataRow>
        </Section>
      )}

      {gov && (
        <Section title="Government">
          <StatGrid cols={3}>
            <Stat label="Form" value={gov.form} />
            <Stat label="Legitimacy" value={`${Math.round(gov.legitimacy * 100)}%`} />
            <Stat label="Seats" value={String(gov.seats.length)} />
          </StatGrid>
          {gov.debate && <FootNote>Debating: {gov.debate.family} (round {gov.debate.round}/{gov.debate.round_cap})</FootNote>}
        </Section>
      )}

      {ideology && (
        <Section title="Ideology">
          <FootNote>
            {ideology.dominant_id >= 0 ? `Prevailing: ${ideology.dominant_name}` : "No dominant doctrine yet"}
            {" · "}{ideology.scholars.length} scholar{ideology.scholars.length === 1 ? "" : "s"}
          </FootNote>
        </Section>
      )}

      {tiers.length > 0 && (
        <Section title="Culture tiers">
          {tiers.slice(0, 4).map((t) => (
            <DataRow key={t.culture} cols="1fr 0.6fr 0.5fr" zebra>
              <span>{t.culture}</span>
              <span style={{ fontSize: 11, opacity: 0.7 }}>{Math.round(t.residents_frac * 100)}%</span>
              <Badge tone={t.tier <= 1 ? "good" : t.tier >= 3 ? "bad" : "neutral"}>{t.tier_name}</Badge>
            </DataRow>
          ))}
        </Section>
      )}

      {(venues.length > 0 || gallery.length > 0) && (
        <Section title="Venues & masterworks">
          <FootNote>
            {venues.length} venue{venues.length === 1 ? "" : "s"} · {gallery.length} masterwork{gallery.length === 1 ? "" : "s"}
          </FootNote>
        </Section>
      )}

      {detail && detail.is_estate === false && (
        <Section title="Economy at a glance">
          <StatGrid cols={3}>
            <Stat label="Treasury" value={Math.round(detail.treasury ?? 0).toLocaleString()} />
            <Stat label="Trade wealth" value={detail.trade_wealth.toFixed(2)} />
            <Stat label="Food balance" value={detail.food_balance.toFixed(2)} />
          </StatGrid>
        </Section>
      )}

      <Section title="Recent chronicle">
        {journal.length === 0 && <EmptyNote>Nothing recorded yet.</EmptyNote>}
        {journal.map((j, i) => (
          <FootNote key={i}>{j.text}</FootNote>
        ))}
      </Section>
    </div>
  );
}
