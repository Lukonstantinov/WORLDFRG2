import { useEffect, useState } from "react";
import {
  campaignGetGovernment, campaignGetCityIdeology, campaignGetCityGallery, campaignGetVenues,
  campaignGetCultureAcceptance, campaignGetJournal,
} from "@bridge";
import type {
  GovernmentBrief, CityIdeologyBrief, MasterworkBrief, VenueBrief, CultureAcceptanceBrief, JournalEntry,
} from "@types";
import { Section, StatGrid, Stat, DataRow, Badge, FootNote, EmptyNote } from "@ui/kit";

/** `docs/living_world/10_SETTLEMENT_OVERVIEW.md`, slice 10.1/10.2 — the
 *  first thing a player sees when opening a city: development, government,
 *  ideology, culture tiers, venues/masterworks and the last chronicle
 *  lines, each block a summary of a DETAILED window elsewhere.
 *
 *  **Scope cut, recorded (rule 36):** the doc's own 10.1 calls for "one read
 *  query assembling the Overview (no new sim state)". This is built as a
 *  CLIENT-SIDE composition of the rows 04/06/07/08's own already-shipped
 *  read commands instead of a single new Rust aggregator — each of those
 *  commands takes its own `db: State<'_, WorldDb>` and locks the same
 *  `std::sync::Mutex` internally, so calling them FROM one more Rust
 *  command would deadlock on that same lock; composing independent invokes
 *  client-side avoids inventing a second, parallel read path for data these
 *  commands already serve correctly. City portrait art reflecting
 *  buildings/venues (10.3) and tidying the OTHER tabs into pure detail
 *  windows (10.4) are NOT built this session — queued as Q10.2/Q10.3.
 *  War/realm/horde-threat (row 09 Parts B/C/E) has no summary here since
 *  those parts were not built this session either (see `hordes.rs`). */
export function SettlementOverviewTab({ hub }: { hub: number }) {
  const [gov, setGov] = useState<GovernmentBrief | null>(null);
  const [ideology, setIdeology] = useState<CityIdeologyBrief | null>(null);
  const [gallery, setGallery] = useState<MasterworkBrief[]>([]);
  const [venues, setVenues] = useState<VenueBrief[]>([]);
  const [tiers, setTiers] = useState<CultureAcceptanceBrief[]>([]);
  const [journal, setJournal] = useState<JournalEntry[]>([]);

  useEffect(() => {
    let alive = true;
    campaignGetGovernment(hub).then((v) => { if (alive) setGov(v); }).catch(() => { if (alive) setGov(null); });
    campaignGetCityIdeology(hub).then((v) => { if (alive) setIdeology(v); }).catch(() => { if (alive) setIdeology(null); });
    campaignGetCityGallery(hub).then((v) => { if (alive) setGallery(v); }).catch(() => { if (alive) setGallery([]); });
    campaignGetVenues(hub).then((v) => { if (alive) setVenues(v); }).catch(() => { if (alive) setVenues([]); });
    campaignGetCultureAcceptance(hub).then((v) => { if (alive) setTiers(v); }).catch(() => { if (alive) setTiers([]); });
    campaignGetJournal(hub, -1).then((v) => { if (alive) setJournal(v.slice(-5).reverse()); }).catch(() => { if (alive) setJournal([]); });
    return () => { alive = false; };
  }, [hub]);

  return (
    <div>
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

      <Section title="Recent chronicle">
        {journal.length === 0 && <EmptyNote>Nothing recorded yet.</EmptyNote>}
        {journal.map((j, i) => (
          <FootNote key={i}>{j.text}</FootNote>
        ))}
      </Section>
    </div>
  );
}
