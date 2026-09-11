/**
 * GENERATION_UX_REDESIGN_PLAN.md Slice 8 — a minimal, dependency-free PDF
 * writer. No new package (Rust or npm) is added: PDF's 14 STANDARD fonts
 * (Helvetica among them) need no font embedding at all — any compliant
 * reader already has them — so real, selectable, searchable vector TEXT is
 * reachable without the "new Rust crate + font-embedding step" the plan's
 * own status note once named as the blocker. What this build does NOT
 * attempt is image compression: each page's map raster is written as a RAW
 * uncompressed RGB stream (no `/Filter`), which PDF has always supported —
 * larger files than a JPEG/Flate-compressed page, but no codec dependency
 * either. That is the honest trade this module makes.
 *
 * Supports exactly what the Export dialog's Atlas mode needs: an image page
 * (one map raster, full-bleed, with a title/caption in real text) and a
 * text-only page (the gazetteer / legend), nothing more general.
 */

type PdfValue = number | string | boolean | PdfValue[] | { [k: string]: PdfValue } | PdfRef;

class PdfRef {
  constructor(public id: number) {}
  toString() { return `${this.id} 0 R`; }
}

function serializeDict(d: { [k: string]: PdfValue }): string {
  const parts: string[] = ["<<"];
  for (const [k, v] of Object.entries(d)) {
    parts.push(`/${k} ${serializeValue(v)}`);
  }
  parts.push(">>");
  return parts.join(" ");
}

function serializeValue(v: PdfValue): string {
  if (v instanceof PdfRef) return v.toString();
  if (typeof v === "number") return Number.isInteger(v) ? String(v) : v.toFixed(4);
  if (typeof v === "boolean") return v ? "true" : "false";
  if (Array.isArray(v)) return `[${v.map(serializeValue).join(" ")}]`;
  if (typeof v === "object") return serializeDict(v);
  // string: either a /Name (no spaces, starts with letter and not quoted) is
  // passed pre-formatted by the caller; plain strings here are PDF literal
  // strings, escaped per spec.
  return v as string;
}

/** A PDF literal string `(...)`, with `(`, `)`, `\` escaped. */
function pdfString(s: string): string {
  const escaped = s.replace(/[\\()]/g, (c) => "\\" + c);
  return `(${escaped})`;
}

/** Escape a plain ASCII string down to WinAnsi-safe bytes (drop anything
 *  outside the standard font's coverage rather than mis-render it — a
 *  gazetteer entry with an unusual glyph degrades to '?' rather than
 *  corrupting the page). */
function asciiSafe(s: string): string {
  return Array.from(s).map((ch) => {
    const code = ch.codePointAt(0) ?? 63;
    return code >= 32 && code < 127 ? ch : "?";
  }).join("");
}

interface TextLine { text: string; x: number; y: number; size: number; font?: "F1" | "F2" }

export class PdfDocument {
  private objects: (string | Uint8Array)[] = [];
  private pageRefs: PdfRef[] = [];
  private pageW: number;
  private pageH: number;

  constructor(pageWidthPt: number, pageHeightPt: number) {
    this.pageW = pageWidthPt;
    this.pageH = pageHeightPt;
    // Reserve object 1 for the Pages tree (filled in at build()).
    this.objects.push("");
  }

  private nextId(): number {
    this.objects.push("");
    return this.objects.length - 1; // objects[0] unused; ids are 1-based
  }

  private setObject(id: number, content: string) {
    this.objects[id] = content;
  }

  private addStreamObject(dict: { [k: string]: PdfValue }, bytes: Uint8Array): PdfRef {
    const id = this.nextId();
    const header = `${id} 0 obj\n${serializeDict({ ...dict, Length: bytes.length })}\nstream\n`;
    const footer = `\nendstream\nendobj\n`;
    const combined = new Uint8Array(header.length + bytes.length + footer.length);
    // ASCII-only header/footer so byte-offset arithmetic below stays exact.
    for (let i = 0; i < header.length; i++) combined[i] = header.charCodeAt(i);
    combined.set(bytes, header.length);
    for (let i = 0; i < footer.length; i++) combined[header.length + bytes.length + i] = footer.charCodeAt(i);
    this.objects[id] = combined; // stored as raw bytes; build() detects the type
    return new PdfRef(id);
  }

  private addObject(content: string): PdfRef {
    const id = this.nextId();
    this.setObject(id, `${id} 0 obj\n${content}\nendobj\n`);
    return new PdfRef(id);
  }

  /** One shared standard-font resource per family — no embedding, per the
   *  module's own doc comment. Cached so every page shares the same object. */
  private fontRefs: Partial<Record<"F1" | "F2", PdfRef>> = {};
  private fontRef(key: "F1" | "F2"): PdfRef {
    if (!this.fontRefs[key]) {
      const baseFont = key === "F1" ? "Helvetica" : "Helvetica-Bold";
      this.fontRefs[key] = this.addObject(
        serializeDict({ Type: "/Font", Subtype: "/Type1", BaseFont: `/${baseFont}` }),
      );
    }
    return this.fontRefs[key]!;
  }

  /** Add a full-bleed image page: `rgb` is RAW (uncompressed) top-to-bottom
   *  RGB pixel bytes, `w`×`h` its pixel dimensions. `lines` are optional
   *  vector-text overlays (real, selectable type) in PDF points from the
   *  page's bottom-left. */
  addImagePage(rgb: Uint8Array, w: number, h: number, lines: TextLine[] = []) {
    const imgRef = this.addStreamObject(
      { Type: "/XObject", Subtype: "/Image", Width: w, Height: h, ColorSpace: "/DeviceRGB", BitsPerComponent: 8 },
      rgb,
    );
    this.addPageWithResources({ Im1: imgRef }, (name) => {
      const cs = [
        "q",
        `${this.pageW} 0 0 ${this.pageH} 0 0 cm`,
        `/${name(imgRef)} Do`,
        "Q",
      ];
      cs.push(...this.textCommands(lines));
      return cs.join("\n");
    });
  }

  /** A text-only page (the gazetteer / a legend). */
  addTextPage(lines: TextLine[]) {
    this.addPageWithResources({}, () => this.textCommands(lines).join("\n"));
  }

  private textCommands(lines: TextLine[]): string[] {
    const out: string[] = [];
    for (const l of lines) {
      const font = l.font ?? "F1";
      this.fontRef(font); // ensure it exists / gets referenced in Resources below
      out.push("BT", `/${font} ${l.size} Tf`, `${l.x.toFixed(2)} ${l.y.toFixed(2)} Td`, `${pdfString(asciiSafe(l.text))} Tj`, "ET");
    }
    return out;
  }

  private addPageWithResources(xobjects: Record<string, PdfRef>, content: (nameOf: (r: PdfRef) => string) => string) {
    const nameOf = (r: PdfRef) => Object.keys(xobjects).find((k) => xobjects[k] === r) ?? "Im1";
    const body = content(nameOf);
    const bytes = new TextEncoder().encode(body);
    const contentRef = this.addStreamObject({}, bytes);
    const xobjDict: { [k: string]: PdfValue } = {};
    for (const [k, v] of Object.entries(xobjects)) xobjDict[k] = v;
    const resources: { [k: string]: PdfValue } = {
      Font: { F1: this.fontRef("F1"), F2: this.fontRef("F2") },
    };
    if (Object.keys(xobjDict).length > 0) resources.XObject = xobjDict;
    const pageRef = this.addObject(serializeDict({
      Type: "/Page",
      Parent: new PdfRef(1),
      MediaBox: [0, 0, this.pageW, this.pageH],
      Resources: resources,
      Contents: contentRef,
    }));
    this.pageRefs.push(pageRef);
  }

  /** Assemble the final byte stream: header, every object (in order), the
   *  Pages tree (object 1, filled in now that every page is known), the
   *  xref table and trailer. */
  build(): Uint8Array {
    this.setObject(1, `1 0 obj\n${serializeDict({
      Type: "/Pages", Count: this.pageRefs.length, Kids: this.pageRefs,
    })}\nendobj\n`);
    const catalogRef = this.addObject(serializeDict({ Type: "/Catalog", Pages: new PdfRef(1) }));

    const chunks: Uint8Array[] = [];
    const header = "%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
    chunks.push(strToBytes(header));
    const offsets: number[] = [0]; // object 0 is the free-list head
    let pos = chunks[0].length;
    for (let id = 1; id < this.objects.length; id++) {
      offsets[id] = pos;
      const obj = this.objects[id];
      const bytes = typeof obj === "string" ? strToBytes(obj) : obj;
      chunks.push(bytes);
      pos += bytes.length;
    }
    const xrefStart = pos;
    const n = this.objects.length;
    let xref = `xref\n0 ${n}\n0000000000 65535 f \n`;
    for (let id = 1; id < n; id++) {
      xref += `${String(offsets[id]).padStart(10, "0")} 00000 n \n`;
    }
    xref += `trailer\n${serializeDict({ Size: n, Root: catalogRef })}\nstartxref\n${xrefStart}\n%%EOF`;
    chunks.push(strToBytes(xref));

    const total = chunks.reduce((s, c) => s + c.length, 0);
    const out = new Uint8Array(total);
    let off = 0;
    for (const c of chunks) { out.set(c, off); off += c.length; }
    return out;
  }
}

function strToBytes(s: string): Uint8Array {
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
  return out;
}

/** Convert a canvas's RGBA `ImageData` into raw top-to-bottom RGB bytes
 *  (PDF images are top-down, matching `ImageData` — no flip needed). */
export function rgbaToRgb(imageData: ImageData): Uint8Array {
  const { data, width, height } = imageData;
  const out = new Uint8Array(width * height * 3);
  let j = 0;
  for (let i = 0; i < data.length; i += 4) {
    out[j++] = data[i];
    out[j++] = data[i + 1];
    out[j++] = data[i + 2];
  }
  return out;
}

export type { TextLine };
