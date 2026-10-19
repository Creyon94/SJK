// Starforge: a kyber blade being forged inside a star. A molten white-gold core burns under a living
// plasma surface that boils like the face of a sun, and solar flares rise off it in glowing loops that
// arch out round the blade and fall back. In a steady rhythm the forge strikes: a white flash where
// the blow lands, a hot spot that cools back to gold, bright shock rings racing up and down the blade
// and a burst of sparks flying out on every side. Embers drift all round inside a wide warm heat glow.
// A swing makes it burn brighter and throw more sparks (the rhythm never speeds up); a clash is a huge
// blow with a ring of sparks.
// @part The forge
// @arcs Prominences
// @motes Rising sparks
// @room 10
// @palette {"corona.rim_cool":[0.55,0.12,0.02],"corona.rim_hot":[1.0,0.55,0.12],"corona.inner":[1.0,0.88,0.55],"core.white":[1.0,0.96,0.85],"core.fringe_cool":[1.0,0.40,0.08],"core.fringe_hot":[1.0,0.80,0.45],"light.color":[1.0,0.58,0.18],"trail":[1.0,0.55,0.15]}


// one hammer blow every SF_P seconds, always (a swing never changes the rate)
const float SF_P = 1.25;
float sf_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
// star colour by temperature: deep red, orange, gold, white
vec3 sf_heat(float k){
  vec3 c = mix(vec3(0.45, 0.05, 0.0), vec3(1.0, 0.38, 0.05), smoothstep(0.0, 0.45, k));
  c = mix(c, vec3(1.0, 0.75, 0.32), smoothstep(0.4, 0.85, k));
  return mix(c, vec3(1.0, 0.96, 0.86), smoothstep(0.85, 1.25, k));
}
// the ray's distance to a ring of radius R round the blade at height y: the nearer of where the
// ray crosses the ring's plane and where it crosses the ring's cylinder (smooth from any side)
float sf_ring(vec3 ro, vec3 rd, float y, float R){
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
// a blow's height and side
float sf_y(float n){ return uLength*(0.18 + 0.68*h11(n*3.71 + 0.37)); }

vec3 r8_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, top = sf_top();
  float st = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float fx = uFx;
  vec3 white = vec3(1.0, 0.96, 0.86), gold = vec3(1.0, 0.72, 0.28), hot = vec3(1.0, 0.42, 0.07);
  float n = floor(t/SF_P), ph = fract(t/SF_P)*SF_P;
  float ys = sf_y(n), yp = sf_y(n - 1.0);
  float as = h11(n*1.37 + 0.2);

  // The blade itself is Sol's glow and core from the skin's file (sig_glow, sig_core);
  // everything here is a piece placed round it in 3D, each one facing the camera.
  float s;
  vec3 col = vec3(0.0);
  if (top < 0.2) return col;

  // ---- the plasma surface: granules of star-stuff boiling up round the blade ----
  float ta;
  for (int i = 0; i < 26; i++){
    float fi = float(i);
    float f = fract(t*(0.22 + 0.12*h11(fi*1.7)) + h11(fi*4.3));
    float yy = -0.3 + f*(top + 0.3);
    vec3 p = around(0.55 + 0.35*h11(fi*2.9 + floor(t*0.22 + h11(fi*4.3))), h11(fi*6.1) + t*0.05, yy);
    float dd = rayPt(ro, rd, p, ta);
    float life = sin(3.1416*f);
    float spt = exp(-pow((yy - ys)/2.0, 2.0))*exp(-ph*1.8);
    float g = 0.5 + 0.5*sin(t*6.0 + fi*2.7);
    col += sf_heat(0.55 + 0.4*g + spt)*(halo(dd, max(0.16 + 0.1*g, px))*1.1 + halo(dd, 0.6)*0.2)*life*(1.0 + 0.5*st + spt)*fx;
  }

  // the rest is out of the hilt's way: skip rays far from the blade
  float far = raySeg(ro, rd, vec3(0.0, -1.0, 0.0), vec3(0.0, top + 1.0, 0.0), s);
  if (far > 9.0) return col;

  // ---- solar flares: plasma loops arching off the blade and falling back ----
  for (int i = 0; i < 7; i++){
    float fi = float(i);
    float cyc = t/3.6 + fi*0.173 + h11(fi*5.3)*0.5;
    float k = floor(cyc), q = fract(cyc);
    float y0 = (0.08 + 0.78*h11(k*2.13 + fi*7.7))*L;
    float dy = 4.0 + 4.0*h11(k*3.9 + fi);
    if (y0 + dy > top) continue;
    float H = (3.2 + 3.0*h11(k*1.7 + fi*2.0))*smoothstep(0.0, 0.45, q);
    float a = h11(k*6.1 + fi*1.3);
    float env = smoothstep(0.0, 0.12, q)*(1.0 - smoothstep(0.65, 1.0, q));
    if (far > H + 3.0) continue;
    float best = 1e3, bs = 0.0;
    vec3 prev = around(0.4, a, y0);
    for (int j = 1; j <= 8; j++){
      float f = float(j)/8.0;
      vec3 qq = around(0.4 + H*sin(3.1416*f), a + 0.06*sin(3.1416*f) + 0.02*sin(t*1.3 + fi), y0 + dy*f);
      float dd = raySeg(ro, rd, prev, qq, s);
      if (dd < best){ best = dd; bs = (float(j) - 1.0 + s)/8.0; }
      prev = qq;
    }
    float foot = 1.0 - 0.5*sin(3.1416*bs);
    float fl = 0.45 + 1.0*vn(vec2(bs*9.0 - t*2.5, fi*3.0 + k));
    col += (sf_heat(0.7 + 0.45*fl)*halo(best, max(0.07 + 0.16*fl, px))*1.3*fl + hot*halo(best, 0.7)*0.6 + vec3(1.0, 0.25, 0.03)*halo(best, 1.4)*0.16)*env*foot*fx;
  }

  // ---- the hammer blow: flash, shock rings, sparks ----
  if (ys < top){
    vec3 o = around(0.45, as, ys);
    float dp = rayPt(ro, rd, o, ta);
    float k = exp(-ph*7.0);
    col += white*(halo(dp, 0.4 + 0.9*k)*2.2 + exp(-dp/2.5)*0.5)*k*fx;
    // rings racing up and down the blade, and one spreading out flat
    float fade = exp(-ph*2.2)*smoothstep(0.0, 0.04, ph);
    float yu = ys + 20.0*ph, yd = ys - 13.0*ph;
    float ru = 1.0 + 3.0*ph;
    if (yu < top){ float du = sf_ring(ro, rd, yu, ru); col += (white*halo(du, max(0.06, px))*1.4 + gold*halo(du, 0.35)*0.5)*fade*fx; }
    if (yd > 0.0){ float dr = sf_ring(ro, rd, yd, ru); col += (white*halo(dr, max(0.06, px))*1.4 + gold*halo(dr, 0.35)*0.5)*fade*fx; }
    float rf = 0.6 + 9.0*ph;
    float dfl = sf_ring(ro, rd, ys, rf);
    col += (gold*halo(dfl, max(0.1, px))*1.8 + hot*halo(dfl, 0.7)*0.5)*fade*(1.0 - smoothstep(0.3, 0.9, ph))*fx;
    // sparks flying out in all directions
    float dist0 = rayPt(ro, rd, vec3(0.0, ys, 0.0), ta);
    if (dist0 < 9.0){
      for (int i = 0; i < 22; i++){
        float fi = float(i);
        float sa = as + (h11(fi*3.1 + n) - 0.5)*0.9;
        float up = (h11(fi*7.3 + n*1.3) - 0.35)*1.2;
        vec3 dir = normalize(vec3(sin(6.2832*sa), up, cos(6.2832*sa)));
        float v = 9.0 + 9.0*h11(fi*1.7 + n*0.7);
        float tp = ph, tq = max(ph - 0.05, 0.0);
        vec3 p1 = o + dir*v*(1.0 - exp(-tp*2.0))/2.0 + vec3(0.0, 1.5*tp*tp, 0.0);
        vec3 p0 = o + dir*v*(1.0 - exp(-tq*2.0))/2.0 + vec3(0.0, 1.5*tq*tq, 0.0);
        float dd = raySeg(ro, rd, p0, p1, s);
        float life = (1.0 - smoothstep(0.35, 1.1, ph))*(0.7 + 0.6*st);
        col += mix(white, hot, smoothstep(0.0, 0.8, ph))*(halo(dd, max(0.06, px))*2.4 + halo(dd, 0.3)*0.45)*life*fx;
      }
    }
  }

  // ---- embers drifting round in the heat ----
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float f = fract(t*(0.05 + 0.03*h11(fi*2.9)) + h11(fi*5.1));
    float yy = f*(top + 3.0) - 1.0;
    vec3 p = around(1.3 + 3.0*h11(fi*3.7), h11(fi*1.9) + t*0.03 + 0.1*sin(t*0.7 + fi), yy);
    float dd = rayPt(ro, rd, p, ta);
    float tw = 0.6 + 0.4*sin(t*5.0 + fi*2.3);
    col += hot*(halo(dd, max(0.06, px))*1.6 + halo(dd, 0.3)*0.15)*tw*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.75, 1.0, f))*(1.0 + st)*fx;
  }

  // ---- clash: a huge blow and a ring of sparks ----
  if (cl > 0.01){
    float a = 1.0 - cl;
    float yc = min(L*0.6, top);
    float dc = rayPt(ro, rd, vec3(0.0, yc, 0.0), ta);
    col += white*(halo(dc, 1.5)*2.5 + halo(dc, 3.2)*0.7)*cl*cl;
    float dr = sf_ring(ro, rd, yc, 0.6 + 7.0*(1.0 - exp(-3.0*a)));
    col += mix(white, hot, a)*(halo(dr, max(0.08, px))*1.8 + halo(dr, 0.5)*0.4)*cl;
  }
  return col;
}

// the blade: Sol's glow and core from the file, white-hot where the last blow landed
float sf_spot(float y){
  float n = floor(uTime/SF_P), ph = fract(uTime/SF_P)*SF_P;
  return exp(-pow((y - sf_y(n))/1.6, 2.0))*exp(-ph*1.8) + 0.25*clamp(uClash, 0.0, 1.0);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){ return g*(1.0 + 0.25*sin(uTime*2.1 + b.y*0.4)*0.4 + 1.2*sf_spot(b.y) + 0.3*uSwing); }
vec3 sig_core(vec3 c, vec2 b, float px){ return c*(1.0 + 0.8*sf_spot(b.y)); }

// ---- round 9: the big effects layer ----
// Round 11 (Creyon): its own look, Sol's way: its round-8 effects as part 0, its own parts
// (`part(k)`, the `// @part` lines) and Sol's pieces from its file; nothing shared with the others.
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 col = r8_vol(ro, rd, px)*part(0.0);
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += fxAura(ro, rd, y0, y1, 3.6, vec3(1.0, 0.42, 0.07))*part(0.0);
  return col + fx*uFx;
}
