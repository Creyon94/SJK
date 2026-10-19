// Exogorth: the blade is the space slug's living gullet: a real, round, ribbed and rubbery throat
// with a molten amber core inside. Slow swallowing waves bulge up it and the light glows hot through
// the stretched hide between its rings. At the emitter it opens as a maw: a fleshy lip ringed with
// two rows of pale fangs that the blade pushes out of, glowing strands of saliva sagging between the
// fangs and the throat. All round it hangs the dark of an asteroid cave: dust and grit swirl in a
// slow 3D whirlpool and are sucked into the throat, lumps of rock tumble in and are crushed in
// sparks. A swing makes the throat gulp; a clash is the slug's roar: a shockwave rolls out, a burst
// of hot breath and a spray of rocks blow outward and the fangs flare open; on ignition the gullet
// swells up out of the maw.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.55,0.20,0.04],"corona.rim_hot":[1.0,0.55,0.18],"corona.inner":[1.0,0.88,0.65],"core.white":[1.0,0.95,0.85],"core.fringe_cool":[1.0,0.45,0.12],"core.fringe_hot":[1.0,0.75,0.40],"light.color":[1.0,0.55,0.20],"trail":[1.0,0.55,0.20]}

float ex_top(){ float g = clamp(uAge/0.5, 0.0, 1.0); return (uLength + 0.2)*(1.0 - (1.0 - g)*(1.0 - g)); }
// the swallowing waves: how much the throat bulges at y
float ex_bulge(float y){
  float t = uTime;
  float w = pow(0.5 + 0.5*sin(y*0.32 - t*2.2), 4.0) + 0.6*pow(0.5 + 0.5*sin(y*0.21 - t*1.4 + 2.0), 6.0);
  return w*(1.0 + 1.2*uSwing);
}
// the rings of the throat: 1 on a ridge, 0 between
float ex_ring(float y){ return 0.5 + 0.5*cos(y*3.2); }
float ex_w(float y){
  float L = uLength, top = ex_top();
  float w = (0.46 + 0.2*ex_bulge(y))*(1.0 + 0.09*ex_ring(y));
  w *= 1.0 - 0.75*smoothstep(top - 1.4, top + 0.1, y);
  return w;
}
// how far the maw is open: breathing slowly, flaring on a clash
float ex_gape(){ return 0.5 + 0.3*sin(uTime*0.7) + 0.9*uClash + 0.3*uSwing; }
// one fang in the (radial, up, sideways) frame of its slot: distance
float ex_fang(vec3 q, float r0, float len, float lean){
  vec3 a = vec3(r0, 0.25, 0.0);
  vec3 b = vec3(r0 + lean, 0.25 + len, 0.0);
  // a curved fang: the middle bows inward toward the blade
  vec3 pa = q - a, ba = b - a;
  float h = clamp(dot(pa, ba)/dot(ba, ba), 0.0, 1.0);
  vec3 c = a + ba*h - vec3(0.25*sin(h*2.6)*len, 0.0, 0.0);
  return length(q - c) - mix(0.27, 0.02, pow(h, 0.6));
}
// the maw: a fleshy lip on the hilt and two rows of fangs. m returns 1 for fangs, 0 for flesh
float ex_maw(vec3 p, out float m){
  float r = length(p.xz);
  float an = atan(p.z, p.x);
  float g = ex_gape();
  // the lip: a fat ring of flesh round the emitter
  vec2 tq = vec2(r - 0.8, p.y - 0.2);
  float lip = length(tq*vec2(0.95, 1.4)) - 0.26 - 0.03*sin(atan(p.z, p.x)*7.0);
  // outer row: 9 long fangs leaning outward with the gape
  float N = 11.0;
  float sec = 6.2832/N;
  float ia = floor(an/sec + 0.5);
  float da = an - ia*sec;
  vec3 q = vec3(r*cos(da), p.y, r*sin(da));
  float j = h11(ia + 3.0);
  float f1 = ex_fang(q, 0.86, 1.15 + 0.45*j, 0.1 + 0.4*g);
  // inner row: 9 shorter fangs between them, leaning in over the blade
  float ib = floor(an/sec);
  float db = an - (ib + 0.5)*sec;
  vec3 q2 = vec3(r*cos(db), p.y, r*sin(db));
  float f2 = ex_fang(q2, 0.7, 0.75 + 0.3*h11(ib + 7.0), -0.1 + 0.25*g);
  float f = min(f1, f2);
  m = step(f, lip + 0.02);
  // fangs grow out of the lip
  float k = 0.12;
  float hh = clamp(0.5 + 0.5*(f - lip)/k, 0.0, 1.0);
  return mix(f, lip, hh) - k*hh*(1.0 - hh);
}
float ex_sdf(vec3 p, out float m){
  float top = ex_top();
  float thr = (length(p.xz) - ex_w(p.y))*0.75;
  thr = max(thr, max(0.1 - p.y, p.y - top));
  float mw = ex_maw(p, m);
  float d = min(thr, mw);
  m = mw < thr ? m : 0.0;
  if (thr <= mw) m = -1.0;          // -1: the throat
  return d;
}
float ex_sdf0(vec3 p){ float m; return ex_sdf(p, m); }
// a lump of rock: how much it covers the ray (th its depth, sh its face shade)
float ex_rock(vec3 ro, vec3 rd, vec3 c, float sz, float spin, float seed, float px, out float th, out float sh){
  float d = rayPt(ro, rd, c, th);
  sh = 0.0;
  if (d > sz*1.6 + px) return 0.0;
  vec3 o = ro + rd*th - c;
  vec2 a = rot(spin + seed*4.0)*o.xy; o = vec3(a, o.z);
  vec2 b = rot(spin*0.6 + seed*2.0)*o.xz; o = vec3(b.x, o.y, b.y);
  float k = max(max(abs(o.x) + abs(o.y)*0.5, abs(o.y)*1.1 + abs(o.z)*0.4), abs(o.z) + abs(o.x)*0.4) - sz + 0.25*sz*vn(o.xy*5.0/sz + seed);
  sh = 0.45 + 0.55*h21(sign(o.xy)*1.3 + vec2(seed, sign(o.z)));
  return clamp(0.5 - k/max(px, 1e-3), 0.0, 1.0);
}
// rock i being dragged in round the blade
vec3 ex_rockPos(float fi, out float f, out float sz){
  float L = uLength, t = uTime;
  float rate = 0.10 + 0.05*h11(fi*2.3);
  f = fract(t*rate + h11(fi*6.1));
  float cyc = floor(t*rate + h11(fi*6.1));
  float a0 = h11(fi*3.7 + cyc*1.9);
  float y = (0.12 + 0.75*h11(fi*9.3 + cyc))*L;
  float r = 0.6 + 5.0*(1.0 - f)*(1.0 - f);
  sz = (0.28 + 0.3*h11(fi*5.0 + cyc))*(1.0 - smoothstep(0.85, 1.0, f));
  return around(r, a0 + 0.35*f, y + 2.0*(1.0 - f));
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  vec4 col = vec4(0.0);
  float tHit = 1e5;
  float top = ex_top();
  // ---- the gullet and the maw ----
  vec2 sp = raySpan(ro, rd, 2.0, -0.1, top + 0.3);
  if (sp.y > sp.x){
    float tt = sp.x, dmin = 1e3, tmin = sp.x, hit = 0.0, m;
    for (int i = 0; i < 48; i++){
      float d = ex_sdf(ro + rd*tt, m);
      if (d < dmin){ dmin = d; tmin = tt; }
      if (d < 0.002){ hit = 1.0; break; }
      tt += max(d, 0.004);
      if (tt > sp.y) break;
    }
    float cov = hit > 0.5 ? 1.0 : clamp(1.0 - dmin/max(px, 1e-3), 0.0, 1.0);
    if (cov > 0.0){
      float th = hit > 0.5 ? tt : tmin;
      vec3 p = ro + rd*th;
      float mat;
      ex_sdf(p, mat);
      vec2 e = vec2(0.015, 0.0);
      vec3 n = normalize(vec3(ex_sdf0(p + e.xyy) - ex_sdf0(p - e.xyy), ex_sdf0(p + e.yxy) - ex_sdf0(p - e.yxy), ex_sdf0(p + e.yyx) - ex_sdf0(p - e.yyx)));
      vec3 v = -rd;
      vec3 kl = normalize(vec3(-0.4, 0.75, 0.55));
      float ndv = max(dot(n, v), 0.0);
      float dif = max(dot(n, kl), 0.0);
      float spec = pow(max(dot(reflect(-kl, n), v), 0.0), 30.0);
      // the core's light falling on everything from inside the throat
      float rr = length(p.xz);
      vec3 rgb;
      if (mat < -0.5){
        // the throat: dark rubbery rings, the light glowing through where it is stretched thin
        float ring = ex_ring(p.y);
        float bu = ex_bulge(p.y);
        float thin = clamp(bu*0.8 + 0.15, 0.0, 1.0)*(1.0 - ring*0.7);
        float nz = fbm3(vec2(atan(p.z, p.x)*1.5 + p.y*0.3, p.y*1.2));
        vec3 hide = vec3(0.24, 0.21, 0.19)*(0.5 + 0.6*nz)*(0.35 + 0.65*dif)*(0.6 + 0.4*ring);
        vec3 glowT = mix(vec3(1.0, 0.35, 0.08), vec3(1.0, 0.85, 0.5), thin)*thin*(0.35 + 0.75*ndv);
        float vein = halo(abs(vn(vec2(atan(p.z, p.x)*2.5, p.y*0.8)) - 0.5), 0.035);
        rgb = mix(hide, glowT*1.3, smoothstep(0.15, 0.8, thin)) + vec3(1.0, 0.4, 0.1)*vein*thin*0.7;
        // the hot glow between the rings, always faintly lit from inside
        rgb += vec3(1.0, 0.45, 0.12)*(1.0 - ring)*0.25*(0.5 + bu)*ndv;
        rgb += vec3(0.9, 0.8, 0.7)*spec*0.35*(0.4 + 0.6*ring);
      } else if (mat > 0.5){
        // fangs: pale and wet, lit amber from the blade below
        float up = clamp((p.y - 0.25)/1.3, 0.0, 1.0);
        rgb = vec3(0.80, 0.74, 0.62)*(0.3 + 0.6*dif)*(0.75 + 0.25*vn(p.xy*14.0)) + vec3(1.0, 0.5, 0.15)*(1.0 - up)*0.35;
        rgb += vec3(1.0, 0.95, 0.85)*spec*0.6 + vec3(1.0, 0.55, 0.2)*pow(1.0 - ndv, 3.0)*0.4;
        rgb *= mix(1.0, 0.75, up*0.5);
      } else {
        // the lip: dark gum-grey flesh, lit hot on its inner side
        float inner = clamp((0.85 - rr)/0.3, 0.0, 1.0);
        rgb = vec3(0.22, 0.17, 0.15)*(0.4 + 0.6*dif)*(0.7 + 0.3*vn(p.xz*8.0)) + vec3(1.0, 0.45, 0.12)*inner*0.8 + vec3(0.9, 0.7, 0.6)*spec*0.4;
      }
      col = vec4(rgb, cov);
      tHit = th;
    }
  }
  if (uFx < 0.01) return col;
  // ---- rocks dragged in and crushed ----
  vec4 rk = vec4(0.0); float bestT = tHit;
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float f, sz;
    vec3 c = ex_rockPos(fi, f, sz);
    if (c.y > top) continue;
    float th, sh;
    float cv = ex_rock(ro, rd, c, sz, t*(0.6 + h11(fi)), fi, px, th, sh);
    if (cv > 0.0 && th < bestT){
      bestT = th;
      vec3 rgb = vec3(0.22, 0.19, 0.17)*sh + vec3(1.0, 0.5, 0.15)*(0.15 + 0.6*smoothstep(0.4, 1.0, f))*(1.0 - sh);
      rk = vec4(rgb, cv);
    }
  }
  // ---- the roar: a spray of rocks blown out from where it was struck ----
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    for (int i = 0; i < 8; i++){
      float fi = float(i);
      vec3 dir = normalize(vec3(sin(fi*2.39), 0.5*(h11(fi*3.0) - 0.4), cos(fi*2.39)));
      vec3 c = vec3(0.0, L*0.6, 0.0) + dir*(0.9 + 6.0*a) + vec3(0.0, -2.0*a*a, 0.0);
      float th, sh;
      float cv = ex_rock(ro, rd, c, (0.25 + 0.15*h11(fi*7.0))*(1.0 - smoothstep(0.6, 1.0, a)), a*8.0, fi + 20.0, px, th, sh);
      if (cv > 0.0 && th < bestT){
        bestT = th;
        rk = vec4(vec3(0.25, 0.22, 0.2)*sh + vec3(1.0, 0.55, 0.2)*(1.0 - sh)*uClash, cv);
      }
    }
  }
  col = vec4(mix(col.rgb, rk.rgb, rk.a), max(col.a, rk.a));
  return col;
}
// the cave's dust: density at p, swirling round and drawn in
float ex_dust(vec3 p){
  float t = uTime, L = uLength;
  float r = length(p.xz);
  // swirl: the noise is turned round the blade faster near it and pulled inward over time
  vec2 sw = rot(t*0.5 + 2.0/(r + 0.8))*p.xz;
  float n = fbm3(vec2(sw.x*0.45 + r*0.6 + t*0.5, p.y*0.25))*fbm3(vec2(sw.y*0.45 + r*0.6 + t*0.5, p.y*0.22 + 7.0));
  float d = smoothstep(0.25, 0.55, n)*smoothstep(0.6, 1.4, r)*(1.0 - smoothstep(3.6, 5.0, r));
  // the roar: a burst of hot breath round where it was struck
  float br = uClash > 0.01 ? exp(-pow((length(p - vec3(0.0, L*0.6, 0.0)) - 1.0 - 4.0*(1.0 - uClash))/1.2, 2.0))*uClash*2.5 : 0.0;
  return (d*0.8 + br*n)*smoothstep(-0.5, 1.0, p.y)*smoothstep(ex_top() + 2.0, ex_top() - 1.0, p.y);
}
// a strand of saliva i: point at u (0 on a fang .. 1 on the throat), sagging and swaying
vec3 ex_strand(float fi, float u){
  float t = uTime;
  float a = fi/6.0 + 0.04*sin(t*0.9 + fi);
  float g = ex_gape();
  vec3 p0 = around(0.9 + 0.3*g, a, 1.1 + 0.3*h11(fi*3.0));
  vec3 p1 = around(0.5, a + 0.08, 2.2 + 1.8*h11(fi*5.0));
  vec3 p = mix(p0, p1, u);
  p.y -= sin(u*3.1416)*(0.5 + 0.25*sin(t*1.3 + fi*2.0));
  p += around(0.08*sin(u*3.1416)*sin(t*2.0 + fi), a + 0.25, 0.0);
  return p;
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float top = ex_top();
  vec3 amb = vec3(1.0, 0.55, 0.2);
  // the molten core, filling the maw and the hilt's mouth
  vec3 light = blade3(ro, rd, px, amb, 0.34, max(top - 0.4, 0.0));
  float s;
  float d = raySeg(ro, rd, vec3(0.0, 0.0, 0.0), vec3(0.0, top, 0.0), s);
  float ys = s*top;
  float bu = ex_bulge(ys);
  light += amb*halo(max(d - ex_w(ys), 0.0), 0.5 + 0.4*bu)*(0.45 + 0.6*bu) + vec3(0.8, 0.35, 0.1)*halo(d, 2.8)*0.08;
  // the glow in the maw
  float tm;
  light += vec3(1.0, 0.6, 0.25)*halo(rayPt(ro, rd, vec3(0.0, 0.5, 0.0), tm), 0.9)*0.7;
  if (uFx > 0.01){
    vec3 fx = vec3(0.0);
    // the swirling dust, lit amber by the blade, a cold grey further out
    vec3 m = marchSteps(ro, rd, 5.0, -0.5, L + 2.0, 16.0);
    if (m.y > m.x){
      float tt = m.x + m.z*h21(rd.xy*517.0 + ro.xz);
      for (int i = 0; i < 16; i++){
        vec3 p = ro + rd*tt;
        float dn = ex_dust(p);
        if (dn > 0.001){
          float r = length(p.xz);
          fx += mix(vec3(0.30, 0.25, 0.20), vec3(1.0, 0.55, 0.25), exp(-r/1.8))*dn*(0.08 + 0.3*exp(-r/1.5))*m.z;
        }
        tt += m.z;
      }
    }
    // grit and specks spiralling in, crushed into sparks at the throat
    for (int i = 0; i < 20; i++){
      float fi = float(i);
      float rate = 0.15 + 0.1*h11(fi*3.1);
      float f = fract(t*rate + h11(fi*5.7));
      float cyc = floor(t*rate + h11(fi*5.7));
      float y = (0.08 + 0.85*h11(fi*2.9 + cyc))*L;
      float R = 0.6 + 5.0*(1.0 - f)*(1.0 - f);
      float a = h11(fi*1.3 + cyc*2.7) + 0.6*f;
      vec3 p = around(R, a, y + 1.5*(1.0 - f)*sin(fi));
      vec3 q = around(R + 0.25*(1.0 - f) + 0.05, a - 0.015, y + 1.5*(1.0 - f)*sin(fi));
      float dd = raySeg(ro, rd, p, q, s);
      fx += vec3(1.0, 0.75, 0.45)*halo(dd, max(0.04, px))*(0.4 + 0.8*h11(fi*7.7))*smoothstep(0.0, 0.3, f)*s*step(y, top);
      float tq;
      fx += vec3(1.0, 0.7, 0.3)*halo(rayPt(ro, rd, around(ex_w(y) + 0.1, a, y), tq), 0.3)*smoothstep(0.9, 1.0, f)*1.2*step(y, top);
    }
    // glowing saliva strands sagging between the fangs and the throat, a drop gathering on each
    if (top > 3.0){
      for (int i = 0; i < 6; i++){
        float fi = float(i);
        vec3 prev = ex_strand(fi, 0.0);
        float best = 1e3;
        for (int k = 1; k <= 4; k++){
          vec3 nx = ex_strand(fi, float(k)/4.0);
          best = min(best, raySeg(ro, rd, prev, nx, s));
          prev = nx;
        }
        float tq;
        float drop = rayPt(ro, rd, ex_strand(fi, 0.5) - vec3(0.0, 0.12, 0.0), tq);
        fx += vec3(1.0, 0.7, 0.35)*halo(best, max(0.03, px*0.7))*0.8 + amb*halo(best, 0.2)*0.45 + vec3(1.0, 0.85, 0.6)*halo(drop, max(0.07, px))*0.9;
      }
    }
    // the roar: a shockwave rolling out round the blade and a blast of light
    if (uClash > 0.01){
      float a = 1.0 - uClash;
      vec3 h = rayAt(ro, rd, L*0.6);
      if (h.z > 0.0) fx += amb*halo(abs(length(h.xy) - 1.0 - 7.0*a), 0.22 + 0.3*a)*1.6*uClash;
      float tq;
      float dc = rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), tq);
      fx += amb*halo(abs(dc - 1.0 - 5.0*a), 0.3 + 0.4*a)*0.7*uClash + amb*halo(dc, 1.5)*pow(uClash, 2.0)*1.2;
      // a pulse of light up the whole gullet
      fx += amb*halo(max(d - ex_w(ys), 0.0), 1.2)*uClash*0.6;
    }
    light += fx*uFx;
  }
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
