
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
// ---- 3D helpers (round 8): effects drawn along the true view ray ----
// In sig_vol(ro, rd, px) a signature gets the view ray in the blade's frame: ro the camera,
// rd its unit direction, y along the blade from the hilt, x across it to the camera's side,
// z toward the camera. Everything placed with these sits round the blade in depth and keeps
// its shape from every side, from above and in first person.
// The distance from the ray to point p; t returns how far along the ray that is.
float rayPt(vec3 ro, vec3 rd, vec3 p, out float t){
  t = max(dot(p - ro, rd), 0.0);
  return length(ro + rd*t - p);
}
// The distance from the ray to the segment a-b; s returns where on it (0 at a, 1 at b).
float raySeg(vec3 ro, vec3 rd, vec3 a, vec3 b, out float s){
  vec3 ba = b - a, w = ro - a;
  float bd = dot(rd, ba), bb = dot(ba, ba), dw = dot(rd, w), bw = dot(ba, w);
  float den = max(bb - bd*bd, 1e-6);
  s = clamp((bw - bd*dw)/den, 0.0, 1.0);
  vec3 q = a + ba*s;
  float t = max(dot(q - ro, rd), 0.0);
  vec3 pr = ro + rd*t;
  s = clamp(dot(pr - a, ba)/max(bb, 1e-6), 0.0, 1.0);
  return length(pr - (a + ba*s));
}
// Where the ray crosses the plane across the blade at height h: xz there, and t (negative:
// never, or behind the camera).
vec3 rayAt(vec3 ro, vec3 rd, float h){
  float t = abs(rd.y) > 1e-5 ? (h - ro.y)/rd.y : -1.0;
  vec3 p = ro + rd*max(t, 0.0);
  return vec3(p.x, p.z, t);
}
// Where the ray is within R of the blade's axis and between heights y0 and y1: the span of
// t (empty when x >= y).
vec2 raySpan(vec3 ro, vec3 rd, float R, float y0, float y1){
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  vec2 s = vec2(0.0, 1e5);
  if (a < 1e-6){
    if (c > 0.0) return vec2(1.0, 0.0);
  } else {
    float h = b*b - a*c;
    if (h < 0.0) return vec2(1.0, 0.0);
    h = sqrt(h);
    s = vec2((-b - h)/a, (-b + h)/a);
  }
  if (abs(rd.y) > 1e-6){
    float ta = (y0 - ro.y)/rd.y, tb = (y1 - ro.y)/rd.y;
    s = vec2(max(s.x, min(ta, tb)), min(s.y, max(ta, tb)));
  } else if (ro.y < y0 || ro.y > y1){
    return vec2(1.0, 0.0);
  }
  s.x = max(s.x, 0.0);
  return s;
}
// A point round the blade: radius r, angle a (turns), height y.
vec3 around(float r, float a, float y){ return vec3(r*sin(6.2832*a), y, r*cos(6.2832*a)); }
// The blade itself in three dimensions: a white-hot core and a coloured glow round the
// axis from just inside the hilt to the tip. It fills the hilt's mouth as a stock blade
// does, seen from any side, from above and in first person. w is the core's half width.
vec3 blade3(vec3 ro, vec3 rd, float px, vec3 col, float w, float top){
  float s;
  float d = raySeg(ro, rd, vec3(0.0, -0.6, 0.0), vec3(0.0, top, 0.0), s);
  return vec3(1.0)*exp(-pow(d/max(w, px), 2.0))*1.4 + col*halo(d, w*2.2)*0.75 + col*halo(d, w*6.0)*0.18;
}
// A lightning bolt in three dimensions from a to c: 10 kinks, each pushed off the line in
// both directions across it by up to jag of its length.
float bolt3(vec3 ro, vec3 rd, vec3 a, vec3 c, float seed, float jag){
  vec3 d = c - a; float L = max(length(d), 1e-3);
  vec3 n1 = normalize(cross(d, abs(d.y) < 0.9*L ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
  vec3 n2 = cross(d/L, n1);
  float best = 1e3; vec3 prev = a; float s;
  for (int i = 1; i <= 10; i++){
    float f = float(i)/10.0;
    float k = i == 10 ? 0.0 : sin(f*3.1416)*jag*L;
    vec3 q = a + d*f + (n1*(h11(seed*17.13 + float(i)*7.31) - 0.5) + n2*(h11(seed*5.71 + float(i)*3.17) - 0.5))*k;
    best = min(best, raySeg(ro, rd, prev, q, s));
    prev = q;
  }
  return best;
}
// A soft cloud of light: march the ray through a smooth density inside radius R between
// heights y0 and y1 in n steps (the caller sums its own field; this gives the steps).
// Returns the span and the step length in z.
vec3 marchSteps(vec3 ro, vec3 rd, float R, float y0, float y1, float n){
  vec2 s = raySpan(ro, rd, R, y0, y1);
  return vec3(s, max(s.y - s.x, 0.0)/n);
}
// ---- round 9: the big effects (Creyon: "dramatic and beautiful, like beams, strays, rays") ----
// Every one is placed in the blade's own frame (fixed to the hilt), sized in units, and fades
// in and out smoothly; the loops have constant bounds and stop at `count`.
// How near the ray passes the blade's axis between heights y0 and y1 (for early outs).
float fxNear(vec3 ro, vec3 rd, float y0, float y1){
  float s;
  return raySeg(ro, rd, vec3(0.0, y0, 0.0), vec3(0.0, y1, 0.0), s);
}
// The ray's distance to a ring of radius R round the blade at height y: the nearer of where
// it crosses the ring's plane and the ring's cylinder (smooth from any side).
float ringD(vec3 ro, vec3 rd, float y, float R){
  float best = 1e3;
  vec3 h = rayAt(ro, rd, y);
  if (h.z > 0.0) best = abs(length(h.xy) - R);
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  float dsc = b*b - a*c;
  if (dsc > 0.0 && a > 1e-6){
    float q = sqrt(dsc);
    float t0 = (-b - q)/a, t1 = (-b + q)/a;
    if (t0 > 0.0) best = min(best, abs(ro.y + rd.y*t0 - y));
    if (t1 > 0.0) best = min(best, abs(ro.y + rd.y*t1 - y));
  }
  return best;
}
// A star flare at p facing the camera: a white-hot heart of radius size and n thin rays of
// length len, turned by spin (radians).
vec3 fxFlare(vec3 ro, vec3 rd, vec3 p, float px, float size, float len, float n, float spin, vec3 col){
  float t;
  float d = rayPt(ro, rd, p, t);
  if (d > len + size*4.0) return vec3(0.0);
  vec3 q = ro + rd*t - p;
  vec3 u = normalize(cross(rd, abs(rd.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
  vec3 v = cross(rd, u);
  vec2 e = vec2(dot(q, u), dot(q, v));
  float r = length(e);
  float k = 6.2832/max(n, 1.0);
  float aa = mod(atan(e.y, e.x) + spin + k*0.5, k) - k*0.5;
  float across = abs(r*sin(aa)), along = r*cos(aa);
  float w = max(px, 0.03 + 0.012*along);
  float ray = exp(-pow(across/w, 2.0))*exp(-max(along, 0.0)/max(len, 0.01)*2.6)*step(0.0, along);
  return vec3(1.0)*(halo(d, max(size, px))*1.8 + ray*0.9) + col*(halo(d, size*3.0)*0.7 + ray*0.8 + halo(d, size*8.0)*0.15);
}
// Beams of light shooting out of the blade between heights y0 and y1: count of them (up to
// 24), each up to len long, flaring out and fading rate times a second, seed chooses them.
vec3 fxRays(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float len, float rate, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > len*1.6 + 1.0) return acc;
  float s;
  for (int i = 0; i < 24; i++){
    float fi = float(i);
    if (fi >= count) break;
    float cyc = uTime*rate*(0.8 + 0.4*h11(fi*3.3 + seed)) + h11(fi*7.13 + seed);
    float k = floor(cyc), q = fract(cyc);
    float y = mix(y0, y1, h11(k*3.1 + fi*1.7 + seed));
    float a = h11(k*5.7 + fi*2.3 + seed);
    float tilt = (h11(k*2.9 + fi + seed) - 0.5)*0.9;
    vec3 dir = normalize(vec3(sin(6.2832*a), tilt, cos(6.2832*a)));
    float lr = len*(0.55 + 0.6*h11(k*1.3 + fi + seed))*smoothstep(0.0, 0.3, q);
    vec3 a0 = vec3(0.0, y, 0.0) + dir*0.35;
    float d = raySeg(ro, rd, a0, a0 + dir*max(lr, 0.01), s);
    float env = sin(3.1416*q);
    float w = mix(0.3, 0.07, s);
    acc += (vec3(1.0)*halo(d, max(w*0.45, px))*1.4 + col*halo(d, w*3.0)*0.95 + col*halo(d, w*9.0)*0.22)*(1.0 - 0.75*s)*env;
  }
  return acc;
}
// Lightning in three dimensions: count (up to 10) bolts that leave the blade between y0 and
// y1, arc out up to reach units on every side and come back in, with a fork, struck anew
// rate times a second (Sol's Storm, all round the blade).
vec3 fxArcs(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float reach, float rate, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > reach + 2.0) return acc;
  float t;
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    if (fi >= count) break;
    float cyc = uTime*rate*(0.7 + 0.6*h11(fi*1.9 + seed)) + h11(fi*3.3 + seed)*5.0;
    float k = floor(cyc), q = fract(cyc);
    float ya = mix(y0, y1, h11(k*1.9 + fi*2.7 + seed));
    float yb = min(ya + 3.0 + 6.0*h11(k*2.3 + fi + seed), y1 + 2.0);
    float a = h11(k*4.1 + fi*2.0 + seed);
    float R = reach*(0.45 + 0.55*h11(k*7.7 + fi + seed));
    vec3 mid = around(R, a, mix(ya, yb, 0.5) + (h11(k*9.1 + fi) - 0.5)*2.0);
    if (rayPt(ro, rd, mid, t) > R + (yb - ya)*0.6 + 1.5) continue;
    float env = (1.0 - q)*(1.0 - q)*(0.6 + 0.4*step(0.5, fract(q*9.0 + fi)));
    float d = min(bolt3(ro, rd, around(0.25, a, ya), mid, k + fi*1.37 + seed, 0.5),
                  bolt3(ro, rd, mid, around(0.25, a, yb), k + fi*1.37 + seed + 0.5, 0.5));
    vec3 fork = mid + around(R*0.55, a + 0.12*(h11(k + fi) - 0.5), (h11(k*3.3 + fi) - 0.5)*3.0);
    float d2 = bolt3(ro, rd, mid, fork, k*2.1 + fi + seed, 0.6);
    acc += (boltLight(d, col, px) + boltLight(d2, col, px)*0.55)*env;
  }
  return acc;
}
// Sparks spraying off the blade between y0 and y1: count (up to 32) streaks flying out up
// to len units, white-hot then cooling to col, rate cycles a second.
vec3 fxSparks(vec3 ro, vec3 rd, float px, float y0, float y1, float count, float len, float rate, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > len + 1.5) return acc;
  float s;
  for (int i = 0; i < 32; i++){
    float fi = float(i);
    if (fi >= count) break;
    float cyc = uTime*rate*(0.7 + 0.6*h11(fi*2.1 + seed)) + h11(fi*5.9 + seed);
    float k = floor(cyc), q = fract(cyc);
    float y = mix(y0, y1, h11(k*2.7 + fi*1.3 + seed));
    float a = h11(k*6.3 + fi*0.7 + seed);
    vec3 dir = normalize(vec3(sin(6.2832*a), 0.2 + 0.9*(h11(k*1.1 + fi + seed) - 0.3), cos(6.2832*a)));
    float v = len*(0.5 + 0.5*h11(k*4.4 + fi + seed));
    vec3 o = around(0.3, a, y);
    float e1 = 1.0 - exp(-q*3.0), e0 = 1.0 - exp(-max(q - 0.07, 0.0)*3.0);
    float d = raySeg(ro, rd, o + dir*v*e0, o + dir*v*e1, s);
    float life = 1.0 - smoothstep(0.55, 1.0, q);
    acc += mix(vec3(1.0, 0.97, 0.9), col, smoothstep(0.0, 0.6, q))*(halo(d, max(0.05, px))*2.0 + halo(d, 0.28)*0.3)*life;
  }
  return acc;
}
// Coils of light wound round the blade from y0 to y1 at radius R, turns per 10 units,
// travelling up at speed; knots of light run along them. count (up to 4) strands.
vec3 fxCoils(vec3 ro, vec3 rd, float px, float y0, float y1, float R, float turns, float speed, float count, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > R + 1.5) return acc;
  float s;
  for (int j = 0; j < 4; j++){
    float fj = float(j);
    if (fj >= count) break;
    float best = 1e3, bf = 0.0;
    vec3 prev = vec3(0.0);
    for (int i = 0; i <= 24; i++){
      float f = float(i)/24.0;
      float y = mix(y0, y1, f);
      vec3 p = around(R*(0.85 + 0.15*sin(f*12.0 + uTime*2.0 + fj)), y*turns/10.0 - uTime*speed + fj/max(count, 1.0), y);
      if (i > 0){
        float d = raySeg(ro, rd, prev, p, s);
        if (d < best){ best = d; bf = (float(i) - 1.0 + s)/24.0; }
      }
      prev = p;
    }
    float knot = pow(0.5 + 0.5*sin(bf*40.0 - uTime*9.0 + fj*2.0), 6.0);
    float ends = smoothstep(0.0, 0.08, bf)*(1.0 - smoothstep(0.9, 1.0, bf));
    acc += (vec3(1.0)*halo(best, max(0.05, px))*(0.5 + 1.5*knot) + col*halo(best, 0.3)*(0.5 + knot) + col*halo(best, 1.0)*0.12)*ends;
  }
  return acc;
}
// Motes (dust, embers, sparks of light) drifting up round the blade between radii r0 and r1:
// count (up to 40), each a soft point that twinkles.
vec3 fxMotes(vec3 ro, vec3 rd, float px, float y0, float y1, float r0, float r1, float count, float speed, float seed, vec3 col){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0, y1) > r1 + 1.0) return acc;
  float t;
  for (int i = 0; i < 40; i++){
    float fi = float(i);
    if (fi >= count) break;
    float f = fract(uTime*speed*(0.6 + 0.8*h11(fi*2.9 + seed)) + h11(fi*5.1 + seed));
    float y = mix(y0 - 1.0, y1 + 2.0, f);
    vec3 p = around(mix(r0, r1, h11(fi*3.7 + seed)), h11(fi*1.9 + seed) + uTime*0.04 + 0.08*sin(uTime*0.7 + fi), y);
    float d = rayPt(ro, rd, p, t);
    float tw = 0.55 + 0.45*sin(uTime*5.0 + fi*2.3);
    acc += col*(halo(d, max(0.07, px))*1.8 + halo(d, 0.35)*0.2)*tw*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.8, 1.0, f));
  }
  return acc;
}
// A shock ring racing out from the blade at height y: q its age (0 to 1), out to R.
vec3 fxShock(vec3 ro, vec3 rd, float px, float y, float q, float R, vec3 col){
  if (q <= 0.0 || q >= 1.0) return vec3(0.0);
  float d = ringD(ro, rd, y, 0.5 + R*(1.0 - pow(1.0 - q, 2.0)));
  float env = (1.0 - q)*smoothstep(0.0, 0.05, q);
  return (vec3(1.0)*halo(d, max(0.07, px))*1.4 + col*halo(d, 0.45)*0.7 + col*halo(d, 1.4)*0.15)*env;
}
// A wide soft aura round the blade from y0 to y1: its light reaching w units out.
vec3 fxAura(vec3 ro, vec3 rd, float y0, float y1, float w, vec3 col){
  float d = fxNear(ro, rd, y0, y1);
  return col*(halo(d, w)*0.35 + halo(d, w*0.4)*0.25);
}
