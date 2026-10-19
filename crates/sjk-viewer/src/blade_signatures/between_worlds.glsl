// Between Worlds: a blade from the World Between Worlds, the path of light through the void
// outside time. A calm white-gold blade stands in its own small starry void. Faint echoes of the
// blade from other moments drift beside it in depth, translucent, fading in and out like ripples
// of time. Voices of light, soft streams, flow along it from both ends, from the hilt up and from
// the tip down, passing each other. Soft portal rings of shimmering light open and close along it,
// and star dust drifts slowly through the dark. A swing scatters the echoes wide; a clash makes
// every portal flash open at once.
// @room 10
// @palette {"corona.rim_cool":[0.42,0.34,0.16],"corona.rim_hot":[1.0,0.86,0.55],"corona.inner":[1.0,0.96,0.85],"core.white":[1.0,1.0,0.97],"core.fringe_cool":[0.90,0.78,0.50],"core.fringe_hot":[1.0,0.90,0.65],"light.color":[1.0,0.90,0.66],"trail":[1.0,0.90,0.68]}


// the ray's distance to a ring of radius R round the blade at height h
float bww_ring(vec3 ro, vec3 rd, float h, float R){
  float best = 1e3;
  vec3 q = rayAt(ro, rd, h);
  if (q.z > 0.0) best = abs(length(q.xy) - R);
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  float dsc = b*b - a*c;
  if (dsc > 0.0 && a > 1e-6){
    float k = sqrt(dsc);
    float t0 = (-b - k)/a, t1 = (-b + k)/a;
    if (t0 > 0.0) best = min(best, abs(ro.y + rd.y*t0 - h));
    if (t1 > 0.0) best = min(best, abs(ro.y + rd.y*t1 - h));
  }
  return best;
}

vec3 r8_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float top = L*clamp(uAge/0.45, 0.0, 1.0);
  float fxOn = uFx*smoothstep(0.3, 1.2, uAge);
  vec3 gold = vec3(1.0, 0.80, 0.45);
  vec3 pale = vec3(1.0, 0.95, 0.85);
  vec3 dusk = vec3(0.55, 0.45, 0.30);

  // The blade itself is Sol's glow and core from the file (sig_glow, sig_core).
  vec3 light = vec3(0.0);

  if (fxOn < 0.001) return light;

  // echoes of the blade from other moments: faint copies offset in depth, drifting and fading
  for (int e = 0; e < 4; e++){
    float fe = float(e);
    float per = 6.0 + 1.7*fe;
    float f = fract(uTime/per + fe*0.27);
    float vis = smoothstep(0.0, 0.25, f)*(1.0 - smoothstep(0.55, 1.0, f));
    float ang = h11(fe*3.3 + floor(uTime/per + fe*0.27)*1.7);
    float off = (0.9 + 1.6*f + 2.5*sw)*(1.0 - 0.6*cl);
    vec3 a0 = around(off*0.35, ang, 0.6);
    vec3 a1 = around(off, ang + 0.06*sin(uTime*0.3 + fe), min(top, L*(0.92 + 0.05*sin(uTime*0.4 + fe))));
    float se;
    float de = raySeg(ro, rd, a0, a1, se);
    float tf = smoothstep(0.0, 0.08, se)*(1.0 - smoothstep(0.85, 1.0, se));
    float flick = 0.8 + 0.2*sin(se*30.0 - uTime*3.0 + fe);
    vec3 ec = e == 1 ? mix(gold, vec3(1.0, 0.65, 0.30), 0.6) : (e == 3 ? mix(pale, vec3(0.75, 0.85, 1.0), 0.5) : pale);
    light += ec*(exp(-pow(de/max(0.16, px), 2.0))*0.55 + halo(de, 0.6)*0.22)*vis*tf*flick;
  }

  // voices of light: soft streams winding loosely along the blade, two rising from the hilt and
  // two falling from the tip, glowing pulses travelling along them
  for (int k = 0; k < 4; k++){
    float fk = float(k);
    float dir = k < 2 ? 1.0 : -1.0;
    float rr = 0.75 + 0.35*fk;
    float turns = (0.55 + 0.18*fk)*dir;
    float ph = fk*0.31 + uTime*0.04*dir;
    vec3 prev = around(rr*0.4, ph, 0.0);
    for (int j = 1; j <= 16; j++){
      float fj = float(j)/16.0;
      float y = fj*L;
      float rj = rr*(0.4 + 0.6*sin(min(fj*1.6, 1.0)*1.5708))*(1.0 - 0.5*smoothstep(0.85, 1.0, fj)) + 0.10*sin(fj*4.0 + uTime*0.5 + fk);
      vec3 q = around(rj, ph + fj*turns, y);
      if (y - L/16.0 < top){
        float sg;
        float d = raySeg(ro, rd, prev, q, sg);
        if (d < 2.0){
          float u = (fj - (1.0 - sg)/16.0);
          // travel: rising streams move up, falling ones down
          float pos = dir > 0.0 ? u : 1.0 - u;
          float pulse = pow(0.5 + 0.5*sin(pos*14.0 - uTime*(2.2 + 2.0*sw) + fk*1.7), 3.0);
          float vis = smoothstep(0.0, 0.12, pos)*(1.0 - smoothstep(0.8, 1.0, pos));
          vec3 vc = dir > 0.0 ? gold : pale;
          float w = max(px*0.8, 0.05);
          light += (vec3(1.0)*halo(d, w)*0.45*pulse + vc*halo(d, max(w*2.5, 0.14))*(0.10 + 0.45*pulse) + vc*halo(d, 0.7)*0.06*(0.3 + pulse))*vis;
        }
      }
      prev = q;
    }
  }

  // portals: soft shimmering rings opening and closing along the blade, a veil of light inside
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    float per = 5.5 + 1.9*fk;
    float f = fract(uTime/per + fk*0.37);
    float open = sin(clamp(f/0.7, 0.0, 1.0)*3.1416);
    open = max(open, cl*0.45);
    float h = L*(0.25 + 0.27*fk) + 1.2*sin(uTime*0.25 + fk*2.0);
    if (h < top && open > 0.01){
      float R = 0.6 + 2.4*open + 4.0*cl*(1.0 - cl);
      float dr = bww_ring(ro, rd, h, R);
      float sh = 0.7 + 0.3*sin(uTime*1.3 + h);
      light += (pale*halo(dr, max(0.05, px))*1.2 + mix(gold, pale, 0.5)*halo(dr, 0.3 + 0.3*open)*0.45)*open*sh*(0.6 + 0.5*cl);
    }
  }

  // the starry void: tiny stars round the blade, some slowly drifting, twinkling
  for (int i = 0; i < 34; i++){
    float fi = float(i);
    float rr = 1.6 + 3.8*h11(fi*2.7);
    float a = h11(fi*6.1) + uTime*0.004*(h11(fi*4.4) - 0.5);
    float y = mod(L*h11(fi*8.3) + uTime*(0.15 + 0.25*h11(fi*1.1))*(i < 17 ? 1.0 : -1.0), L + 2.0) - 0.5;
    vec3 q = around(rr, a, y);
    if (q.y < top){
      float tt; float d = rayPt(ro, rd, q, tt);
      float tw = 0.4 + 0.6*pow(0.5 + 0.5*sin(uTime*(1.3 + 2.0*h11(fi*5.5)) + fi*3.0), 2.0);
      float edge = smoothstep(-0.5, 1.0, y)*(1.0 - smoothstep(L - 1.0, L + 1.5, y));
      float w = max(px*0.9, 0.035);
      vec3 sc = h11(fi*9.9) > 0.6 ? vec3(0.80, 0.86, 1.0) : pale;
      light += (sc*halo(d, w)*1.4 + sc*halo(d, w*3.5)*0.18)*tw*edge;
    }
  }

  return light;
}

// the blade: Sol's glow and core from the file, breathing calmly
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.85 + 0.15*sin(b.y*0.5 - uTime*0.8) + 0.10*sin(b.y*0.21 + uTime*0.5) + 0.5*clamp(uClash, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }

// ---- round 9: the big effects layer ----
// Creyon's round 9: every mythical dramatic, with big effects all round it in 3D (lab.glsl fx*).
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  vec3 col = r8_vol(ro, rd, px);
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);

  vec3 gold = vec3(1.0, 0.85, 0.55), pale = vec3(1.0, 0.96, 0.88), ahs = vec3(0.75, 0.88, 1.0), ana = vec3(1.0, 0.12, 0.06);
  float s, ta;
  if (fxNear(ro, rd, y0, y1) < 10.0){
    // pathways: long straight lines of light through the void round the blade, pulses running on them
    for (int i = 0; i < 9; i++){
      float fi = float(i);
      float a = h11(fi*3.3) + t*0.01, R = 3.0 + 3.0*h11(fi*5.7);
      float y = mix(y0, y1, h11(fi*7.1));
      vec3 c = around(R, a, y);
      vec3 dir = normalize(vec3(cos(6.2832*a), (h11(fi*2.9) - 0.5)*1.6, -sin(6.2832*a)));
      float len = 6.0 + 6.0*h11(fi*1.3);
      vec3 pa = c - dir*len, pb = c + dir*len;
      float d = raySeg(ro, rd, pa, pb, s);
      float vis = 0.55 + 0.45*sin(t*0.3 + fi*1.7);
      float pulse = pow(0.5 + 0.5*sin(s*18.0 - t*(2.0 + 0.5*fi)), 8.0);
      float ends = smoothstep(0.0, 0.15, s)*(1.0 - smoothstep(0.85, 1.0, s));
      fx += (vec3(1.0)*halo(d, max(0.04, px))*(0.35 + 1.4*pulse) + gold*halo(d, 0.25)*(0.25 + 0.6*pulse))*ends*vis;
      fx += fxFlare(ro, rd, c, px, 0.12, 1.8, 4.0, 0.0, gold)*vis*0.6;
    }
    fx += fxMotes(ro, rd, px, y0, y1, 1.0, 6.5, 30.0, 0.04, 2.2, pale);
    fx += fxAura(ro, rd, y0, y1, 3.0, gold)*0.7;
  }
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 10.0, pale)*1.6;
  return col + fx*uFx;
}

// round 10: the duel circles the blade, so it alone is drawn in true 3D (Sol's way)
vec3 sig_orbit(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);

  vec3 pale = vec3(1.0, 0.96, 0.88), ahs = vec3(0.75, 0.88, 1.0), ana = vec3(1.0, 0.12, 0.06);
  float s, ta;
    // the duel: two lights circling the blade on crossing paths, trailing, meeting in a flash
    float P = 4.0, ph = fract(t/P), k = floor(t/P);
    for (int j = 0; j < 2; j++){
      float fj = float(j);
      vec3 cc = j == 0 ? ahs : ana;
      float best = 1e3, bs = 0.0;
      vec3 prev = vec3(0.0);
      for (int i = 0; i <= 10; i++){
        float tt = t - float(i)*0.08;
        float a = tt*(0.32 + 0.04*fj)*(fj == 0.0 ? 1.0 : -1.0) + fj*0.5;
        vec3 p = around(2.6 + 1.2*sin(tt*0.9 + fj*2.0), a, L*0.6 + L*0.25*sin(tt*0.55 + fj*3.1));
        if (i > 0){ float d = raySeg(ro, rd, prev, p, s); if (d < best){ best = d; bs = (float(i) - 1.0 + s)/10.0; } }
        prev = p;
      }
      float tail = 1.0 - bs;
      fx += vec3(1.0)*halo(best, max(0.1, px))*tail*2.2 + cc*halo(best, 0.5)*tail*1.8 + cc*halo(best, 1.6)*0.35*tail;
    }
    // where they meet: a flash and sparks, white and red
    vec3 meet = around(3.0, h11(k*2.1), mix(L*0.4, L*0.85, h11(k*4.7)));
    float fl = exp(-ph*7.0);
    fx += fxFlare(ro, rd, meet, px, 0.25, 5.0, 6.0, k, mix(ahs, ana, 0.5))*fl*1.4;
    fx += fxShock(ro, rd, px, meet.y, ph*1.6, 6.0, pale)*0.8;
  return fx*uFx;
}
