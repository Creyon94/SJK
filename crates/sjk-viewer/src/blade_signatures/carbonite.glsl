// Carbonite: a blade that keeps being frozen in carbonite and breaking free. A wave of freezing
// steam climbs from the hilt and seals the blade inside a real block of dark bronze carbonite:
// a slightly lumpy, rounded square slab with cast relief, grooves across it and a cold sheen,
// cold vapour billowing off it on every side. The light trapped inside beats through hairline
// cracks like a heartbeat; the cracks spread and brighten until the slab bursts into shards that
// fly outward all round and the blade blazes free in a cloud of cold mist, before it freezes
// over again from the hilt up. A swing makes the cracks flare; a clash chips shards off the slab
// in a ring of light and steam. The emitter always glows in the gap below the slab.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.20,0.45,0.75],"corona.rim_hot":[0.60,0.88,1.0],"corona.inner":[0.92,0.98,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.50,0.80,1.0],"core.fringe_hot":[0.80,0.95,1.0],"light.color":[0.60,0.88,1.0],"trail":[0.60,0.88,1.0]}

const float CB_W = 0.78;      // the slab's half width
const float CB_RR = 0.24;     // how rounded its corners are
const float CB_Y0 = 0.3;      // the slab's foot, above the glowing emitter
const float CB_P = 12.0;      // seconds per freeze and break
float cb_ph(){ return mod(uTime - 0.5, CB_P); }
float cb_btop(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
// the freezing front climbing from the hilt
float cb_front(){ return -0.5 + (uLength + 2.5)*smoothstep(0.0, 1.6, cb_ph()); }
// 1 while the slab stands
float cb_held(){ return step(cb_ph(), 8.0); }
// the slab's top
float cb_stop(){ return min(min(cb_front(), uLength + 0.5), cb_btop() + 0.3); }
// how far the cracks have spread
float cb_crack(){ float ph = cb_ph(); return clamp(0.15 + smoothstep(5.0, 7.9, ph)*0.85 + 0.6*uSwing + uClash, 0.0, 1.3); }
// the burst: 0 at the break .. 1 when the shards are gone
float cb_burst(){ return clamp((cb_ph() - 8.0)/1.5, 0.0, 1.0); }
float cb_beat(){ return 0.55 + 0.45*pow(0.5 + 0.5*sin(uTime*2.6), 10.0) + 0.35*pow(0.5 + 0.5*sin(uTime*2.6 - 0.6), 14.0); }
// the slab's shape: a rounded, slightly lumpy square prism turned a little round the blade
vec2 cb_q(vec3 p){
  float sh = smoothstep(7.4, 8.0, cb_ph())*0.05*sin(uTime*70.0);   // it shudders before it bursts
  return rot(0.5)*p.xz + vec2(sh, 0.0);
}
float cb_sdf(vec3 p){
  float L = uLength;
  float w = CB_W + 0.06*(vn(vec2(p.y*0.7, 3.0)) - 0.5);
  w *= 1.0 - 0.35*smoothstep(L - 1.5, L + 0.5, p.y);
  vec2 q = cb_q(p);
  float d2 = sdBox(q, vec2(w - CB_RR)) - CB_RR;
  float top = cb_stop();
  vec2 e = vec2(d2, max(p.y - top, CB_Y0 - p.y) + 0.25);
  return min(max(e.x, e.y), 0.0) + length(max(e, vec2(0.0))) - 0.25;
}
// cold steam: density of the vapour at p (two noise slices crossed, so it is round in 3D)
float cb_dens(vec3 p){
  float t = uTime, L = uLength;
  float r = length(p.xz);
  float ph = cb_ph();
  float held = cb_held();
  float n = fbm3(vec2(p.x*0.55 + 1.3, p.y*0.32 - t*1.1 - r*0.25))*fbm3(vec2(p.z*0.55 - 2.1, p.y*0.29 - t*0.9 - r*0.3 + 4.0));
  n = smoothstep(0.12, 0.55, n);
  // a sleeve of vapour pouring off the slab, a soft mist round the free blade
  float shell = exp(-max(r - 0.9, 0.0)/1.3)*smoothstep(0.5, 1.0, r);
  float base = held*0.55*smoothstep(-0.5, 0.0, cb_front() - p.y) + (1.0 - held)*0.22;
  // the freezing wave: a thick billow at the climbing front
  float wave = exp(-pow((p.y - cb_front())/1.8, 2.0))*smoothstep(2.6, 1.6, ph)*2.2*exp(-max(r - 1.0, 0.0)/2.2);
  // the burst: a shell of mist blown outward
  float bu = cb_burst();
  float burst = (bu > 0.0 && bu < 1.0) ? exp(-pow((r - 0.8 - 2.6*sqrt(bu))/1.1, 2.0))*(1.0 - bu)*1.5 : 0.0;
  float cl = uClash > 0.01 ? exp(-length(p - vec3(0.0, L*0.6, 0.0))/2.0)*uClash*2.0 : 0.0;
  float d = n*(base*shell + wave + burst + cl);
  return d*(1.0 - smoothstep(2.8, 4.8, r))*smoothstep(cb_btop() + 2.5, cb_btop(), p.y)*smoothstep(-0.8, 0.5, p.y)*(1.0 - smoothstep(L + 0.5, L + 3.5, p.y));
}
// march the steam from the start of its span up to tmax
vec3 cb_steam(vec3 ro, vec3 rd, float tmax){
  vec3 m = marchSteps(ro, rd, 5.0, -0.8, uLength + 3.5, 18.0);
  if (m.y <= m.x) return vec3(0.0);
  float te = min(m.y, tmax);
  if (te <= m.x) return vec3(0.0);
  float dt = (te - m.x)/18.0;
  float t = m.x + dt*h21(rd.xy*517.0 + ro.xz);
  float acc = 0.0, lit = 0.0;
  float beat = cb_beat();
  float free = 1.0 - cb_held();
  for (int i = 0; i < 18; i++){
    vec3 p = ro + rd*t;
    float d = cb_dens(p);
    if (d > 0.001){
      float r = length(p.xz);
      // lit by the blade: brighter near it, beating with the trapped heart while frozen
      float g = 0.35 + 1.1*exp(-r/1.4)*mix(0.5 + 0.6*beat*cb_crack(), 1.6, free);
      float tr = exp(-acc*1.2);
      lit += d*g*tr*dt;
      acc += d*dt;
    }
    t += dt;
  }
  return vec3(0.72, 0.86, 0.95)*lit*0.75;
}
// the light beating through the hairline cracks on a face: face coords (u, y)
float cb_cracks(vec2 f, float px){
  vec2 cell;
  float e = voro(vec2(f.x*2.4, f.y*0.75), cell);
  float reach = cb_crack();
  float keep = step(h21(cell + 3.1), reach);
  return halo(e, max(0.045 + 0.05*reach, px*1.2))*keep;
}
// a flying shard of slab: how much it covers the ray; th its depth, sh.x its facet shade,
// sh.y the glowing broken edge
float cb_shard(vec3 ro, vec3 rd, vec3 c, float sz, float spin, float seed, float px, out float th, out vec2 sh){
  float d = rayPt(ro, rd, c, th);
  sh = vec2(0.0);
  if (d > sz*2.0 + px) return 0.0;
  vec3 o = ro + rd*th - c;
  vec2 a = rot(spin + seed*6.0)*o.xy; o = vec3(a, o.z);
  vec2 b = rot(spin*0.7 + seed*3.0)*o.yz; o = vec3(o.x, b);
  // an irregular broken triangle, thick as a slab plate
  float j = h11(seed*3.7);
  float tri = max(max(dot(o.xy, vec2(0.0, 1.0))*(0.8 + 0.5*j), dot(o.xy, vec2(0.87, -0.5))), dot(o.xy, vec2(-0.87 + 0.3*j, -0.5)));
  float k = max(tri*1.25, abs(o.z)*2.2 + abs(o.x)*0.15) - sz*0.75;
  sh = vec2(0.5 + 0.5*h21(sign(o.xy) + vec2(seed*7.0, sign(o.z))), halo(abs(k), 0.035 + px*0.8));
  return clamp(0.5 - k/max(px, 1e-3), 0.0, 1.0);
}
vec4 sig_vol_solid(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  vec4 col = vec4(0.0);
  float tHit = 1e5;
  if (uFx < 0.01) return col;
  // ---- the slab ----
  if (cb_held() > 0.5 && cb_stop() > CB_Y0 + 0.2){
    vec2 sp = raySpan(ro, rd, CB_W*1.5, CB_Y0 - 0.1, cb_stop() + 0.2);
    if (sp.y > sp.x){
      float tt = sp.x, dmin = 1e3, tmin = sp.x;
      bool hit = false;
      for (int i = 0; i < 40; i++){
        float d = cb_sdf(ro + rd*tt);
        if (d < dmin){ dmin = d; tmin = tt; }
        if (d < 0.002){ hit = true; break; }
        tt += d*0.9;
        if (tt > sp.y) break;
      }
      float cov = hit ? 1.0 : clamp(1.0 - dmin/max(px, 1e-3), 0.0, 1.0);
      if (cov > 0.0){
        vec3 p = ro + rd*(hit ? tt : tmin);
        vec2 e = vec2(0.03, 0.0);
        vec3 n = normalize(vec3(cb_sdf(p + e.xyy) - cb_sdf(p - e.xyy), cb_sdf(p + e.yxy) - cb_sdf(p - e.yxy), cb_sdf(p + e.yyx) - cb_sdf(p - e.yyx)));
        vec2 q = cb_q(p);
        // face coordinates: across the face (u) and up it (y); each face its own pattern
        bool fx = abs(q.x) > abs(q.y);
        float u = fx ? q.y : q.x;
        float fid = fx ? (q.x > 0.0 ? 0.0 : 1.0) : (q.y > 0.0 ? 2.0 : 3.0);
        vec2 f = vec2(u + fid*5.3, p.y);
        // cast relief: a lumpy surface, grooves across, raised panel ribs
        float nz = fbm3(vec2(f.x*2.6, f.y*0.9));
        float gy = abs(fract(p.y/4.0) - 0.5)*4.0;
        float groove = 1.0 - 0.55*halo(gy, 0.12);
        float lipLight = halo(gy - 0.18, 0.08)*0.5;            // the lit lip above each groove
        float panel = halo(abs(abs(u) - 0.42), 0.05)*0.35;     // a raised rib down each face
        vec3 kl = normalize(vec3(-0.45, 0.6, 0.65));
        vec3 v = -rd;
        float dif = max(dot(n, kl), 0.0);
        float spec = pow(max(dot(reflect(-kl, n), v), 0.0), 24.0);
        float fres = pow(1.0 - max(dot(n, v), 0.0), 3.0);
        vec3 bronze = vec3(0.34, 0.27, 0.19)*(0.55 + 0.7*(nz - 0.35))*groove;
        vec3 rgb = bronze*(0.25 + 0.95*dif) + vec3(0.55, 0.48, 0.38)*(lipLight + panel)*(0.4 + 0.6*dif);
        rgb += vec3(0.75, 0.82, 0.9)*spec*0.45*(0.6 + 0.4*nz);
        // a cold sheen sliding up the slab
        float sheen = halo(p.y - mod(t*6.0, L + 20.0) + 10.0 - u*2.0, 1.4);
        rgb += vec3(0.45, 0.6, 0.7)*sheen*(0.25 + 0.5*fres);
        rgb += vec3(0.35, 0.5, 0.6)*fres*0.35;
        // frost where it is newest, at the climbing front
        rgb = mix(rgb, vec3(0.78, 0.88, 0.94)*(0.6 + 0.4*nz), halo(p.y - cb_front(), 1.3)*0.7*smoothstep(2.6, 1.4, cb_ph()));
        // the light inside beating through the cracks, the glow under the surface around them
        float cr = cb_cracks(f, px);
        float glowIn = (0.5 + 1.5*cb_beat())*(0.5 + 0.9*cb_crack());
        rgb += vec3(0.75, 0.95, 1.0)*cr*glowIn*1.2;
        rgb += vec3(0.25, 0.55, 0.8)*halo(abs(u), 0.5)*0.12*glowIn*smoothstep(0.4, 1.2, cb_crack());
        // steam that hangs in front of the slab
        rgb += cb_steam(ro, rd, hit ? tt : tmin)*uFx;
        col = vec4(rgb, cov);
        tHit = hit ? tt : tmin;
      }
    }
  }
  // ---- the burst: shards of slab flying outward all round, lit by the freed blade ----
  float bu = cb_burst();
  if (bu > 0.0 && bu < 1.0){
    float bestT = 1e5; vec4 sc = vec4(0.0);
    for (int i = 0; i < 30; i++){
      float fi = float(i);
      float a = h11(fi*1.7 + 0.3);
      float y0 = (0.03 + 0.94*fract(fi*0.381 + 0.07))*L;
      float vr = 2.2 + 3.6*h11(fi*5.1);
      vec3 c = around(0.55 + vr*bu, a, y0 + 2.2*(h11(fi*7.3) - 0.35)*bu - 2.5*bu*bu);
      float sz = (0.75 + 0.55*h11(fi*9.1))*(1.0 - smoothstep(0.55, 1.0, bu));
      float th; vec2 sh;
      float cv = cb_shard(ro, rd, c, sz, bu*(5.0 + 7.0*h11(fi)), fi, px, th, sh);
      if (cv > 0.0 && th < bestT){
        bestT = th;
        vec3 rgb = vec3(0.30, 0.24, 0.17)*sh.x*1.2 + vec3(0.7, 0.92, 1.0)*sh.y*1.6*(1.0 - bu*0.7);
        sc = vec4(rgb, cv*(1.0 - smoothstep(0.7, 1.0, bu)));
      }
    }
    if (sc.a > 0.0){
      sc.rgb += cb_steam(ro, rd, bestT);
      col = vec4(mix(col.rgb, sc.rgb, sc.a), max(col.a, sc.a));
    }
  }
  // ---- a clash chips the slab: a few shards blown off where it was struck ----
  if (uClash > 0.01 && cb_held() > 0.5){
    float a = 1.0 - uClash;
    for (int i = 0; i < 6; i++){
      float fi = float(i);
      vec3 c = vec3(0.0, L*0.6, 0.0) + vec3(sin(fi*2.4), 0.6*(h11(fi*3.0) - 0.4), cos(fi*2.4))*(0.8 + 4.5*a);
      float th; vec2 sh;
      float cv = cb_shard(ro, rd, c, 0.4*(1.0 - a*0.6), a*9.0, fi + 40.0, px, th, sh);
      if (cv > 0.0 && th < tHit){
        col = vec4(mix(col.rgb, vec3(0.32, 0.26, 0.19)*sh.x + vec3(0.6, 0.9, 1.0)*sh.y*1.5*uClash, cv), max(col.a, cv));
      }
    }
  }
  col.a *= uFx;
  return col;
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime;
  float top = cb_btop();
  float held = cb_held();
  float bu = cb_burst();
  // the freed blade blazes as the slab bursts
  float flare = 1.0 + 0.9*(1.0 - bu)*step(0.001, bu);
  vec3 c = vec3(0.6, 0.88, 1.0);
  vec3 light = blade3(ro, rd, px, c, 0.36, top)*flare;
  float ta;
  float dAx = raySeg(ro, rd, vec3(0.0, CB_Y0, 0.0), vec3(0.0, min(cb_stop(), top), 0.0), ta);
  if (uFx > 0.01){
    // trapped light: a soft heartbeat glow round the frozen slab
    light += c*halo(max(dAx - CB_W*1.1, 0.0), 0.55)*held*cb_beat()*0.45*cb_crack()*step(CB_Y0 + 0.2, cb_stop())*uFx;
    // the burst: the freed blade's blaze lights the air round it
    if (bu > 0.0 && bu < 1.0) light += c*halo(max(dAx, 0.0), 1.5 + 3.0*bu)*(1.0 - bu)*0.6*uFx;
    // clash: a ring of light round the struck point, and a flash
    if (uClash > 0.01){
      float a = 1.0 - uClash;
      vec3 h = rayAt(ro, rd, L*0.6);
      if (h.z > 0.0) light += vec3(0.75, 0.95, 1.0)*halo(abs(length(h.xy) - 0.8 - 6.0*a), 0.25)*1.4*uClash*uFx;
      float tc; float dc = rayPt(ro, rd, vec3(0.0, L*0.6, 0.0), tc);
      light += vec3(0.75, 0.95, 1.0)*halo(dc, 1.4)*pow(uClash, 2.0)*uFx;
    }
    // the vapour all round
    light += cb_steam(ro, rd, 1e5)*uFx;
  }
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return vec3(0.0); }
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
