import { useEffect, useMemo, useState } from "react";
import { campaignGetIndividual } from "@bridge";
import type { GovernmentBrief, IndividualBrief, SeatBrief } from "@types";
import { T, FZ, SERIF, RADIUS } from "@ui/campaign/chronicleTheme";
import { CoatOfArms, houseColor } from "@ui/heraldry/CoatOfArms";
import { Bust } from "@ui/campaign/personShared";
import { LifeStory } from "@ui/campaign/LifeStory";
import { OpenLifeButton } from "@ui/campaign/PersonWindow";
import { TraitChip, IdeologyBars, officeIcon, pathMeta, TRAIT_META, traitColor } from "@ui/campaign/traitIcons";
import { GovFormBadge } from "@ui/campaign/govFormBadge";
import { CouncilRoom, ROOM_LABEL, type RoomKind } from "@ui/campaign/CouncilRoom";

/** The council chamber (2026-09-30 redesign of the Government window/tab).
 *  Instead of a table of names, the government reads as a ROOM: the seats sit
 *  on a semicircle around the head, each a portrait ringed in the colour of
 *  whom they answer to, with their office icon and — while a debate is on —
 *  how they are leaning. Below, each seat is a card carrying the person: their
 *  traits as icons, the ideology they carry, why they sit, how able they are.
 *  Click a card for the seat holder's whole life story. Everything shown is
 *  the sim's own data (`campaign_get_government` + `campaign_get_individual`). */

const ALLEGIANCE = [
  { word: "house", color: "#c9a227" },
  { word: "ruler's kin", color: "#7fd0a0" },
  { word: "commons", color: "#6a86a6" },
];
const FAMILY_ICON: Record<string, string> = {
  "citizenship and culture": "🪪", foreigners: "🧳", learning: "📚", welfare: "🌾",
  economy: "⚖", military: "⚔", constitution: "📜", buildings: "🏗",
};

function seatRing(s: SeatBrief): string {
  if (s.allegiance === 0 && s.house_name) return houseColor(s.house_name);
  return ALLEGIANCE[s.allegiance]?.color ?? T.line;
}

function leanColor(v: number): string {
  if (v > 0.15) return "#5fc08a";
  if (v < -0.15) return "#e0735a";
  return "#9fb4cc";
}

export function CouncilChamber({ brief, compact }: { brief: GovernmentBrief; compact?: boolean }) {
  const [people, setPeople] = useState<Record<number, IndividualBrief>>({});
  const [openSeat, setOpenSeat] = useState<number | null>(null);
  const [room, setRoom] = useState<RoomKind>(() => {
    try { const v = localStorage.getItem("wf.councilRoom"); if (v === "chamber" || v === "table" || v === "hemicycle") return v; } catch { /* storage unavailable */ }
    return "chamber";
  });
  useEffect(() => { try { localStorage.setItem("wf.councilRoom", room); } catch { /* storage unavailable */ } }, [room]);
  const idsKey = brief.seats.map((s) => s.individual_id).join(",");
  useEffect(() => {
    let alive = true;
    const ids = brief.seats.map((s) => s.individual_id).filter((i) => i >= 0);
    Promise.all(ids.map((id) => campaignGetIndividual(id).then((p) => [id, p] as const).catch(() => [id, null] as const)))
      .then((pairs) => {
        if (!alive) return;
        const m: Record<number, IndividualBrief> = {};
        for (const [id, p] of pairs) if (p) m[id] = p;
        setPeople(m);
      });
    return () => { alive = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [idsKey, brief.hub]);

  const deb = brief.debate;
  const votes = useMemo(() => {
    if (!deb) return null;
    let yes = 0, no = 0, undecided = 0;
    for (const s of brief.seats) {
      const v = s.vote_lean ?? 0;
      if (v > 0.15) yes++; else if (v < -0.15) no++; else undecided++;
    }
    return { yes, no, undecided };
  }, [deb, brief.seats]);

  const head = brief.seats.find((s) => s.role === 0) ?? brief.seats[0];
  const others = brief.seats.filter((s) => s !== head);

  return (
    <div data-no-drag style={{ fontSize: FZ.body, color: T.ink }}>
      {/* ── Header: form · head · legitimacy · lean ───────────────────── */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap", marginBottom: 6 }}>
        <GovFormBadge form={brief.form} kind={brief.form_kind} />
        {brief.head_title && <span style={{ fontSize: FZ.tiny, color: T.inkMid }}>led by the <b style={{ color: T.parchment }}>{brief.head_title}</b></span>}
        <span style={{ flex: 1 }} />
        <Gauge label="legitimacy" value={brief.legitimacy} color={brief.legitimacy >= 0.5 ? T.good : brief.legitimacy >= 0.3 ? T.warn : T.bad} />
        <Gauge label="political pts" value={Math.min(1, brief.gov_points / 4)} text={brief.gov_points.toFixed(1)} color="#8e7bd8" />
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
        <span style={{ fontSize: FZ.micro, color: T.inkDim }}>conservative</span>
        <div style={{ position: "relative", flex: 1, height: 6, borderRadius: 3, background: "linear-gradient(90deg,#8e9dd8,#2a3a52 50%,#c9a227)" }}>
          <span style={{ position: "absolute", top: -3, left: `calc(${((brief.gov_position + 1) / 2) * 100}% - 6px)`, width: 12, height: 12, borderRadius: "50%", background: T.parchment, border: `2px solid ${T.panel}` }} />
        </div>
        <span style={{ fontSize: FZ.micro, color: T.inkDim }}>libertarian</span>
      </div>

      {/* ── Prevailing doctrine and its demands ───────────────────────── */}
      {brief.dominant_ideology ? (
        <div style={{ margin: "4px 0 8px", padding: "5px 8px", borderRadius: RADIUS.md, background: "rgba(154,138,224,0.08)", border: "1px solid rgba(154,138,224,0.3)" }}>
          <div style={{ fontSize: FZ.tiny, color: "#b8aaf0" }}>📜 The streets hold to <b style={{ fontFamily: SERIF }}>{brief.dominant_ideology}</b></div>
          {(brief.demands ?? []).length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 3 }}>
              {(brief.demands ?? []).map(([what, met], i) => (
                <span key={i} title={met ? "a live edict meets this demand" : "unmet — it feeds unrest and presses onto the agenda"}
                  style={{ fontSize: FZ.micro, padding: "0 6px", borderRadius: 999, lineHeight: 1.6,
                    color: met ? "#80c890" : "#e0a080", border: `1px solid ${met ? "#80c89066" : "#e0a08066"}` }}>
                  {met ? "✓" : "✗"} {what}
                </span>
              ))}
            </div>
          )}
        </div>
      ) : (
        <div style={{ fontSize: FZ.micro, color: T.inkDim, margin: "2px 0 8px" }}>No doctrine holds the streets yet.</div>
      )}

      {/* ── The room: chamber · table · hemicycle ─────────────────────── */}
      <div style={{ display: "flex", gap: 4, marginBottom: 4, alignItems: "center" }}>
        {(Object.keys(ROOM_LABEL) as RoomKind[]).map((k) => (
          <span key={k} onClick={() => setRoom(k)} style={{
            cursor: "pointer", fontSize: FZ.micro, padding: "1px 8px", borderRadius: 999,
            color: room === k ? T.panel : T.inkDim, background: room === k ? T.gold : "transparent", border: `1px solid ${room === k ? T.gold : T.line}`,
          }}>{ROOM_LABEL[k]}</span>
        ))}
        {deb && votes && (
          <span style={{ marginLeft: "auto", fontSize: FZ.micro, color: T.inkMid }}>
            <span style={{ color: "#5fc08a" }}>● {votes.yes} aye</span> · <span style={{ color: "#e0735a" }}>● {votes.no} nay</span> · <span>● {votes.undecided} wavering</span>
          </span>
        )}
      </div>
      <div style={{ marginBottom: 6 }}>
        <CouncilRoom brief={brief} people={people} kind={room} openSeat={openSeat} onSeat={setOpenSeat} />
      </div>

      {/* ── The motion on the floor ───────────────────────────────────── */}
      {deb && (
        <div style={{ margin: "2px 0 8px", padding: "6px 8px", borderRadius: RADIUS.md, background: T.card, border: `1px solid ${T.line}` }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{ fontSize: 14 }}>{FAMILY_ICON[deb.family] ?? "📜"}</span>
            <span style={{ fontFamily: SERIF, color: T.parchment, fontSize: FZ.base, flex: 1 }}>
              Motion: to {deb.what || deb.family}
            </span>
            <span style={{ fontSize: FZ.micro, color: deb.major ? T.gold : T.inkDim }}>{deb.major ? "MAJOR" : "minor"}</span>
          </div>
          {deb.agenda && brief.dominant_ideology && (
            <div style={{ fontSize: FZ.micro, color: "#b8aaf0", marginTop: 2 }}>pressed onto the floor by the partisans of {brief.dominant_ideology}</div>
          )}
          <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 4 }}>
            <span style={{ fontSize: FZ.micro, color: "#e0735a" }}>fail</span>
            <div style={{ position: "relative", flex: 1, height: 7, borderRadius: 4, background: "linear-gradient(90deg,#5a2a24,#1e2e42 50%,#244a34)" }}>
              <span style={{ position: "absolute", top: -2, left: `calc(${((deb.tally + 1) / 2) * 100}% - 5px)`, width: 10, height: 11, borderRadius: 3, background: "#5fd0ff" }} />
            </div>
            <span style={{ fontSize: FZ.micro, color: "#5fc08a" }}>pass</span>
          </div>
          <div style={{ display: "flex", gap: 3, marginTop: 4, alignItems: "center" }}>
            <span style={{ fontSize: FZ.micro, color: T.inkDim, marginRight: 3 }}>rounds</span>
            {Array.from({ length: deb.round_cap }, (_, k) => (
              <span key={k} style={{ width: 9, height: 9, borderRadius: 2, background: k < deb.round ? "#5fd0ff" : "transparent", border: `1px solid ${k < deb.round ? "#5fd0ff" : T.line}` }} />
            ))}
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3,minmax(0,1fr))", gap: 5, marginTop: 6 }}>
            {([["aye", "#5fc08a", (v: number) => v > 0.15], ["wavering", "#9fb4cc", (v: number) => Math.abs(v) <= 0.15], ["nay", "#e0735a", (v: number) => v < -0.15]] as const).map(([word, col, test]) => {
              const hit = brief.seats.map((s, i) => ({ s, i })).filter((e) => test(e.s.vote_lean ?? 0));
              return (
                <div key={word} style={{ borderRadius: RADIUS.md, border: `1px solid ${T.lineSoft}`, background: "rgba(0,0,0,.18)", padding: "4px 6px" }}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 4, marginBottom: 3 }}>
                    <span style={{ width: 7, height: 7, borderRadius: "50%", background: col }} />
                    <span style={{ fontFamily: SERIF, fontSize: FZ.base, color: T.parchment }}>{hit.length}</span>
                    <span style={{ fontSize: FZ.micro, color: T.inkDim, textTransform: "uppercase", letterSpacing: 0.5 }}>{word}</span>
                  </div>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 3, minHeight: 22 }}>
                    {hit.map(({ s, i }) => {
                      const p = people[s.individual_id];
                      return (
                        <div key={i} title={`${s.office_title} ${s.name}`} onClick={() => setOpenSeat(i === brief.seats.indexOf(head) ? 0 : others.indexOf(s) + 1)}
                          style={{ width: 26, height: 26, borderRadius: "50%", overflow: "hidden", cursor: "pointer", boxShadow: `0 0 0 2px ${seatRing(s)}`, background: T.card }}>
                          {p ? <Bust person={p} size={26} /> : null}
                        </div>
                      );
                    })}
                    {hit.length === 0 && <span style={{ fontSize: FZ.micro, color: T.inkFaint }}>no one</span>}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
      {/* ── One civic agenda: the culture questions also before the council ── */}
      {(brief.culture_motions ?? []).length > 0 && (
        <div style={{ margin: "0 0 8px", padding: "5px 8px", borderRadius: RADIUS.md, background: T.card, border: `1px dashed ${T.line}` }}>
          <div style={{ fontSize: FZ.micro, color: T.inkDim, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 3 }}>
            {deb ? "Also on the agenda" : "On the agenda"} · the city's peoples
          </div>
          {(brief.culture_motions ?? []).map((m, k) => (
            <div key={k} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: FZ.tiny, padding: "2px 0" }}>
              <span>{m.warmer ? "🤝" : "🚫"}</span>
              <span style={{ color: T.ink, flex: 1 }}>
                the <b>{m.culture}</b>: {m.from_tier} → <b style={{ color: m.warmer ? "#7fd0a0" : "#e08a6a" }}>{m.to_tier}</b>
                {m.reason && <span style={{ color: T.inkFaint }}> · {m.reason}</span>}
              </span>
              <div style={{ position: "relative", width: 60, height: 5, borderRadius: 3, background: "linear-gradient(90deg,#5a2a24,#1e2e42 50%,#244a34)" }}>
                <span style={{ position: "absolute", top: -2, left: `calc(${((m.tally + 1) / 2) * 100}% - 3px)`, width: 6, height: 9, borderRadius: 2, background: "#5fd0ff" }} />
              </div>
            </div>
          ))}
        </div>
      )}

      {/* ── The seats, as people ──────────────────────────────────────── */}
      <div style={{ display: "grid", gridTemplateColumns: compact ? "1fr" : "1fr 1fr", gap: 5 }}>
        {[head, ...others].filter(Boolean).map((s, i) => (
          <SeatCard key={i} s={s as SeatBrief} p={people[(s as SeatBrief).individual_id]} formKind={brief.form_kind}
            debate={!!deb} on={openSeat === i} onClick={() => setOpenSeat(openSeat === i ? null : i)} />
        ))}
      </div>
      {openSeat != null && (() => {
        const s = [head, ...others][openSeat];
        const p = s ? people[s.individual_id] : undefined;
        if (!s || !p) return null;
        return (
          <div style={{ marginTop: 6, padding: "7px 9px", borderRadius: RADIUS.md, background: "rgba(0,0,0,0.2)", border: `1px solid ${seatRing(s)}66` }}>
            <div style={{ fontFamily: SERIF, color: T.parchment, fontSize: FZ.base, marginBottom: 4 }}>
              {officeIcon(s.role, brief.form_kind)} {s.office_title} {p.name} — a life
              <span style={{ marginLeft: 8 }}><OpenLifeButton id={p.id} label="📖 Decisions, travels & full life" /></span>
            </div>
            <LifeStory life={p.life} places={p.places} color={seatRing(s)} fallback={p.life_log.slice().reverse()} />
          </div>
        );
      })()}

      {/* ── Edicts in force · recent history ──────────────────────────── */}
      <div style={{ marginTop: 8, display: "grid", gridTemplateColumns: compact ? "1fr" : "1fr 1fr", gap: 8 }}>
        <div>
          <SubHead>Edicts in force ({brief.edicts.length})</SubHead>
          {brief.edicts.length === 0 && <div style={{ fontSize: FZ.tiny, color: T.inkDim }}>None in force.</div>}
          {brief.edicts.map((e, i) => (
            <div key={i} style={{ display: "flex", gap: 5, alignItems: "baseline", fontSize: FZ.tiny, padding: "1px 0" }}>
              <span>{FAMILY_ICON[e.family] ?? "📜"}</span>
              <span style={{ flex: 1, color: T.inkMid }}>{e.what || e.family}{e.major ? " ★" : ""}</span>
              <span style={{ color: T.inkFaint }}>{e.enacted_year}–{e.expires_year}</span>
            </div>
          ))}
        </div>
        <div>
          <SubHead>Recent history</SubHead>
          {brief.history.length === 0 && <div style={{ fontSize: FZ.tiny, color: T.inkDim }}>Nothing decided yet.</div>}
          {brief.history.slice().reverse().slice(0, 8).map((h, i) => {
            const c = h.regime_kind ? "#e0a0e0" : h.outcome === "passed" ? "#80c890" : h.outcome === "failed" ? "#e08080" : "#e6c86a";
            return (
              <div key={i} style={{ display: "flex", gap: 5, alignItems: "baseline", fontSize: FZ.tiny, padding: "1px 0" }}>
                <span style={{ color: T.inkFaint, minWidth: 26 }}>{h.year}</span>
                <span style={{ flex: 1, color: T.inkMid }}>{h.regime_kind ? `⚡ ${h.regime_kind}` : `${FAMILY_ICON[h.family] ?? "📜"} ${h.family}`}</span>
                {!h.regime_kind && <span style={{ color: c }}>{h.outcome}</span>}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function SubHead({ children }: { children: React.ReactNode }) {
  return <div style={{ fontSize: FZ.micro, color: T.inkDim, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 3 }}>{children}</div>;
}

function Gauge({ label, value, color, text }: { label: string; value: number; color: string; text?: string }) {
  const r = 11, c = 2 * Math.PI * r;
  return (
    <span title={label} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
      <svg width={28} height={28}>
        <circle cx={14} cy={14} r={r} fill="none" stroke={T.line} strokeWidth={3} />
        <circle cx={14} cy={14} r={r} fill="none" stroke={color} strokeWidth={3} strokeDasharray={`${c * Math.max(0, Math.min(1, value))} ${c}`} transform="rotate(-90 14 14)" strokeLinecap="round" />
        <text x={14} y={17} textAnchor="middle" fontSize={8} fill={T.ink}>{text ?? Math.round(value * 100)}</text>
      </svg>
      <span style={{ fontSize: FZ.micro, color: T.inkDim }}>{label}</span>
    </span>
  );
}

function SeatCard({ s, p, formKind, debate, on, onClick }: {
  s: SeatBrief; p?: IndividualBrief; formKind?: string; debate: boolean; on: boolean; onClick: () => void;
}) {
  const ring = seatRing(s);
  const path = pathMeta(s.path);
  const traits = s.traits ?? p?.traits ?? [];
  const strengths = p?.trait_strength ?? [];
  const lean = s.vote_lean ?? 0;
  const pips = Math.round(s.suitability * 5);
  return (
    <div onClick={onClick} style={{
      display: "flex", gap: 7, padding: "6px 7px", borderRadius: RADIUS.md, cursor: "pointer",
      background: on ? `${ring}14` : T.card, border: `1px solid ${on ? ring : T.lineSoft}`, borderLeft: `3px solid ${ring}`,
    }}>
      <div style={{ flex: "0 0 auto", alignSelf: "flex-start", width: 40, height: 40, borderRadius: 7, overflow: "hidden", boxShadow: `0 0 0 1px ${ring}88` }}>
        {p ? <Bust person={p} size={40} /> : <div style={{ width: 40, height: 40, display: "grid", placeItems: "center", fontSize: 18 }}>{officeIcon(s.role, formKind)}</div>}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 4 }}>
          <span style={{ fontSize: 11 }}>{officeIcon(s.role, formKind)}</span>
          <span style={{ fontSize: FZ.micro, color: T.gold, textTransform: "uppercase", letterSpacing: 0.4, fontWeight: 700 }}>{s.office_title}</span>
          <span style={{ flex: 1 }} />
          {debate && <span style={{ fontSize: FZ.micro, color: leanColor(lean) }}>{lean > 0.15 ? "aye" : lean < -0.15 ? "nay" : "undecided"}</span>}
        </div>
        <div style={{ fontFamily: SERIF, fontSize: FZ.base, color: T.parchment, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {s.name}{s.age ? <span style={{ color: T.inkDim, fontSize: FZ.tiny, fontFamily: "inherit" }}> · {s.age}</span> : null}
          {s.famous && <span title="a notable" style={{ color: T.gold }}> ★</span>}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 2, flexWrap: "wrap" }}>
          <span title={`sits by ${s.path}`} style={{ fontSize: FZ.micro, color: path.color, border: `1px solid ${path.color}55`, borderRadius: 999, padding: "0 5px", lineHeight: 1.5 }}>
            {path.icon} {path.short}
          </span>
          <span title={`suitability ${Math.round(s.suitability * 100)}%`} style={{ letterSpacing: 1, fontSize: 8, color: T.gold }}>
            {"●".repeat(pips)}<span style={{ color: T.line }}>{"●".repeat(5 - pips)}</span>
          </span>
          {s.allegiance === 0 && s.house_name
            ? <span style={{ display: "inline-flex", alignItems: "center", gap: 3, fontSize: FZ.micro, color: ring }}><CoatOfArms name={s.house_name} size={12} />{s.house_name}</span>
            : <span style={{ fontSize: FZ.micro, color: ring }}>{ALLEGIANCE[s.allegiance]?.word}</span>}
        </div>
        {traits.length > 0 && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 2, marginTop: 3 }}>
            {traits.map((t, i) => <TraitChip key={t} name={t} strength={strengths[i] ?? 1} compact
              style={{ color: traitColor(t) }} />)}
          </div>
        )}
        {s.ideology_seeded && s.ideology && (
          <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 3 }} title="the seat holder's own ideology">
            <IdeologyBars v={s.ideology} width={40} height={12} />
            {p?.ideology_name && <span style={{ fontSize: FZ.micro, color: "#b8aaf0" }}>{p.ideology_name}</span>}
          </div>
        )}
        {traits.length > 0 && !on && (
          <div style={{ fontSize: FZ.micro, color: T.inkFaint, marginTop: 2, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {traits.map((t) => TRAIT_META[t]?.blurb).filter(Boolean).slice(0, 2).join(" · ")}
          </div>
        )}
      </div>
    </div>
  );
}
