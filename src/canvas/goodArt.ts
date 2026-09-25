// Trade-good artwork — TRUE pixel art, the ONLY goods treatment in the app.
// Ported from the design handoff's `wf-pixel-goods.js`; every sprite function and
// every number is kept as written (the numbers ARE the design).
//
// Every good is a hand-placed 24×24 sprite: integer-grid shapes (no anti-aliasing),
// a five-step hue-shifted ramp per good (shadows lean violet, lights lean cream),
// a small fixed material set (wood, leaf, steel, glass, burlap, clay, stone, paper,
// gold), top-left light, and a selective outline — each edge pixel takes a darkened
// copy of the colour it borders instead of one flat black line.
// Rendered at an integer scale with smoothing off; dark goods get a 1-px light rim
// so they still read on a dark panel. One entry per GOOD_DEFS name plus a crate
// fallback, stencilled in the good's own tint, for user-added goods.

const N = 24;
const T2 = Math.PI * 2;

type Rgb = [number, number, number];
type Mask = (x: number, y: number) => boolean;
type Pt = number[];
type Ramp = string[];

function hx(h: string): Rgb { h = (h || "#888888").replace("#", ""); if (h.length === 3) h = h.split("").map((c) => c + c).join(""); return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)) as Rgb; }
function toHex(c: number[]): string { return "#" + c.map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0")).join(""); }
function mix(a: string, b: string, t: number): string { const A = hx(a), B = hx(b); return toHex(A.map((v, i) => v + (B[i] - v) * t)); }
function lumOf(h: string): number { const c = hx(h); return (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]) / 255; }

/** Five-step ramp: 0 deepest · 1 shadow · 2 base · 3 light · 4 highlight. */
export function ramp(hex: string): Ramp {
  return [mix(hex, "#140a1e", 0.74), mix(hex, "#1c1230", 0.44), toHex(hx(hex)), mix(hex, "#fff2cc", 0.36), mix(hex, "#fffcf0", 0.7)];
}
const WOOD  =["#2a170c","#4e2e16","#7a4a24","#a26a36","#c89058"];
const LEAF  =["#15290f","#244a1a","#3c7228","#5f9a3a","#8cc25a"];
const STEEL =["#1e242c","#46505c","#7a8694","#b4c0cc","#e8f0f6"];
const BURLAP=["#3a2814","#664a2a","#94723e","#bc9a5c","#dcc088"];
const GLASS =["#1c3a40","#3c7078","#72aab0","#b0e0e0","#f0ffff"];
const GOLD  =["#4a3008","#8a5c10","#c8961e","#ecc44a","#fff0a8"];
const PAPER =["#5a4a30","#a08c64","#d2c29a","#ece0c0","#fffaea"];
const CLAY  =["#3e1e10","#6e3a20","#a05a34","#c47e50","#e2a878"];
const STONE =["#2c2c30","#56565c","#86868c","#b4b4ba","#e0e0e4"];
const WHITE =["#6a6a74","#a8a8b2","#d8d8de","#f0f0f2","#ffffff"];
const RED   =["#3a0a0e","#701620","#b0282c","#d8503a","#f08a6a"];
const FLAME =["#7a2a08","#c85a10","#f09a20","#ffd24a","#fff6c0"];
const FOAM  =["#8a8070","#c8c0b0","#ece6da","#f8f4ec","#ffffff"];
const BLUE  =["#101c3a","#1e3a78","#3462b8","#6a94dc","#b0ccf4"];
const ENDG  =ramp("#d8b07a");

// ── masks: (x,y) → inside?  tested at pixel centres ─────────────────────────
const R=(x:number,y:number,w:number,h:number):Mask=>(X,Y)=>X>=x&&X<x+w&&Y>=y&&Y<y+h;
const C=(cx:number,cy:number,r:number):Mask=>(X,Y)=>{const dx=X+.5-cx,dy=Y+.5-cy;return dx*dx+dy*dy<=r*r;};
const E=(cx:number,cy:number,rx:number,ry:number):Mask=>(X,Y)=>{const dx=(X+.5-cx)/rx,dy=(Y+.5-cy)/ry;return dx*dx+dy*dy<=1;};
const RE=(cx:number,cy:number,rx:number,ry:number,a:number):Mask=>{const c=Math.cos(a),s=Math.sin(a);return (X,Y)=>{const x=X+.5-cx,y=Y+.5-cy,u=(x*c+y*s)/rx,v=(-x*s+y*c)/ry;return u*u+v*v<=1;};};
const SE=(cx:number,cy:number,rx:number,ry:number,p:number):Mask=>(X,Y)=>Math.pow(Math.abs(X+.5-cx)/rx,p)+Math.pow(Math.abs(Y+.5-cy)/ry,p)<=1;
const P=(pts:Pt[]):Mask=>(X,Y)=>{const x=X+.5,y=Y+.5;let ins=false;for(let i=0,j=pts.length-1;i<pts.length;j=i++){const xi=pts[i][0],yi=pts[i][1],xj=pts[j][0],yj=pts[j][1];if(((yi>y)!==(yj>y))&&(x<(xj-xi)*(y-yi)/(yj-yi)+xi))ins=!ins;}return ins;};
const Ln=(x0:number,y0:number,x1:number,y1:number,w:number):Mask=>(X,Y)=>{const x=X+.5,y=Y+.5,dx=x1-x0,dy=y1-y0,l=dx*dx+dy*dy||1;let t=((x-x0)*dx+(y-y0)*dy)/l;t=t<0?0:t>1?1:t;const ex=x0+t*dx-x,ey=y0+t*dy-y;return ex*ex+ey*ey<=w*w/4;};
const U=(...m:Mask[]):Mask=>(X,Y)=>m.some(f=>f(X,Y));
const D=(a:Mask,b:Mask):Mask=>(X,Y)=>a(X,Y)&&!b(X,Y);
const I=(a:Mask,b:Mask):Mask=>(X,Y)=>a(X,Y)&&b(X,Y);
const above=(y:number):Mask=>(X,Y)=>Y<y, below=(y:number):Mask=>(X,Y)=>Y>=y;
const tear=(cx:number,cy:number,r:number)=>U(C(cx,cy,r),P([[cx-r*0.78,cy-r*0.55],[cx,cy-r*2.3],[cx+r*0.78,cy-r*0.55]]));
const oct=(x:number,y:number,w:number,h:number,k:number)=>P([[x+k,y],[x+w-k,y],[x+w,y+k],[x+w,y+h-k],[x+w-k,y+h],[x+k,y+h],[x,y+h-k],[x,y+k]]);
/** A curved, tapering horn/tusk: quadratic Bézier A→B via Q, width w0→w1. */
function taper(A:Pt,Q:Pt,B:Pt,w0:number,w1:number,t0=0,t1=1){
  const L:Pt[]=[],Rt:Pt[]=[],n=18;
  for(let i=0;i<=n;i++){const t=t0+(t1-t0)*i/n,u=1-t;
    const x=u*u*A[0]+2*u*t*Q[0]+t*t*B[0], y=u*u*A[1]+2*u*t*Q[1]+t*t*B[1];
    const dx=2*u*(Q[0]-A[0])+2*t*(B[0]-Q[0]), dy=2*u*(Q[1]-A[1])+2*t*(B[1]-Q[1]), l=Math.hypot(dx,dy)||1;
    const w=(w0+(w1-w0)*t)/2, nx=-dy/l*w, ny=dx/l*w;
    L.push([x+nx,y+ny]); Rt.push([x-nx,y-ny]);}
  return P(L.concat(Rt.reverse()));
}
function fish(cx:number,cy:number,L:number,H:number,a:number){
  const c=Math.cos(a),s=Math.sin(a),ux=c,uy=s,vx=-s,vy=c;
  const tb=[cx-ux*(L/2-1),cy-uy*(L/2-1)];
  return U(RE(cx,cy,L/2,H/2,a),P([tb,[tb[0]-ux*3.6+vx*H*0.62,tb[1]-uy*3.6+vy*H*0.62],[tb[0]-ux*2.4,tb[1]-uy*2.4],[tb[0]-ux*3.6-vx*H*0.62,tb[1]-uy*3.6-vy*H*0.62]]));
}

// ── painter ────────────────────────────────────────────────────────────────
type Mode = "bevel" | "ball" | "cylV" | "cylH" | number;
function painter(){
  const b:(string|null)[]=new Array(N*N).fill(null);
  const set=(x:number,y:number,c:string)=>{x=Math.floor(x);y=Math.floor(y);if(x>=0&&y>=0&&x<N&&y<N)b[y*N+x]=c;};
  const scan=(m:Mask)=>{const pts:[number,number][]=[];let x0=N,y0=N,x1=-1,y1=-1;
    for(let y=0;y<N;y++)for(let x=0;x<N;x++)if(m(x,y)){pts.push([x,y]);if(x<x0)x0=x;if(x>x1)x1=x;if(y<y0)y0=y;if(y>y1)y1=y;}
    return {pts,x0,y0,x1,y1};};
  function f(m:Mask,r:Ramp,mode:Mode="bevel"){
    const {pts,x0,y0,x1,y1}=scan(m); if(!pts.length) return;
    const cx=(x0+x1+1)/2, cy=(y0+y1+1)/2, rx=Math.max(1,(x1-x0+1)/2), ry=Math.max(1,(y1-y0+1)/2);
    for(const [x,y] of pts){
      const up=!m(x,y-1), lf=!m(x-1,y), dn=!m(x,y+1), rt=!m(x+1,y);
      let i=2;
      if(typeof mode==="number") i=mode;
      else if(mode==="bevel") i=(up||lf)?3:(dn||rt)?1:2;
      else if(mode==="ball"){const hx=cx-rx*0.36,hy=cy-ry*0.4,d=Math.hypot((x+.5-hx)/rx,(y+.5-hy)/ry);
        i=d<0.3?4:d<0.72?3:d<1.12?2:1; if((dn||rt)&&i>1) i=1;}
      else if(mode==="cylV"||mode==="cylH"){const t=mode==="cylV"?(x+.5-x0)/(x1-x0+1):(y+.5-y0)/(y1-y0+1);
        i=t<0.12?2:t<0.24?4:t<0.42?3:t<0.74?2:1;}
      set(x,y,r[i]);
    }
  }
  /** Brilliant/cushion facet shader for cut stones. */
  function gem(m:Mask,r:Ramp,table:number,sq?:boolean){
    const {pts,x0,y0,x1,y1}=scan(m); if(!pts.length) return;
    const cx=(x0+x1+1)/2, cy=(y0+y1+1)/2;
    for(const [x,y] of pts){
      const dx=x+.5-cx, dy=y+.5-cy, rr=sq?Math.max(Math.abs(dx),Math.abs(dy)):Math.hypot(dx,dy);
      let i;
      if(rr<table) i=(dx+dy<-table*0.5)?4:3;
      else{const k=Math.floor((Math.atan2(dy,dx)+Math.PI)/(Math.PI/4))%8; i=(k%2)?2:1; if(dx+dy<0) i+=1;}
      if(!m(x+1,y)||!m(x,y+1)) i=Math.min(i,1);
      else if((!m(x-1,y)||!m(x,y-1))&&i<3) i=3;
      set(x,y,r[i]);
    }
  }
  function line(x0:number,y0:number,x1:number,y1:number,c:string,m?:Mask){x0=Math.round(x0);y0=Math.round(y0);x1=Math.round(x1);y1=Math.round(y1);
    if(![x0,y0,x1,y1].every(Number.isFinite)) return;
    const dx=Math.abs(x1-x0),dy=-Math.abs(y1-y0),sx=x0<x1?1:-1,sy=y0<y1?1:-1;let e=dx+dy;
    for(let guard=0;guard<200;guard++){if(!m||m(x0,y0))set(x0,y0,c);if(x0===x1&&y0===y1)break;const e2=2*e;if(e2>=dy){e+=dy;x0+=sx;}if(e2<=dx){e+=dx;y0+=sy;}}}
  const px=(x:number,y:number,c:string)=>set(x,y,c);
  const dots=(list:number[][],c:string)=>list.forEach(([x,y])=>set(x,y,c));
  return {b,f,gem,line,px,dots};
}
type Painter = ReturnType<typeof painter>;

// ── shared object builders ─────────────────────────────────────────────────
function cube(g:Painter,x:number,y:number,s:number,h:number,r:Ramp){
  g.f(P([[x,y],[x+s,y+s/2],[x+s,y+s/2+h],[x,y+h]]),r,2);
  g.f(P([[x+s,y+s/2],[x+2*s,y],[x+2*s,y+h],[x+s,y+s/2+h]]),r,1);
  g.f(P([[x,y],[x+s,y-s/2],[x+2*s,y],[x+s,y+s/2]]),r,3);
  g.line(x+1,y,x+s,y-s/2+0.5,r[4]);
}
function ear(g:Painter,bx:number,by:number,tx:number,ty:number,n:number,r:Ramp,awn?:number){
  const dx=tx-bx,dy=ty-by,L=Math.hypot(dx,dy),ux=dx/L,uy=dy/L,px=-uy,py=ux;
  g.line(bx,by,tx-ux*2,ty-uy*2,r[1]);
  for(let k=n-1;k>=0;k--){const s=L-1.2-k*2.1,cx=bx+ux*s,cy=by+uy*s;
    for(const sd of [-1,1]){g.f(C(cx+px*1.25*sd,cy+py*1.25*sd,1.35),r,"bevel");
      if(awn) g.line(cx+px*2.2*sd,cy+py*2.2*sd,cx+px*(2.2+awn*0.35)*sd+ux*awn,cy+py*(2.2+awn*0.35)*sd+uy*awn,r[3]);}}
  g.f(C(tx-ux*0.6,ty-uy*0.6,1.3),r,"bevel");
  if(awn) g.line(tx,ty,tx+ux*awn*0.8,ty+uy*awn*0.8,r[3]);
}
function logEnd(g:Painter,cx:number,cy:number,r:number){
  g.f(C(cx,cy,r),WOOD,"bevel"); g.f(C(cx,cy,r-1.2),ENDG,2);
  g.f(D(C(cx,cy,r-2.3),C(cx,cy,r-3.1)),ENDG,1); g.px(cx-0.5,cy-0.5,ENDG[1]); g.px(cx-1.5,cy-2.2,ENDG[4]);
}
function barrel(){return P([[7,4],[17,4],[19,8],[19.5,13],[19,17],[17,21],[7,21],[5,17],[4.5,13],[5,8]]);}
function barrelBody(g:Painter,r:Ramp){
  const m=barrel(); g.f(m,r,"cylV");
  for(const x of [9,12,15]) g.line(x,6,x,20,r[1],m);
  for(const y of [7,18]) g.line(4,y,20,y,STEEL[2],m);
  for(const y of [8,19]) g.line(4,y,20,y,STEEL[1],m);
}
function goldBar(g:Painter,x:number,y:number,w:number,r:Ramp){
  g.f(P([[x+0.5,y],[x+w-0.5,y],[x+w,y+4.5],[x,y+4.5]]),r,"bevel");
  g.f(P([[x+1.8,y-2.2],[x+w-1.8,y-2.2],[x+w-0.5,y],[x+0.5,y]]),r,3);
  g.line(x+2,y-2,x+w-2,y-2,r[4]); g.px(x+w/2-0.5,y+2,r[1]); g.px(x+w/2+0.5,y+2,r[1]);
}
function cutGem(g:Painter,cx:number,cy:number,rad:number,r:Ramp){g.gem(C(cx,cy,rad),r,rad*0.48); g.px(cx-rad*0.45,cy-rad*0.5,"#ffffff");}
function crystal(g:Painter,bx:number,by:number,w:number,h:number,lean:number,r:Ramp){
  const sh=(p:Pt)=>[p[0]+lean*(by-p[1]),p[1]], x0=bx-w/2, x1=bx+w/2, top=by-h-w*0.75;
  g.f(P([[x0,by],[x0,by-h],[bx,top],[bx,by]].map(sh)),r,3);
  g.f(P([[bx,by],[bx,top],[x1,by-h],[x1,by]].map(sh)),r,1);
  const t=sh([bx,top]); g.line(t[0]-0.6,t[1]+1.5,t[0]-0.6,by-2,r[4]);
}
function bowlOf(g:Painter,cx:number,cy:number,w:number,pr:Ramp){
  g.f(I(E(cx,cy+0.5,w*0.82,w*0.55),above(cy+1)),pr,"ball");
  g.f(I(E(cx,cy,w,w*0.78),below(cy+0.5)),CLAY,"ball");
  g.f(R(cx-w,cy,w*2,1.2),CLAY,3);
}

// ── the sprites ────────────────────────────────────────────────────────────
const SPR: Record<string, (g: Painter, T: Ramp) => void> = {
  // Staples
  wheat(g,T){ for(const [tx,ty] of [[5,4],[12,2],[19,4]]) ear(g,12,21,tx,ty,5,T,2.4); g.f(R(10,16,5,2),BURLAP,"bevel"); },
  rice(g,T){
    g.f(I(E(12,13.5,9,5.5),above(14)),T,"ball");
    g.dots([[7,11],[10,10],[13,9],[16,11],[9,12],[12,12],[15,12],[18,13],[11,11]],T[1]);
    g.dots([[8,10],[12,9],[14,10],[6,12]],T[4]);
    g.line(13,11,20,2,WOOD[3]); g.line(15,11,21,4,WOOD[2]);
    g.f(I(E(12,13,10,8),below(13.5)),BLUE,"ball"); g.f(R(2,13,20,1.2),BLUE,3);
    g.line(4,16,20,16,WHITE[3],I(E(12,13,10,8),below(13.5))); g.f(R(8,20,8,2),BLUE,"bevel");
  },
  barley(g,T){ ear(g,6,22,13,4,6,T,5.5); ear(g,11,22,20,7,5,T,5); g.f(R(6,18,6,2),BURLAP,"bevel"); },
  millet(g,T){
    g.f(Ln(8,17,2,11,1.8),LEAF,"bevel"); g.line(6,22,10,12,LEAF[2]);
    const pts=[[10,12],[10.5,9],[12,6.5],[14.5,5],[17,6],[18.5,8.5],[19,11.5],[18.5,14.5],[17.5,17]];
    pts.forEach(([x,y],i)=>{g.f(C(x,y,2.3-i*0.08),T,"ball"); if(i>1) g.f(C(x+1.6,y+1.2,1.4),T,"ball");});
  },
  dates(g,T){
    g.line(3,4,12,6,WOOD[2]); g.line(12,6,19,5,WOOD[2]);
    g.line(12,5,21,1,LEAF[1]); for(let t=0.2;t<1;t+=0.2){const x=12+9*t,y=5-4*t; g.line(x,y,x+1,y+2.5,LEAF[3]); g.line(x,y,x-0.5,y-2.2,LEAF[2]);}
    for(const [x,y,a] of [[8,10,0.3],[11.5,11,0],[15,10.5,-0.2],[18.5,9,-0.4],[9.5,15.5,0.25],[13,16,0],[16.5,15,-0.3],[12,20,0.1]]){g.line(12,6,x,y-2,WOOD[1]); g.f(RE(x,y,1.8,2.9,a),T,"ball");}
  },
  honey(g,T){
    g.f(Ln(13,9,21,1.5,1.8),WOOD,"bevel");
    g.f(E(11,15,7.5,6.5),CLAY,"ball"); g.f(R(5,7,12,3),CLAY,"bevel");
    g.f(E(11,7.5,4.5,1.3),T,2); g.f(P([[5,8],[8,8],[8,13],[7,14.5],[6,13.5],[5.5,11]]),T,"bevel");
    g.px(6,15,T[2]); g.dots([[13,14],[14,15],[15,14]],CLAY[3]);
  },
  // Wine, oil & vine
  wine(g,T){
    g.line(12,6,13,2,WOOD[2]); g.f(P([[13,4],[19,1.5],[21.5,5],[17.5,7.5]]),LEAF,"bevel"); g.line(14,4,20,3,LEAF[1]);
    for(const [x,y] of [[6,8.5],[10,8.5],[14,8.5],[18,8.5],[8,12],[12,12],[16,12],[10,15.5],[14,15.5],[12,19]]) g.f(C(x,y,2.2),T,"ball");
  },
  oliveoil(g,T){
    for(const s of [-1,1]) g.f(D(E(12+s*5,8,2.6,3.4),E(12+s*5,8,1.2,2)),CLAY,"bevel");
    g.f(U(E(13,12.5,6.5,7),P([[9,17],[17,17],[14,22],[12,22]])),CLAY,"ball");
    g.f(R(11,3,4,5),CLAY,"cylV"); g.f(R(10,2,6,2),CLAY,"bevel"); g.f(E(13,2.6,2,0.8),T,2);
    g.line(7,13,19,13,CLAY[1],E(13,12.5,6.5,7));
    g.line(1,21,8,15,WOOD[2]); g.f(RE(4,15.5,3.2,1.1,-0.7),ramp("#8a9a6a"),"bevel"); g.f(RE(7,19.5,3,1,0.3),ramp("#8a9a6a"),"bevel");
    g.f(C(4,19.5,1.9),T,"ball"); g.f(C(8,16,1.9),T,"ball");
  },
  citrus(g,T){
    g.f(C(12,13.5,8),T,"ball"); g.dots([[9,12],[13,10],[16,14],[11,17],[15,18],[8,16]],T[3]);
    g.px(12,5.5,WOOD[1]); g.f(RE(16,4.5,4,1.6,-0.45),LEAF,"bevel"); g.line(13,5,18,3,LEAF[1]);
  },
  beer(g,T){
    g.f(I(D(E(16.5,13.5,4.2,5),E(16.5,13.5,2,3)),(X)=>X>=15),T,"bevel");
    g.f(R(5,7,11,14),T,"cylV"); g.f(R(5,20,11,1),T,1);
    g.dots([[8,13],[11,16],[9,18],[12,11],[7,16]],T[4]);
    g.f(U(C(7,6,2.5),C(10.5,5,3),C(14,6,2.5),R(5,6,11,2)),FOAM,"ball");
    g.f(P([[12.5,7],[15,7],[15,11],[14,12],[13,11]]),FOAM,"bevel");
  },
  mead(g,T){
    g.f(taper([5,6],[5,19],[20,18],7.5,1.2),ramp("#d6b77a"),"bevel");
    g.f(taper([5,6],[5,19],[20,18],7.5,1.2,0,0.1),GOLD,"bevel");
    g.f(taper([5,6],[5,19],[20,18],7.5,1.2,0.5,0.56),GOLD,"bevel");
    g.f(taper([5,6],[5,19],[20,18],7.5,1.2,0.92,1),GOLD,"bevel");
    g.f(E(5,5.6,3.4,1.3),T,2); g.px(4,5,T[4]);
  },
  brandy(g,T){
    g.f(R(10,3,4,7),GLASS,"cylV"); g.f(R(10,1,4,3),BURLAP,"bevel");
    g.f(C(12,15.5,6.8),T,"ball"); g.f(R(8,14,8,4),PAPER,"bevel"); g.line(9,16,14,16,PAPER[1]);
    g.dots([[8,11],[8,12],[7,13]],T[4]);
  },
  citrus_liqueur(g,T){
    g.f(R(10,3,4,4),GLASS,"cylV"); g.f(R(10,1,4,2),GOLD,"bevel");
    g.f(U(R(7,9,9,13),P([[7,9.5],[16,9.5],[14,6.5],[9,6.5]])),T,"cylV");
    g.f(R(7,13,9,4),PAPER,"bevel"); g.line(8,15,14,15,PAPER[1]);
    const Y=ramp("#f4d23a"); g.f(C(18.5,18.5,4.2),Y,3); g.f(D(C(18.5,18.5,4.2),C(18.5,18.5,3.2)),Y,1);
    for(let k=0;k<6;k++){const a=k*T2/6; g.line(18.5,18.5,18.5+Math.cos(a)*2.6,18.5+Math.sin(a)*2.6,Y[4]);}
  },
  // Cash crops
  sugar(g,T){
    for(const [x0,y0,x1,y1] of [[6,22,9,4],[12,22,12,3],[18,22,15,5]]){g.f(Ln(x0,y0,x1,y1,2.8),T,"cylV");
      for(let t=0.18;t<0.98;t+=0.21){const x=x0+(x1-x0)*t,y=y0+(y1-y0)*t; g.line(x-1.4,y,x+1.4,y,T[1]);}}
    g.f(Ln(9,5,3,1.5,1.6),LEAF,"bevel"); g.f(Ln(12,4,18,1,1.6),LEAF,"bevel"); g.f(Ln(15,6,21,4,1.6),LEAF,"bevel"); g.f(Ln(12,9,5,10,1.4),LEAF,"bevel");
  },
  refined_sugar(g,T){
    g.f(P([[12,1.5],[13.4,3],[18.5,20],[5.5,20],[10.6,3]]),T,"cylV");
    g.f(P([[6.2,15],[17.8,15],[18.8,21],[5.2,21]]),BLUE,"bevel"); g.line(6,17,18,17,WHITE[3]); g.dots([[12,18],[11,18]],WHITE[3]);
  },
  tobacco(g,T){
    for(const [cx,cy,a] of [[8,10,-0.38],[16,10,0.38],[12,9,0]]){g.f(RE(cx,cy,3.1,8.6,a),T,"bevel"); g.line(12,18,cx+8.6*Math.sin(a),cy-8.6*Math.cos(a),T[1]);}
    g.line(11,20,10,22,T[1]); g.line(13,20,14,22,T[1]); g.f(R(10,17,5,3),RED,"bevel");
  },
  indigo(g,T){
    cube(g,2,14,5,5,T); cube(g,12,14,5,5,T); cube(g,7,9,5,5,T);
    g.dots([[3,22],[6,23],[18,22],[21,22],[10,22]],T[3]);
  },
  coffee(g,T){
    for(const [x,y,a] of [[9.5,7.5,-0.5],[15.5,7,0.35],[6.5,14,0.6],[17.5,14,0.7],[12,18,-0.3]]){
      g.f(RE(x,y,3,4,a),T,"ball"); const s=Math.sin(a),c=Math.cos(a); g.line(x+s*2.6,y-c*2.6,x-s*2.6,y+c*2.6,T[0]);}
  },
  tea(g,T){
    g.line(6,21,10,5,T[1]);
    g.f(RE(6,11,2.4,4.6,-1.0),T,"bevel"); g.f(RE(13,8.5,2.4,4.6,0.95),T,"bevel"); g.f(RE(6.5,17,2.2,4,-1.1),T,"bevel"); g.f(RE(10.5,3.5,1.4,2.6,0.2),T,3);
    g.f(I(E(16.5,16,5.6,5.2),below(16)),WHITE,"ball"); g.f(E(16.5,16,5.2,1.3),ramp("#b0802a"),2); g.f(R(14,20.5,5,1),WHITE,1);
    g.dots([[15,13],[16,12],[18,13],[17,11]],WHITE[2]);
  },
  cacao(g,T){
    g.line(15,4,18,1,WOOD[2]);
    g.f(RE(11,12,5.5,9.5,0.55),T,"ball");
    const a=0.55,s=Math.sin(a),c=Math.cos(a);
    for(const o of [-2.4,0,2.4]) g.line(11+c*o+s*7.5,12+s*o-c*7.5,11+c*o-s*7.5,12+s*o+c*7.5,T[1],RE(11,12,5.3,9.3,0.55));
    const B=ramp("#c08a62"); g.f(RE(18,18.5,2,2.8,0.4),B,"ball"); g.f(RE(20.5,15,1.8,2.5,-0.3),B,"ball");
  },
  // Spices & aromatics
  spices(g,T){
    g.f(Ln(13,11,19,2.5,3),STONE,"bevel"); g.f(C(19,2.8,1.8),STONE,"ball");
    g.f(I(E(12,13,7,3),above(13.5)),T,"ball");
    g.f(I(E(12,13.5,9,7.5),below(13.5)),STONE,"ball"); g.f(R(3,13,18,1.6),STONE,3); g.f(R(8,20,8,2),STONE,"bevel");
    g.dots([[6,12],[18,12],[15,11]],T[3]);
  },
  cloves(g,T){
    for(const [hx,hy,tx,ty] of [[6,5,9,19],[12,4,12,20],[18,5,15,19],[4,18,19,15]]){
      g.f(Ln(hx,hy,tx,ty,2),T,"bevel"); g.f(C(hx,hy,2.1),T,"ball"); g.dots([[hx-1.5,hy-1.5],[hx+1,hy-1.5]],T[3]);}
  },
  pepper(g,T){
    const rows:[number,number[]][]=[[19,[4,8,12,16,20]],[15.5,[6,10,14,18]],[12,[8,12,16]],[8.5,[10,14]],[5,[12]]];
    for(const [y,xs] of rows) for(const x of xs){g.f(C(x,y,2),T,"ball"); g.px(x,y+0.5,T[1]);}
  },
  cinnamon(g,T){
    for(const [o] of [[0],[4],[8]]){const x0=3+o*0.2,y0=19-o,x1=19+o*0.2,y1=9-o;
      g.f(Ln(x0,y0,x1,y1,3.6),T,"bevel"); g.f(C(x0,y0,1.8),T,1); g.px(x0,y0,T[3]);}
    g.line(9,19,13,6,RED[2]); g.line(10,19,14,6,RED[1]);
  },
  frankincense(g,T){
    g.f(E(12,19,9.5,3),WOOD,"bevel");
    for(const [x,y,r] of [[10,10.5,2.6],[15,10,2.4],[7,15,3],[12,14.5,3.3],[17,15,2.8]]){g.f(U(C(x,y,r),C(x+r*0.4,y+r*0.5,r*0.7)),T,"ball"); g.px(x-r*0.4,y-r*0.5,T[4]);}
  },
  incense(g,T){
    for(const [x0,x1,y1] of [[10,7,5],[12,12,4],[14,17,6]]){g.line(x0,14,x1,y1,WOOD[1]); g.px(x1,y1,FLAME[3]);
      g.dots([[x1,y1-1.5],[x1+1,y1-2.5],[x1,y1-3.5],[x1-1,y1-4.5]].filter(p=>p[1]>=0),T[3]);}
    g.f(U(I(E(12,17,6.5,5),below(15)),R(5,14,14,2)),CLAY,"ball"); g.f(E(12,14.6,5.8,1.1),BURLAP,3);
  },
  saffron(g,T){
    const V=ramp("#8e5cc8");
    g.line(12,22,12,13,LEAF[2]); g.line(10,22,6,12,LEAF[3]); g.line(14,22,18,12,LEAF[3]);
    g.f(RE(8.5,9.5,2.6,5,-0.5),V,"ball"); g.f(RE(15.5,9.5,2.6,5,0.5),V,"ball"); g.f(RE(12,8.5,2.9,5.6,0),V,"ball");
    for(const [x,y] of [[8,2],[12,1],[16,2]]){g.line(12,9,x,y,RED[2]); g.px(x,y,RED[3]);}
    g.dots([[11,7],[13,7],[12,6]],T[3]); g.dots([[4,20],[5,21],[19,21],[20,20]],T[2]);
  },
  perfume(g,T){
    g.f(R(11,5,2,5),GLASS,"cylV"); g.f(R(10,8,4,1),GOLD,3);
    g.f(C(12,15,6.8),T,"ball"); g.line(9,11,9,14,T[4]);
    g.f(tear(12,4.2,1.9),GOLD,"ball");
  },
  nutmeg(g,T){
    g.f(E(10,11.5,6,7.5),T,"ball"); g.dots([[8,8],[9,9],[11,10],[12,11],[8,13],[9,14],[12,15],[7,11]],T[1]);
    g.f(C(17,17,4.6),T,"bevel"); g.f(C(17,17,3.4),ramp("#d8b88a"),2);
    g.dots([[16,16],[17,17],[18,16],[16,18],[18,18],[17,15]],T[1]);
  },
  mace(g,T){
    const seed=E(12,12.5,6.8,8.8); g.f(seed,ramp("#5a321a"),"ball");
    for(const [x,y] of [[6.5,17],[9.5,20.5],[14.5,20.5],[17.5,17]]) g.f(I(Ln(12,4,x,y,1.8),seed),T,"bevel");
    g.f(I(Ln(5,10,19,10,1.5),seed),T,"bevel"); g.f(I(Ln(6,15,18,15,1.5),seed),T,"bevel");
    g.f(C(12,4,1.8),T,"ball");
  },
  dragons_blood(g,T){ g.f(tear(9,15.5,4.6),T,"ball"); g.f(tear(17,18,3),T,"ball"); g.f(tear(17,9,2.3),T,"ball"); g.dots([[7,13],[16,16],[16,8]],T[4]); },
  camphor(g,T){
    cube(g,3,9,8,7,T); g.dots([[7,6],[12,5],[10,8]],WHITE[4]);
    g.f(RE(18,18.5,2.3,4.4,0.9),LEAF,"bevel"); g.f(RE(20.5,14.5,1.8,3.4,-0.3),LEAF,"bevel"); g.line(14,21,20,17,LEAF[1]);
  },
  benzoin(g,T){
    for(const [x,y,r] of [[12,8.5,3.8],[7.5,15,4.5],[16,16,4.2]]){g.f(U(C(x,y,r),C(x+r*0.5,y-r*0.3,r*0.6)),T,"ball");
      g.line(x-r*0.4,y,x+r*0.3,y+r*0.35,WHITE[3]);}
  },
  sandalwood(g,T){
    g.f(R(3,8,14,9),T,"cylH"); for(const y of [10,13,15]) g.line(4,y,15,y,T[1]);
    g.f(E(17,12.5,3,4.6),T,1); g.f(E(17,12.5,2.3,3.7),T,4);
    g.f(D(E(17,12.5,1.6,2.5),E(17,12.5,0.8,1.3)),T,2);
    for(const [x,y] of [[5,20],[10,21],[15,20],[19,21]]) g.f(P([[x,y-1],[x+2,y],[x,y+1.2],[x-1.5,y]]),T,3);
  },
  // Textiles & animal
  silk(g,T){
    g.f(E(12,18.5,6.5,2),WOOD,"bevel");
    g.f(R(7,5,10,14),T,"cylV"); for(let y=7;y<18;y+=2) g.line(7,y,16,y,T[1]);
    g.f(E(12,5,6.5,2),WOOD,"bevel"); g.f(E(12,5,1.5,0.8),WOOD,0);
    g.line(17,12,21,21,T[3]); g.f(RE(4.5,19,2.3,3.3,0.6),WHITE,"ball");
  },
  cotton(g,T){
    for(const pts of [[[12,21],[3,15],[9,13]],[[12,21],[21,15],[15,13]],[[12,21],[10,15],[14,15]]]) g.f(P(pts),WOOD,"bevel");
    g.line(12,21,12,23,WOOD[1]);
    for(const [x,y,r] of [[12,6.5,3.8],[8,10,4.2],[16,10,4.2],[8.5,15,3.9],[15.5,15,3.9]]) g.f(C(x,y,r),T,"ball");
  },
  flax(g,T){
    for(let i=-3;i<=3;i++) g.line(12+i*0.6,21,12+i*2.3,6,i%2?BURLAP[3]:BURLAP[2]);
    g.f(R(9,14,7,2.4),T,"bevel");
    const F=ramp("#6f8fe0");
    for(let i=-3;i<=3;i++){const x=12+i*2.3; if(i%2) {g.f(C(x,5,1.7),F,"ball"); g.px(x,5,GOLD[3]);} else g.f(C(x,5.5,1.3),BURLAP,"ball");}
  },
  wool_fleece(g,T){
    for(const [x,y,r] of [[7,9,3.6],[12,8,3.8],[17,9,3.5],[4.5,13.5,3.2],[9.5,13,3.8],[14.5,13,3.8],[19.5,13.5,3],[7,18,3.5],[12,18,3.8],[17,18,3.4]]){g.f(C(x,y,r),T,"ball"); g.px(x+0.8,y+0.8,T[1]);}
  },
  wool_llama(g,T){
    g.f(Ln(14,2,20,13,1.4),WOOD,3); g.f(Ln(20,2,14,13,1.4),WOOD,3);
    const m=C(11,13.5,8); g.f(m,T,"ball");
    for(const [a,b,c,d] of [[4,10,17,19],[5,7,15,21],[6,17,17,6],[9,20,19,10],[3,13,11,5]]) g.line(a,b,c,d,T[1],m);
    g.line(17,19,22,22,T[2]); g.dots([[20,1],[14,1]],WOOD[4]);
  },
  furs(g,T){
    g.f(taper([13,19],[20,23],[21,14],3.4,2.2),T,"bevel"); g.f(C(21,14,1.6),WHITE,"ball");
    g.f(P([[11,2],[13,3],[14,1.5],[15,5],[16,7],[20,8],[19,10.5],[16,11],[15,14],[20,16],[19,19],[15,18],[13,20],[9,20],[7,18],[3,19],[2,16],[7,14],[6,11],[3,10.5],[2,8],[6,7],[7,5],[8,1.5],[9,3]]),T,"bevel");
    g.line(11,5,11,18,T[1]); g.dots([[9,4],[13,4]],T[0]); g.dots([[11,3],[11,2]],T[0]);
  },
  hides(g,T){
    g.f(P([[4,4],[9,5],[12,3],[15,5],[20,4],[19,9],[21,13],[19,17],[20,21],[15,19],[12,21],[9,19],[4,21],[5,16],[3,12],[5,8]]),T,"bevel");
    g.f(C(9,10,2.6),T,1); g.f(C(15,13,2.1),T,1); g.f(C(10.5,16,1.8),T,1);
    g.dots([[4,4],[20,4],[4,21],[20,21],[12,3],[12,21]],BURLAP[3]);
  },
  horses(g,T){
    g.f(P([[6,19],[3,17],[3,14],[7,9],[10,5],[12,1.5],[13,4],[15,2.5],[16,6],[19,10],[20.5,15],[20,21],[12,21],[11,17],[8,19]]),T,"bevel");
    g.f(Ln(14,4,20,13,2.4),ramp("#3a2418"),"bevel"); g.f(Ln(11,4,9,7,1.6),ramp("#3a2418"),2);
    g.px(9,9,"#140a08"); g.px(8,9,"#f4ece0"); g.px(4,15,T[0]); g.line(6,12,11,15,RED[2]);
  },
  ivory(g,T){
    g.f(taper([17,21],[1,18],[5,3],6,0.9),T,"bevel");
    g.f(E(17,20.5,3,1.4),T,1);
    for(const t of [0.3,0.5,0.7]){const u=1-t,x=u*u*17+2*u*t*1+t*t*5,y=u*u*21+2*u*t*18+t*t*3; g.px(x+1,y,T[1]);}
  },
  cloth(g,T){
    const bolt=(x:number,y:number,w:number,h:number)=>{g.f(R(x,y,w,h),T,"cylH"); for(let k=x+3;k<x+w;k+=4) g.line(k,y,k,y+h-1,T[1]);
      g.f(E(x+w,y+h/2,2.2,h/2),T,3); g.f(D(E(x+w,y+h/2,1.4,h/2-1.2),E(x+w,y+h/2,0.6,1)),T,1);};
    bolt(4,4,14,7); bolt(2,12,16,8);
    g.f(P([[3,19],[15,19],[16,22.5],[5,22.5]]),T,"bevel");
  },
  linen(g,T){
    for(const [x,y,w] of [[3,16,17],[4,11,16],[3,6,17]]){g.f(R(x,y,w,5),T,"bevel"); g.line(x+2,y+2,x+w-3,y+2,T[1]);}
    g.f(R(11,6,2,15),BLUE,2); g.dots([[10,5],[13,5],[9,4],[14,4]],BLUE[3]);
  },
  cotton_cloth(g,T){
    g.f(P([[8,3],[10,4],[14,4],[16,3],[21.5,6.5],[19.5,11.5],[17,10.5],[17,21],[7,21],[7,10.5],[4.5,11.5],[2.5,6.5]]),T,"bevel");
    g.f(P([[10,4],[14,4],[12,7.5]]),T,1); g.line(12,8,12,16,T[1]); g.dots([[13,10],[13,13]],T[4]);
    g.line(7,10,7,12,T[1]); g.line(17,10,17,12,T[1]);
  },
  silk_brocade(g,T){
    const m=P([[5,4],[19,4],[19,18],[17,20.5],[15,18],[13,20.5],[11,18],[9,20.5],[7,18],[5,20.5]]); g.f(m,T,"bevel");
    for(let y=6;y<18;y++) for(let x=6;x<19;x++) if(((x+y)%4===0||(x-y+40)%4===0)&&(x+y)%2===0&&m(x,y)) g.px(x,y,GOLD[3]);
    g.f(Ln(3,3.5,21,3.5,2),WOOD,"bevel"); g.dots([[2,3],[21,3]],GOLD[3]);
    g.dots([[5,21],[9,21],[13,21],[17,21]],GOLD[3]);
  },
  carpets(g,T){
    g.f(R(3,5,18,14),T,"bevel"); g.f(R(5,7,14,10),T,2);
    g.line(5,7,18,7,GOLD[2]); g.line(5,16,18,16,GOLD[2]); g.line(5,7,5,16,GOLD[2]); g.line(18,7,18,16,GOLD[2]);
    g.f(P([[12,8.5],[15.5,12],[12,15.5],[8.5,12]]),GOLD,"bevel"); g.f(P([[12,10.5],[13.5,12],[12,13.5],[10.5,12]]),BLUE,2);
    for(let y=6;y<19;y+=2){g.px(2,y,PAPER[3]); g.px(21,y,PAPER[3]);}
    g.dots([[7,9],[16,9],[7,14],[16,14]],BLUE[3]);
  },
  leather_goods(g,T){
    g.f(P([[8,2],[15,2],[15,14],[21,16],[21.5,20],[6,20],[6,15],[8,13]]),T,"bevel");
    g.f(R(6,20,16,2),WOOD,0); g.f(R(8,2,7,2),T,3); g.dots([[13,6],[13,8],[13,10],[13,12]],T[4]); g.line(9,5,9,12,T[1]);
  },
  hemp(g,T){
    g.f(D(C(12,12.5,8.5),C(12,12.5,5.8)),T,"ball"); g.f(D(C(12,12.5,5.2),C(12,12.5,2.6)),T,"ball");
    for(let k=0;k<12;k++){const a=k*T2/12; g.px(12+Math.cos(a)*7.1,12.5+Math.sin(a)*7.1,T[1]); if(k%2) g.px(12+Math.cos(a)*3.9,12.5+Math.sin(a)*3.9,T[1]);}
    g.f(Ln(17,19,22,22.5,2),T,"bevel");
  },
  // Forestry & craft
  timber(g,T){
    logEnd(g,7.5,16.5,4.6); logEnd(g,16.5,16.5,4.6); logEnd(g,12,9,4.6);
    g.line(15,5,21,1,WOOD[1]); g.dots([[16,3],[17,5],[18,2],[19,4],[20,1],[21,3],[17,3],[19,2]],T[3]);
  },
  hardwoods(g,T){
    g.f(P([[3,12],[15,5],[21,8.5],[9,15.5]]),T,3);
    g.f(P([[9,15.5],[21,8.5],[21,13.5],[9,20.5]]),T,2);
    for(const o of [1.5,3.2]) g.line(10,15.5+o,20,9.5+o,T[1]);
    g.f(P([[3,12],[9,15.5],[9,20.5],[3,17]]),ramp(mix(T[2],"#d8b07a",0.35)),2);
    g.dots([[5,15],[6,16],[6,15],[5,16]],ramp(mix(T[2],"#d8b07a",0.35))[1]);
    g.line(4,12.2,14,6.5,T[4]);
  },
  paper(g,T){
    g.f(R(5,6,14,12),T,"bevel"); for(const y of [8,10,12,14]) g.line(7,y,y===14?12:16,y,T[1]);
    g.f(R(3,3,18,3),T,"cylH"); g.f(R(3,17,18,3),T,"cylH"); g.f(C(16,15,1.7),RED,"ball");
    g.dots([[3,4],[20,4],[3,18],[20,18]],T[1]);
  },
  clay(g,T){
    g.f(U(E(12,16,8.5,5.5),C(9,11.5,4),C(14.5,11,4.5)),T,"ball");
    g.line(8,15,11,17,T[1]); g.line(13,14,16,16,T[1]); g.dots([[10,9],[15,8]],T[4]);
    g.f(P([[15,17],[21,17],[21,20],[15,20]]),T,"bevel");
  },
  ceramics(g,T){
    const body=U(E(12,14,6.5,6.5),R(10,5,4,4)); g.f(body,WHITE,"ball"); g.f(E(12,4.6,3.9,1.3),WHITE,"bevel"); g.f(R(9,20,6,2),WHITE,"bevel");
    g.line(4,10,20,10,T[2],body); g.line(4,18,20,18,T[2],body); g.line(9,6,15,6,T[2],body);
    g.px(12,14,T[1]); g.dots([[11,13],[13,13],[11,15],[13,15]],T[2]); g.dots([[9,15],[15,13]],T[3]);
  },
  glassware(g,T){
    g.f(E(12,20,5.6,1.6),T,"bevel"); g.f(R(11,11,2,8),T,"cylV"); g.f(C(12,14.5,1.5),T,"ball");
    g.f(P([[6,2],[18,2],[17,7],[14,11],[10,11],[7,7]]),T,"ball"); g.dots([[8,4],[8,5],[9,6]],T[4]); g.line(6,2,18,2,T[4]);
  },
  books(g,T){
    const book=(x:number,y:number,w:number,h:number,r:Ramp)=>{g.f(R(x,y,w,h),r,"bevel"); g.f(R(x+2,y+1,w-2,h-2),PAPER,2); g.line(x+2,y+2,x+w-1,y+2,PAPER[1]); g.f(R(x,y,2,h),r,"bevel");};
    book(3,16,18,5,T); book(5,11,15,5,RED); book(4,6,16,5,BLUE);
    g.px(4,8,GOLD[3]); g.px(6,13,GOLD[3]); g.px(4,18,GOLD[3]); g.line(15,6,15,3,RED[2]);
  },
  furniture(g,T){
    g.f(R(5,14,2,7),T,1); g.f(R(15,12,2,5),T,1);
    g.f(D(R(6,2,11,10),U(R(8,4,2,7),R(11,4,2,7),R(14,4,1.5,7))),T,"bevel");
    g.f(R(7,15,2,7),T,"bevel"); g.f(R(17,15,2,7),T,"bevel");
    g.f(P([[5,12],[17,12],[19.5,15],[7,15]]),T,3); g.f(R(7,15,13,1.5),T,1);
    g.f(P([[7,12.5],[16.5,12.5],[18,14.3],[8.5,14.3]]),RED,3);
  },
  candles(g,T){
    g.f(E(12,21,9.5,1.8),GOLD,"bevel");
    g.f(R(6,7,4,14),T,"cylV"); g.f(R(13,11,4,10),T,"cylV");
    g.dots([[6,8],[6,9],[13,12],[16,12],[16,13]],T[4]);
    g.px(8,6,WOOD[0]); g.px(15,10,WOOD[0]);
    g.f(tear(8,4.6,1.4),FLAME,"ball"); g.f(tear(15,8.6,1.4),FLAME,"ball");
  },
  soap(g,T){
    g.f(P([[3,12],[12,16],[12,20],[3,16]]),T,2); g.f(P([[12,16],[21,12],[21,16],[12,20]]),T,1);
    g.f(P([[3,12],[12,8],[21,12],[12,16]]),T,3); g.f(D(P([[7,12],[12,10],[17,12],[12,14]]),P([[9,12],[12,11],[15,12],[12,13]])),T,2);
    for(const [x,y,r] of [[17,6,2.6],[8,6,1.8],[13,3,1.3]]){g.f(D(C(x,y,r),C(x,y,r-0.9)),WHITE,3); g.px(x-r*0.5,y-r*0.5,WHITE[4]);}
  },
  statuary(g,T){
    g.f(R(7,17,10,5),STONE,"bevel"); g.f(R(6,16,12,1.5),STONE,3);
    g.f(U(I(E(12,16,6.2,3.4),above(16.5)),R(10.5,10,3,4),E(12,7.6,3.6,4.3),P([[8.4,8],[7.6,9.4],[8.6,9.6]])),T,"ball");
    g.px(10,7,T[1]); g.line(9,4,15,4,T[1]);
  },
  ivory_carvings(g,T){
    g.f(U(E(12,20,6,1.8),R(7,18,10,2)),T,"bevel");
    g.f(P([[9,18],[15,18],[14,11],[10,11]]),T,"cylV"); g.f(R(8,9,8,2),T,"bevel");
    g.f(C(12,6.8,3),T,"ball"); g.f(R(11,0.5,2,4),T,3); g.f(R(9.5,1.5,5,1.5),T,3);
    g.line(10,14,14,14,T[1]);
  },
  pitch(g,T){
    barrelBody(g,T); g.f(E(12,4.5,5,1.3),T,3);
    g.f(P([[14.5,4],[17,4],[17,9],[16,10.5],[15,9]]),T,0); g.px(15,6,T[4]);
  },
  // Minerals & metals
  salt(g,T){ cube(g,6,10,5.5,5.5,T); cube(g,2,15,4.5,4.5,T); cube(g,12,16,4.5,4,T); g.dots([[8,9],[4,14],[14,15]],"#ffffff"); },
  bay_salt(g,T){
    g.f(Ln(13,11,20,3,1.8),WOOD,"bevel"); g.f(E(20,3,1.8,1.2),WOOD,3);
    g.f(P([[2,21],[5,16],[9,12],[12,10],[15,12],[19,16],[22,21]]),T,"ball");
    g.dots([[9,14],[12,12],[15,15],[6,18],[17,19],[11,17]],"#ffffff"); g.dots([[8,17],[14,18],[18,17]],T[1]);
  },
  iron(g,T){
    g.f(P([[4,21],[5,15],[9,11.5],[15,12],[19.5,15],[20,21]]),STONE,"bevel");
    g.dots([[8,15],[9,15],[13,17],[14,17],[10,19],[16,15],[17,18]],"#9a4a2a");
    g.f(Ln(7,20,17,5,1.8),WOOD,"bevel");
    g.f(P([[10,5.5],[14,3],[18,3],[22,6.5],[18,5.2],[14,5.4]]),T,"bevel");
  },
  copper(g,T){
    g.f(P([[3,6],[8,8],[16,8],[21,6],[19,12],[21,18],[16,16],[8,16],[3,18],[5,12]]),T,"bevel");
    g.dots([[9,11],[13,10],[15,13],[11,14],[7,13]],T[1]); g.line(6,8,17,8.5,T[4]);
  },
  tin(g,T){
    g.f(P([[4,11],[18,8],[21,17],[7,20]]),T,2); g.line(6,12,18,9.5,T[4]); g.line(7,19,20,16.5,T[1]);
    g.f(Ln(5,9,18,6,5.5),T,"cylH"); g.f(E(5,9,2.5,2.8),T,3); g.f(D(E(5,9,1.5,1.7),E(5,9,0.7,0.8)),T,1);
  },
  lead(g,T){
    g.f(P([[3,12],[21,12],[19,20],[5,20]]),T,"bevel"); g.f(P([[5,8],[19,8],[21,12],[3,12]]),T,3);
    g.line(6,8.5,18,8.5,T[4]); g.line(10,14,14,18,T[1]); g.line(14,14,10,18,T[1]);
  },
  gold(g,T){ goldBar(g,1.5,16,10.5,T); goldBar(g,12,16,10.5,T); goldBar(g,6.8,10.5,10.5,T); g.px(9,8,"#ffffff"); },
  silver(g,T){
    g.f(U(R(3,11,11,9),E(8.5,20,5.5,1.8)),T,"cylV"); for(let y=13;y<20;y+=2.4) g.line(3,y,13,y,T[1]);
    g.f(E(8.5,11,5.5,1.8),T,3); g.px(7,10.5,T[4]);
    g.f(E(17.5,14.5,4,5.6),T,"ball"); g.f(D(E(17.5,14.5,3,4.4),E(17.5,14.5,2.3,3.6)),T,1); g.dots([[17,13],[18,14],[17,15]],T[4]);
  },
  gemstones(g,T){ cutGem(g,8,15.5,4.6,RED); cutGem(g,15.5,16.5,4.6,BLUE); cutGem(g,12,8.5,4.2,T); cutGem(g,19,9,3,ramp("#2cc86a")); },
  jade(g,T){
    g.f(D(C(12,11.5,8.5),C(12,11.5,3)),T,"ball"); g.f(D(C(12,11.5,6.2),C(12,11.5,5.4)),T,1);
    g.dots([[12,5],[17,9],[17,14],[12,18],[7,14],[7,9]],T[3]);
    g.line(12,20,12,22,RED[2]); g.dots([[11,22],[13,22]],RED[2]);
  },
  ruby(g,T){ cutGem(g,12,12,9.5,T); g.dots([[7,6],[6,7],[8,7],[7,8]],"#ffffff"); },
  sapphire(g,T){ g.gem(SE(12,12,9,8,3.2),T,4.2,true); g.dots([[7,6],[6,7],[8,7],[7,8]],"#ffffff"); },
  emerald(g,T){
    g.f(oct(4,3,16,18,2.5),T,"bevel"); g.f(oct(6,5,12,14,2),T,1); g.f(oct(7,6,10,12,1.6),T,3); g.f(oct(8.5,7.5,7,9,1),T,2);
    g.line(4.5,5,8,7.5,T[4]); g.line(19.5,19,16,16,T[0]); g.line(8,6,11,6,T[4]);
  },
  diamond(g,T){
    g.f(P([[3,9],[21,9],[12,21.5]]),T,2); g.f(P([[6.5,5],[17.5,5],[21,9],[3,9]]),T,3);
    for(const x of [7,12,17]) g.line(12,21,x,9,T[1]);
    g.line(6.5,5,8,9,T[1]); g.line(17.5,5,16,9,T[1]); g.line(10,5,12,9,T[4]); g.line(14,5,12,9,T[1]); g.line(7,5,17,5,T[4]);
    g.dots([[9,12],[15,13]],"#f0b8d8"); g.dots([[13,11],[10,15]],"#a8d8ff"); g.dots([[6,7],[5,6],[7,6],[6,5]],"#ffffff");
  },
  amethyst(g,T){
    g.f(E(12,20.5,9.5,2.5),STONE,"bevel");
    crystal(g,7,20,4.6,7,-0.35,T); crystal(g,17,20,4.6,6,0.35,T); crystal(g,12,20,6,9,0,T);
  },
  topaz(g,T){ g.gem(tear(12,15,5.8),T,3.2); g.dots([[9,11],[10,10]],"#ffffff"); },
  garnet(g,T){ cutGem(g,8,15,4.4,T); cutGem(g,16,14.5,4.4,T); cutGem(g,12,7.5,3.8,T); },
  carnelian(g,T){
    g.f(E(12,12.5,8.5,7),T,"ball"); g.f(D(E(12,12.5,6.4,5),E(12,12.5,5.4,4.1)),T,3); g.f(D(E(12,12.5,3.6,2.8),E(12,12.5,2.6,2)),T,3);
    g.dots([[8,8],[9,8]],T[4]);
  },
  turquoise(g,T){
    g.f(R(11,2,2,3),STEEL,"bevel");
    g.f(E(12,12.5,9,8),STEEL,"bevel"); g.f(E(12,12.5,7.5,6.5),T,"ball");
    const m=E(12,12.5,7,6); g.line(6,10,9,12,WOOD[1],m); g.line(9,12,8,16,WOOD[1],m); g.line(13,8,15,11,WOOD[1],m); g.line(15,11,18,12,WOOD[1],m); g.line(12,13,14,17,WOOD[1],m);
    for(let k=0;k<10;k++){const a=k*T2/10; g.px(12+Math.cos(a)*8.3,12.5+Math.sin(a)*7.4,STEEL[4]);}
  },
  marble(g,T){
    cube(g,3,8,9,9,T);
    g.line(5,13,8,16,STONE[2]); g.line(8,16,10,20,STONE[2]); g.line(8,6,12,8,STONE[2]); g.line(12,8,15,7,STONE[2]); g.line(15,12,17,16,STONE[1]); g.line(17,16,19,17,STONE[1]);
  },
  lapis_lazuli(g,T){
    g.f(P([[4,12],[7,6],[13,4],[19,7],[21,13],[17,19],[9,20],[5,17]]),T,"bevel");
    g.f(P([[7,6],[13,4],[19,7],[14,10],[8,10]]),T,3);
    g.dots([[10,13],[15,12],[12,16],[17,15],[8,15],[11,7]],GOLD[3]); g.line(6,15,11,18,WHITE[2]);
  },
  alum(g,T){
    g.f(P([[11,2],[11,12],[3.5,12]]),T,3); g.f(P([[11,2],[18.5,12],[11,12]]),T,2);
    g.f(P([[3.5,12],[11,12],[11,21]]),T,2); g.f(P([[11,12],[18.5,12],[11,21]]),T,1);
    g.line(11,3,11,20,T[4]);
    g.f(P([[18,14],[21,17.5],[18,21],[15,17.5]]),T,"bevel");
  },
  mercury(g,T){
    g.f(E(12,20.5,9,1.8),T,"bevel");
    for(const [x,y,r] of [[10,14,5],[17,17,3],[15.5,8.5,2],[5.5,19,1.8],[20,11,1.3]]){g.f(C(x,y,r),T,"ball"); g.px(x-r*0.45,y-r*0.45,"#ffffff");}
  },
  bog_iron(g,T){
    g.line(4,21,5,7,LEAF[2]); g.line(6,21,8,9,LEAF[3]); g.f(RE(5,7,1.2,2.4,0.05),ramp("#6a4020"),"bevel");
    for(const [x,y,r] of [[11,16.5,4.3],[17.5,17,3.6],[14.5,11,3.4],[19,11.5,2.4]]){g.f(U(C(x,y,r),C(x-r*0.5,y+r*0.4,r*0.6)),T,"ball");}
    g.dots([[10,15],[12,17],[17,16],[15,10],[19,11]],"#b0602a");
  },
  coal(g,T){
    g.f(P([[6,13],[9,7],[14,6],[16.5,11],[11,14]]),T,"bevel");
    g.f(P([[3,20.5],[4,15],[8,13],[11.5,16],[10.5,21]]),T,"bevel");
    g.f(P([[9,21],[10,14.5],[15,11],[19.5,14],[20.5,21]]),T,"bevel");
    g.dots([[9,8],[10,8],[5,15],[11,15],[15,12],[16,12]],T[4]);
  },
  metalware(g,T){
    g.f(C(8.5,11,6.8),WOOD,"ball"); g.f(D(C(8.5,11,6.8),C(8.5,11,5.8)),STEEL,2); g.f(C(8.5,11,2),STEEL,"ball");
    g.f(Ln(9,16,20,3,2.2),T,"bevel"); g.line(10,15,19,4,T[4]);
    g.f(Ln(5.5,14,12,19.5,1.8),GOLD,"bevel"); g.f(Ln(7,18,4,21,1.8),WOOD,"bevel"); g.f(C(3.5,21.5,1.4),GOLD,"ball");
  },
  bronzeware(g,T){
    g.f(D(C(12,3.2,2.2),C(12,3.2,1)),T,"bevel");
    g.f(U(I(C(12,10,5.2),above(10)),P([[6.8,10],[17.2,10],[19,18],[5,18]]),E(12,18,7.6,1.8)),T,"cylV");
    g.line(6,13,18,13,T[1],P([[6.8,10],[17.2,10],[19,18],[5,18]])); g.line(5,16,19,16,T[3],P([[6.8,10],[17.2,10],[19,18],[5,18]]));
    g.f(C(12,20.5,1.8),T,1);
  },
  jewelry(g,T){
    g.f(D(E(12,15,7.5,6.3),E(12,15,5,4.3)),T,"ball");
    g.f(P([[9,9],[15,9],[14,11],[10,11]]),T,"bevel");
    cutGem(g,12,6.5,3.4,RED);
  },
  // Marine
  stockfish(g,T){
    for(const x of [7.5,16.5]){const m=fish(x,13.5,15,5.2,Math.PI/2); g.f(m,T,"bevel"); for(let y=9;y<19;y+=2.5) g.line(x-1.5,y,x+1.5,y,T[1],m); g.px(x-1,19.5,T[0]);}
    g.f(Ln(2,3,22,3,1.8),WOOD,"bevel"); g.dots([[7,4],[17,4]],BURLAP[3]);
  },
  herring(g,T){
    const m=fish(13,12,18,8,0); g.f(m,T,"ball"); g.f(I(m,above(9.5)),T,1);
    g.line(15,9,15,14,T[1],m); g.px(18,11,"#101418"); g.px(18,10,"#ffffff");
    g.f(P([[10,15.5],[13,15.5],[11,18]]),T,1); g.f(P([[10,8.5],[14,8.5],[11,6]]),T,1);
    g.dots([[8,12],[10,11],[12,13],[9,14],[11,12]],T[4]);
  },
  salted_herring(g,T){
    barrelBody(g,WOOD); g.f(E(12,5,6.5,2),WHITE,3);
    g.f(P([[8,6],[6.5,1.5],[9.3,3.3],[11,1],[10.8,6]]),T,"bevel"); g.f(P([[14,5.5],[13.5,1],[15.6,2.8],[18,1.5],[16.8,6]]),T,"bevel");
    g.dots([[7,5],[12,5],[17,5]],"#ffffff");
  },
  pearls(g,T){
    const S=ramp("#9a8aa6");
    g.f(I(E(12,11,9.5,6.5),above(11.5)),S,"bevel"); g.f(I(E(12,11,7.8,4.8),above(11.5)),ramp("#e8e0f0"),2);
    g.f(I(E(12,14.5,10,6.5),below(14)),S,"ball"); for(const x of [6,9,12,15,18]) g.line(12,14,x,20,S[1],I(E(12,14.5,10,6.5),below(14.8)));
    g.f(R(2,13.5,20,1.2),ramp("#e8e0f0"),3);
    g.f(C(12,12,3.4),T,"ball"); g.px(11,10.5,"#ffffff"); g.f(C(19.5,20,1.8),T,"ball");
  },
  whaling(g,T){
    g.f(P([[11,17],[10.5,11],[7,9.5],[2,9.5],[3.5,6],[9,6.5],[12,9],[15,6.5],[20.5,6],[22,9.5],[17,9.5],[13.5,11],[13,17]]),T,"bevel");
    g.px(12,9,T[0]); g.dots([[2,11],[22,11],[3,13],[21,13]],WHITE[3]);
    g.f(U(R(1,17,22,5),C(5,17,2),C(12,16.5,2.4),C(19,17,2)),BLUE,"bevel");
    g.dots([[3,16],[4,15],[10,15],[11,14],[14,15],[18,15],[20,15.5]],"#ffffff");
  },
  amber(g,T){
    g.f(P([[5,11],[8,5],[15,4],[20,9],[19.5,17],[13,21],[6,18.5]]),T,"ball");
    g.f(R(11,11,2,3),WOOD,0); g.dots([[10,11],[13,11],[10,13],[13,13],[11,10],[12,10]],WOOD[1]); g.px(12,15,WOOD[1]);
    g.dots([[8,7],[9,7],[8,8]],T[4]);
  },
  dyes(g,T){ bowlOf(g,12,7.5,6,T); bowlOf(g,6,16,5.4,RED); bowlOf(g,18,16,5.4,ramp("#e8b830")); },
  tyrian_purple(g,T){
    g.f(P([[7.5,15],[2,21.5],[6.5,16.5]]),T,"bevel");
    g.f(P([[14,9],[20.5,3.5],[18,10.5]]),T,"bevel");
    g.f(E(11,13,6.4,5.2),T,"ball");
    for(const [a,b,c,d] of [[8,8.5,6,5.5],[12,8,12,4.5],[15.5,10,18,8],[5,13,2,12],[16.5,15,19.5,16],[10,18,9,21],[13.5,17.5,15,20.5]]) g.line(a,b,c,d,T[3]);
    g.f(tear(19,19.5,2.3),T,"ball");
  },
  coral(g,T){
    for(const [a,b,c,d] of [[12,21,12,14],[12,14,7,8],[12,14,17,6.5],[7,8,5,3.5],[7,8,10,3],[17,6.5,15,2.5],[17,6.5,20,4],[12,17,18,13],[18,13,21,10],[12,16,5,14],[5,14,3,10]]) g.f(Ln(a,b,c,d,2.1),T,"bevel");
    for(const [x,y] of [[5,3.5],[10,3],[15,2.5],[20,4],[21,10],[3,10]]) g.f(C(x,y,1.4),T,3);
    g.f(E(12,21.5,5,1.4),STONE,"bevel");
  },
  ambergris(g,T){
    g.f(U(E(12,14,9,6.5),C(8,10,4),C(15,9.5,3.8)),T,"ball");
    g.line(6,13,10,15,T[1]); g.line(12,11,17,13,T[1]); g.line(9,17,15,18,T[1]); g.dots([[8,8],[14,8]],T[4]);
  },
  // Fallback for user-added goods: a lidded crate stencilled in the good's tint.
  __crate(g,T){ cube(g,3,9,9,8,WOOD); g.f(P([[3,11],[12,15.5],[12,18],[3,13.5]]),T,2); g.f(P([[12,15.5],[21,11],[21,13.5],[12,18]]),T,1); },
};

// ── build + cache + draw ───────────────────────────────────────────────────
interface Sprite { cv: HTMLCanvasElement; lit: HTMLCanvasElement; lum: number; known: boolean }
const CACHE = new Map<string, Sprite>();

function finish(b:(string|null)[]){
  const out=b.slice(), at=(x:number,y:number)=>x<0||y<0||x>=N||y>=N?null:b[y*N+x];
  for(let y=0;y<N;y++)for(let x=0;x<N;x++){ if(b[y*N+x]) continue;
    const n=at(x,y+1)||at(x+1,y)||at(x,y-1)||at(x-1,y);
    if(n) out[y*N+x]=mix(n,"#0c0610",0.72);
  }
  return out;
}

/** The finished 24×24 sprite for one good, cached per (name, tint). Use `cv`
 *  directly on a canvas, or `cv.toDataURL()` for a DOM `<img>`/CSS background. */
export function goodSprite(name:string,color:string):Sprite{
  const key=name+"|"+color;
  let s=CACHE.get(key); if(s) return s;
  const g=painter(); const T=ramp(color||"#9a8a70");
  (SPR[name]||SPR.__crate)(g,T);
  const px=finish(g.b);
  const cv=document.createElement("canvas"); cv.width=cv.height=N;
  const lit=document.createElement("canvas"); lit.width=lit.height=N;
  const a=cv.getContext("2d")!, l=lit.getContext("2d")!;
  const id=a.createImageData(N,N), il=l.createImageData(N,N);
  px.forEach((c,i)=>{ if(!c) return; const v=hx(c); id.data.set([v[0],v[1],v[2],255],i*4); il.data.set([255,248,232,255],i*4); });
  a.putImageData(id,0,0); l.putImageData(il,0,0);
  s={cv,lit,lum:lumOf(color||"#9a8a70"),known:!!SPR[name]};
  CACHE.set(key,s); return s;
}

/** Draw a good's pixel sprite centred on (cx,cy) in a `size` square.
 *  Integer-scaled with smoothing off whenever the device size allows ≥2×;
 *  below that it draws the exact size with smoothing on. Prefer 24/48/72/96
 *  device-px slots. */
export function drawPixelIcon(ctx:CanvasRenderingContext2D,cx:number,cy:number,size:number,color:string,name:string,opts:{glow?:number}={}){
  const s=goodSprite(name,color);
  const dev=Math.abs(ctx.getTransform?ctx.getTransform().a:1)||1;
  let k=size*dev/N, draw=size, smooth=true;
  if(k>=2){ k=Math.floor(k); draw=k*N/dev; smooth=false; }
  const x=Math.round((cx-draw/2)*dev)/dev, y=Math.round((cy-draw/2)*dev)/dev, p=draw/N;
  ctx.save(); ctx.imageSmoothingEnabled=smooth;
  const glow=opts.glow??1;
  if(glow && s.lum<0.3){
    ctx.globalAlpha=Math.min(0.85,0.5*glow);
    for(const [ox,oy] of [[-1,0],[1,0],[0,-1],[0,1]]) ctx.drawImage(s.lit,x+ox*p,y+oy*p,draw,draw);
    ctx.globalAlpha=1;
  }
  ctx.drawImage(s.cv,x,y,draw,draw);
  ctx.restore();
}

export const PIXEL_GOODS = Object.keys(SPR).filter(k=>k!=="__crate");
export const SPRITE_GRID = N;
