
// ---- signature helpers ----
float h11(float p){ p=fract(p*0.1031); p*=p+33.33; p*=p+p; return fract(p); }
float h21(vec2 p){ vec3 q=fract(vec3(p.xyx)*0.1031); q+=dot(q,q.yzx+33.33); return fract((q.x+q.y)*q.z); }
vec2 h22(vec2 p){ vec3 q=fract(vec3(p.xyx)*vec3(0.1031,0.1030,0.0973)); q+=dot(q,q.yzx+33.33); return fract((q.xx+q.yz)*q.zy); }
float vn(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.0-2.0*f);
  return mix(mix(h21(i),h21(i+vec2(1.0,0.0)),f.x),mix(h21(i+vec2(0.0,1.0)),h21(i+vec2(1.0)),f.x),f.y); }
float fbm(vec2 p){ float s=0.0, a=0.5; for(int i=0;i<5;i++){ s+=a*vn(p); p=p*2.03+vec2(1.7,9.2); a*=0.5; } return s; }
float fbm3(vec2 p){ float s=0.0, a=0.5; for(int i=0;i<3;i++){ s+=a*vn(p); p=p*2.03+vec2(1.7,9.2); a*=0.5; } return s/0.875; }
vec3 hsv(float h, float s, float v){ vec3 p=abs(fract(vec3(h)+vec3(0.0,2.0/3.0,1.0/3.0))*6.0-3.0); return v*mix(vec3(1.0),clamp(p-1.0,0.0,1.0),s); }
mat2 rot(float a){ float c=cos(a), s=sin(a); return mat2(c,-s,s,c); }
float sdSeg(vec2 p, vec2 a, vec2 b){ vec2 pa=p-a, ba=b-a; float h=clamp(dot(pa,ba)/dot(ba,ba),0.0,1.0); return length(pa-ba*h); }
float sdBox(vec2 p, vec2 b){ vec2 d=abs(p)-b; return length(max(d,0.0))+min(max(d.x,d.y),0.0); }
float sdEll(vec2 p, vec2 r){ float k=length(p/r); return (k-1.0)*min(r.x,r.y); }
float fillD(float d, float px){ return clamp(0.5-d/max(px,1e-4),0.0,1.0); }
float halo(float d, float w){ float e=max(d,0.0)/w; return exp(-e*e); }
float voro(vec2 p, out vec2 cell){ vec2 i=floor(p), f=fract(p); float d1=8.0, d2=8.0; cell=vec2(0.0);
  for(int y=-1;y<=1;y++) for(int x=-1;x<=1;x++){ vec2 g=vec2(float(x),float(y)); vec2 o=h22(i+g); float d=length(g+o-f);
    if(d<d1){ d2=d1; d1=d; cell=i+g; } else if(d<d2) d2=d; }
  return d2-d1; }
float sigBody(vec2 b, float r){ float y=b.y, L=uLength; float beyond=max(y-L,0.0); return length(vec2(b.x, beyond))/r*step(-1.0,y); }
// ---- SJK effect helpers (shared by the signatures; Creyon's round 7) ----
// Distance from p to a jagged lightning bolt from a to c: 10 kinks, displaced across the
// line by up to jag of its length, the kinks chosen by seed (strike anew by changing it).
float boltD(vec2 p, vec2 a, vec2 c, float seed, float jag){
  vec2 d = c - a; float L = max(length(d), 1e-3); vec2 n = vec2(-d.y, d.x)/L;
  float best = 1e3; vec2 prev = a;
  for (int i = 1; i <= 10; i++){
    float f = float(i)/10.0;
    float off = i == 10 ? 0.0 : (h11(seed*17.13 + float(i)*7.31) - 0.5)*jag*L*sin(f*3.1416);
    vec2 q = a + d*f + n*off;
    best = min(best, sdSeg(p, prev, q));
    prev = q;
  }
  return best;
}
// A bolt's light: a white-hot thread in a coloured glow, never thinner than a pixel.
vec3 boltLight(float dist, vec3 col, float px){
  return vec3(1.0)*halo(dist, max(px, 0.035))*1.6 + col*halo(dist, 0.22)*0.9 + col*halo(dist, 0.9)*0.18;
}
// A Storm-like loop: a bolt that leaves the blade at height y0, arcs out to reach units on
// side s and comes back in at y1, struck anew rate times a second.
vec3 arcLoop(vec2 b, float y0, float y1, float s, float reach, float seed, float rate, vec3 col, float px){
  float k = floor(uTime*rate + seed*13.0);
  vec2 a = vec2(0.0, y0), c = vec2(0.0, y1);
  vec2 m = vec2(s*reach*(0.75 + 0.25*h11(k + seed)), mix(y0, y1, 0.5) + (h11(k*1.7 + seed) - 0.5)*2.0);
  float d = min(boltD(b, a, m, k + seed, 0.45), boltD(b, m, c, k + seed + 0.5, 0.45));
  return boltLight(d, col, px);
}
// A helix strand wound round the blade: across (x) and how near the viewer it is (z, -1..1)
// at height y, turns per 10 units, phase turns, radius r.
vec2 helix(float y, float r, float turns, float phase){
  float a = 6.2832*(y*turns/10.0 + phase);
  return vec2(r*sin(a), cos(a));
}
