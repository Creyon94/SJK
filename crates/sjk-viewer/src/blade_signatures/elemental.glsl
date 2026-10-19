// Elemental: fire, ice, lightning and stone take the blade in turn, four seconds each. Each new ruler
// climbs from the hilt behind a bright seam, overrunning the last one
// while it reigns: a licking flame with rising embers, faceted ice with frost and snow glints, a
// violet core wrapped in crawling arcs, and dark granite split by glowing seams. Swings stoke
// whichever element rules; clashes flash it.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.35,0.20,0.55],"corona.rim_hot":[0.90,0.60,0.50],"corona.inner":[1.0,0.92,0.85],"core.white":[1.0,0.98,0.95],"core.fringe_cool":[0.50,0.70,1.0],"core.fringe_hot":[1.0,0.65,0.35],"light.color":[0.95,0.75,0.60],"trail":[0.90,0.70,0.60]}

float L_EL(){ return uLength; }
const float EL_P = 6.0;      // seconds each element rules (Creyon: change them more slowly)
float el_u(){ return (uTime - 1.0)/EL_P; }
float el_cur(){ return mod(floor(el_u()), 4.0); }
float el_local(){ return fract(el_u())*EL_P; }
float el_front(){ return -1.0 + (uLength + 4.0)*smoothstep(0.0, 1.3, el_local()); }
vec3 el_tone(float k){ return k < 0.5 ? vec3(1.0, 0.55, 0.15) : k < 1.5 ? vec3(0.55, 0.88, 1.0) : k < 2.5 ? vec3(0.75, 0.45, 1.0) : vec3(1.0, 0.72, 0.30); }
// one element's light at b; s returns how much opaque stone covers it
vec3 el_draw(float k, vec2 b, float px, out float s){
  float t = uTime, L = uLength, ax = abs(b.x);
  float on = smoothstep(-0.2, 0.3, b.y)*(1.0 - smoothstep(L - 0.3, L + 0.6, b.y));
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*4.0));
  float st = uSwing + uClash*0.6;
  s = 0.0;
  vec3 c = vec3(0.0);
  if (k < 0.5){
    // FIRE: a licking flame body, white at the heart, embers rising off it
    float n = fbm3(vec2(b.x*1.6, b.y*0.45 - t*3.2));
    float n2 = vn(vec2(b.x*3.0, b.y*1.2 - t*6.0));
    float w = 0.62 + 0.45*n + 0.2*n2 + 0.3*st;
    float fl = smoothstep(w, w - 0.55, ax)*on;
    float heat = smoothstep(w - 0.2, 0.0, ax);
    c = mix(vec3(0.85, 0.12, 0.02), vec3(1.0, 0.55, 0.08), smoothstep(0.0, 0.5, heat));
    c = mix(c, vec3(1.0, 0.95, 0.7), smoothstep(0.55, 0.95, heat))*fl*1.6;
    c += vec3(1.0, 0.35, 0.05)*halo(bd, 1.5)*0.5*on;
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float age = fract(t*0.6 + h11(fi*2.1));
      float y = mod(h11(fi*7.7)*L + age*7.0, L);
      vec2 p = vec2((h11(fi*3.3) - 0.5)*1.6 + 0.5*sin(t*2.0 + fi)*age, y);
      c += vec3(1.0, 0.6, 0.15)*fillD(length(b - p) - 0.06, px)*(1.0 - age)*1.8;
    }
  } else if (k < 1.5){
    // ICE: faceted crystal, bright facet edges, frost needles at the rim, snow glints
    vec2 cell; float e = voro(vec2(b.x*1.6, b.y*0.45), cell);
    float body = fillD(ax - 0.78, px)*on;
    float face = 0.55 + 0.45*h21(cell);
    c = mix(vec3(0.25, 0.55, 0.95), vec3(0.85, 0.97, 1.0), face)*body*0.75;
    c += vec3(0.9, 1.0, 1.0)*halo(e, 0.05)*body*1.2;
    c += vec3(1.0)*fillD(ax - 0.2, px)*on*0.8;
    // frost needles: short spikes along both edges
    float yy = b.y*2.2; float id = floor(yy); float fy = fract(yy) - 0.5;
    float len = 0.2 + 0.35*h11(id + step(0.0, b.x)*31.0) + 0.3*st;
    float sp = (ax - 0.78)/len;
    float needle = step(0.0, sp)*step(sp, 1.0)*step(abs(fy), 0.18*(1.0 - sp));
    c += vec3(0.75, 0.95, 1.0)*needle*on*0.9;
    c += vec3(0.5, 0.8, 1.0)*halo(bd, 1.6)*0.45*on;
    vec2 gp = b*vec2(2.0, 1.0); vec2 gc = floor(gp);
    float glint = fillD(length(fract(gp) - h22(gc)) - 0.05, px*2.0)*step(0.7, h21(gc))*(0.5 + 0.5*sin(t*5.0 + gc.x*13.0 + gc.y*7.0));
    c += vec3(1.0)*glint*(1.0 - smoothstep(1.2, 2.4, ax))*on*1.5;
  } else if (k < 2.5){
    // LIGHTNING: a violet-white core wrapped in jagged arcs that crawl and re-strike
    float tq = floor(t*14.0);
    c = vec3(0.65, 0.45, 1.0)*halo(bd, 1.2)*0.6*on + vec3(0.9, 0.85, 1.0)*fillD(ax - 0.28, px)*on*1.2;
    for (int i = 0; i < 3; i++){
      float fi = float(i);
      float j = b.y*1.3 + tq*0.37 + fi*11.0;
      float x = (vn(vec2(j, fi*5.0 + tq*0.11)) - 0.5)*1.5 + (vn(vec2(j*4.0, fi + tq))*0.5 - 0.25)*0.6;
      x *= 1.0 + 0.4*st;
      float bolt = halo(abs(b.x - x), 0.04 + px)*1.8 + halo(abs(b.x - x), 0.25)*0.5;
      float live = step(0.25, h11(fi + tq*0.71));
      c += vec3(0.85, 0.75, 1.0)*bolt*live*on;
    }
    float fl = 0.8 + 0.4*h11(tq);
    c *= fl;
  } else {
    // STONE: dark granite, speckled, split by seams of glowing magma-gold
    float body = fillD(ax - 0.82, px)*on;
    vec2 cell; float e = voro(vec2(b.x*1.3, b.y*0.38), cell);
    float seam = halo(e, 0.06)*(0.7 + 0.6*vn(vec2(b.y*0.5 - t*0.8, cell.x)));
    float pulse = 0.75 + 0.25*sin(t*3.0 + b.y*0.3) + st;
    c = vec3(1.0, 0.65, 0.20)*seam*body*1.6*pulse + vec3(1.0, 0.55, 0.15)*halo(bd, 1.4)*0.45*on;
    s = body*(1.0 - smoothstep(0.1, 0.5, seam));
    // a few grains of grit falling off
    for (int i = 0; i < 6; i++){
      float fi = float(i);
      float age = fract(t*0.5 + h11(fi*4.1));
      vec2 p = vec2((h11(fi*1.9) < 0.5 ? -1.0 : 1.0)*(0.9 + age*0.6), h11(fi*6.3)*L - age*age*6.0);
      c += vec3(0.8, 0.6, 0.4)*fillD(length(b - p) - 0.07, px)*(1.0 - age)*0.8;
    }
  }
  return c;
}
float el_stoneRgb(vec2 b){ vec2 g = floor(b*vec2(9.0, 7.0)); return 0.16 + 0.10*h21(g) + 0.08*vn(b*vec2(2.0, 0.7)); }
// both rulers blended across the climbing front
vec3 el_all(vec2 b, float px, out float s){
  float cur = el_cur(), prev = mod(cur + 3.0, 4.0);
  float f = el_front();
  float m = smoothstep(f + 0.4, f - 0.4, b.y);
  vec3 c = vec3(0.0); s = 0.0;
  float s1 = 0.0, s2 = 0.0;
  if (m > 0.001) c += el_draw(cur, b, px, s1)*m;
  if (m < 0.999) c += el_draw(prev, b, px, s2)*(1.0 - m);
  s = s1*m + s2*(1.0 - m);
  // the front itself: a bright seam where one element overruns the other
  float L = uLength;
  float band = halo(abs(b.y - f), 0.35)*(1.0 - smoothstep(0.9, 1.6, abs(b.x)))*step(f, L + 1.0);
  c += mix(el_tone(cur), vec3(1.0), 0.4)*band*1.5;
  // (no orb or gem on the tip: Creyon's rule, no orbs in the blade)
  return c;
}
vec3 elemental_glow0(vec3 g, vec2 b, float r, float px){
  float s; vec3 c = el_all(b, px, s);
  float bd = length(vec2(b.x, max(b.y - uLength, 0.0) + max(-b.y, 0.0)*4.0));
  return c + vec3(1.0)*uClash*halo(bd, 1.3)*0.8 + g*0.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
vec4 sig_solid(vec4 o, vec2 b, float px){
  float s; vec3 c = el_all(b, px, s);
  if (s <= 0.001) return o;
  vec3 rock = vec3(el_stoneRgb(b))*vec3(1.0, 0.95, 0.9);
  float rim = smoothstep(0.55, 0.82, abs(b.x));
  rock *= 1.0 + 0.5*rim;
  vec3 rgb = rock + min(c, vec3(1.0))*0.6;
  return vec4(mix(o.rgb, rgb, s), max(o.a, s));
}

// ==== show (added) ====
// Elemental show: the ruling element throws its own effect off the blade: fire sheds embers and
// curling flame tongues, ice drifts six-armed snowflakes and frost shards, lightning forks bolts off
// to the sides, stone flings rock chips and dust. A swing throws more; a clash bursts the ruler's
// element out in a ring.
vec3 elemental_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float k = el_cur();
  float rise = 0.35 + 0.65*smoothstep(0.3, 1.6, el_local());                 // the new ruler's show starts as it takes over
  vec3 tone = el_tone(k);
  vec3 light = vec3(0.0);
  for (int i = 0; i < 14; i++){
    float fi = float(i);
    float ph = uTime*(0.45 + 0.25*h11(fi*2.3)) + h11(fi*6.7);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.1 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.08 + 0.85*h11(cy*1.7 + fi*4.0))*L;
    float fade = (1.0 - f)*smoothstep(0.0, 0.08, f);
    if (k < 0.5){                                              // fire: embers rising, flame tongues
      vec2 p = vec2(s*(0.6 + 3.5*f) + 0.4*sin(f*9.0 + fi), y0 + 4.0*f);
      light += mix(vec3(1.0, 0.9, 0.5), vec3(1.0, 0.3, 0.05), f)*(halo(length(b - p), 0.14)*2.6 + halo(length(b - p), 0.5)*0.4)*fade;
      float u = (b.x*s - 0.5)/2.5;
      if (u > 0.0 && u < 1.0 && i < 8){
        float dy = b.y - y0 - 2.5*u*u - 0.5*sin(u*8.0 - uTime*10.0)*u;
        light += vec3(1.0, 0.5, 0.1)*exp(-pow(dy/(0.25*(1.0 - u) + 0.04), 2.0))*(1.0 - u)*fade*1.2;
      }
    } else if (k < 1.5){                                       // ice: snowflakes and frost shards
      vec2 p = vec2(s*(0.8 + 5.0*f), y0 - 2.0*f + 0.5*sin(f*6.0 + fi));
      vec2 q = b - p;
      q /= 1.6; float r = length(q), a = atan(q.y, q.x) + uTime*0.8 + fi;
      float arm = halo(abs(sin(a*3.0))*r, 0.03)*step(r, 0.45) + halo(abs(sin(a*3.0 + 1.57))*r, 0.02)*step(r, 0.25)*step(0.12, r);
      light += vec3(0.75, 0.95, 1.0)*(arm*1.4 + halo(r, 0.08))*fade;
    } else if (k < 2.5){                                       // lightning: forks jumping off the sides
      if (i < 10){
        float on = step(0.35, h11(floor(uTime*8.0) + fi*3.0));
        float u = (b.x*s - 0.4)/(3.0 + 3.0*h11(cy + fi));
        if (u > 0.0 && u < 1.0){
          float jag = (vn(vec2(u*12.0, floor(uTime*12.0) + fi)) - 0.5)*1.4*u + (vn(vec2(u*40.0, floor(uTime*12.0) + fi*2.0)) - 0.5)*0.3;
          float dy = b.y - y0 - jag - u*1.5*(h11(cy*5.0 + fi) - 0.5)*2.0;
          light += mix(vec3(1.0), vec3(0.75, 0.5, 1.0), u)*(exp(-pow(dy/0.06, 2.0))*2.4 + halo(abs(dy), 0.4)*0.35)*on*(1.0 - u*0.5);
        }
      }
    } else {                                                   // stone: rock chips flung out, falling, dust
      vec2 v = vec2(s*(3.0 + 3.0*h11(cy + fi)), 2.0*h11(cy*2.0 + fi));
      vec2 p = vec2(s*0.5, y0) + v*f - vec2(0.0, 4.0*f*f);
      vec2 q = b - p;
      float an = uTime*4.0 + fi;
      q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
      float rock = max(abs(q.x) + abs(q.y)*0.6, abs(q.y)) - 0.38 - 0.12*h11(fi);
      light += vec3(0.75, 0.55, 0.35)*(fillD(rock, px)*1.0 + halo(abs(rock), 0.04 + px)*1.4)*fade;
      light += vec3(0.6, 0.5, 0.4)*halo(length(b - p + v*0.08), 0.5)*0.15*fade;
    }
  }
  light *= rise;
  // clash: the ruler's element bursts out in a ring
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 o = vec2(0.0, L*0.6);
    light += tone*(halo(abs(length(b - o) - 1.0 - 6.5*a), 0.14)*1.5 + halo(length(b - o), 1.4)*pow(cl, 3.0))*cl;
  }
  float edge = 1.0 - smoothstep(8.0, 9.8, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}


// ==== aura (round 7) ====
// Each element wraps the blade the way Sol's Storm blade wraps it in lightning: two strands
// winding round it and effects leaping 6 to 8 units off it. Fire: a flaming ribbon spiralling
// up, tongues licking out and sparks flung off. Ice: a chain of frost shards spiralling round,
// glinting, with cold mist and crystal spikes standing off the blade. Lightning: crackling arcs
// round the blade and great loops jumping out and back, struck anew many times a second.
// Stone: boulders with molten cracks orbiting in a helix, lava drips and dust. The next element
// takes over from the hilt behind the seam.
vec3 el_aura(float k, vec2 b, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0);
  vec3 light = vec3(0.0);
  float along = smoothstep(-0.5, 1.5, b.y)*(1.0 - smoothstep(L - 1.0, L + 2.0, b.y));
  if (k < 0.5){
    // fire: two flame ribbons spiralling up, tongues and sparks
    for (int j = 0; j < 2; j++){
      float fj = float(j);
      vec2 h = helix(b.y, 2.4 + 0.6*sin(b.y*0.3 - t*2.0), 1.1, -t*0.55 + fj*0.5);
      float d = abs(b.x - h.x);
      float flick = 0.6 + 0.6*vn(vec2(b.y*0.9 - t*6.0, fj*7.0));
      float front = 0.45 + 0.55*smoothstep(-0.6, 0.6, h.y);
      light += mix(vec3(1.0, 0.35, 0.05), vec3(1.0, 0.85, 0.4), front)*(halo(d, 0.35*flick)*1.3 + halo(d, 1.1)*0.25)*front*along;
    }
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float ph = t*(0.7 + 0.3*h11(fi*3.1)) + h11(fi*7.7);
      float cy = floor(ph), f = fract(ph);
      float s = h11(cy + fi*2.0) < 0.5 ? -1.0 : 1.0;
      float y0 = (0.1 + 0.8*h11(cy*1.3 + fi))*L;
      // a tongue licking out and up
      float u = (b.x*s - 1.0)/(4.0 + 3.0*h11(cy + fi*5.0));
      if (u > 0.0 && u < 1.0){
        float dy = b.y - y0 - 3.0*u*u*f - 0.6*sin(u*9.0 - t*12.0 + fi)*u;
        light += mix(vec3(1.0, 0.8, 0.3), vec3(1.0, 0.25, 0.02), u)*exp(-pow(dy/(0.45*(1.0 - u) + 0.05), 2.0))*(1.0 - u)*(1.0 - f)*smoothstep(0.0, 0.15, f)*1.3;
      }
      // a spark flung far out
      vec2 p = vec2(s*(1.0 + 7.0*f), y0 + 5.0*f - 3.0*f*f);
      light += vec3(1.0, 0.7, 0.3)*halo(length(b - p), max(px, 0.06))*2.2*(1.0 - f);
    }
  } else if (k < 1.5){
    // ice: frost shards on two spirals, glinting; crystal spikes off the blade; cold mist
    for (int j = 0; j < 2; j++){
      float fj = float(j);
      float ph = -t*0.25 + fj*0.5;
      for (int i = 0; i < 9; i++){
        float fi = float(i);
        float y = fract(fi/9.0 + t*0.035 + fj*0.05)*(L + 2.0) - 1.0;
        vec2 h = helix(y, 3.0, 0.9, ph);
        vec2 q = b - vec2(h.x, y);
        q = rot(t*1.5 + fi + fj*3.0)*q;
        float sh = max(abs(q.x)*1.6 + abs(q.y)*0.55, abs(q.y)*0.9) - 0.8*(0.7 + 0.3*h.y);
        float glint = pow(0.5 + 0.5*sin(t*5.0 + fi*2.3 + fj), 8.0);
        light += vec3(0.6, 0.9, 1.0)*(fillD(sh, px)*0.7 + halo(abs(sh), 0.05 + px)*1.4)*(0.5 + 0.5*h.y)*along + vec3(1.0)*halo(length(q), 0.12)*glint*2.0*along;
      }
    }
    for (int i = 0; i < 8; i++){
      float fi = float(i);
      float s = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
      float y0 = (0.1 + 0.11*fi)*L;
      float grow = 0.5 + 0.5*sin(t*0.8 + fi*1.7);
      float len = (2.5 + 3.5*h11(fi*3.3))*grow;
      float d = sdSeg(b, vec2(s*0.6, y0), vec2(s*(0.6 + len), y0 + len*0.55));
      light += vec3(0.75, 0.95, 1.0)*(halo(d, 0.05 + px)*1.2 + halo(d, 0.4)*0.25)*grow;
    }
    float mist = fbm3(vec2(b.x*0.5 - t*0.3, b.y*0.3 - t*0.5));
    light += vec3(0.45, 0.7, 1.0)*mist*mist*0.35*exp(-abs(b.x)/3.0)*along;
  } else if (k < 2.5){
    // lightning: crackling arcs wound round the blade and big loops leaping off it
    for (int j = 0; j < 2; j++){
      float fj = float(j);
      float kk = floor(t*14.0 + fj*0.5);
      float jit = (vn(vec2(b.y*1.7, kk*3.1 + fj*9.0)) - 0.5)*1.6;
      vec2 h = helix(b.y, 1.6 + 0.6*vn(vec2(b.y*0.5, kk)), 1.5, -t*0.8 + fj*0.5);
      float d = abs(b.x - h.x - jit);
      light += mix(vec3(0.7, 0.45, 1.0), vec3(1.0), 0.3)*(halo(d, max(px, 0.04))*1.5 + halo(d, 0.3)*0.5)*(0.4 + 0.6*smoothstep(-0.5, 0.5, h.y))*along;
    }
    for (int i = 0; i < 6; i++){
      float fi = float(i);
      float y0 = (0.06 + 0.15*fi)*L + 2.0*h11(floor(t*1.3 + fi));
      float s = h11(floor(t*2.0) + fi*4.0) < 0.5 ? -1.0 : 1.0;
      float on = step(0.3 - 0.3*sw, h11(floor(t*9.0) + fi*1.9));
      light += arcLoop(b, y0, y0 + 3.0 + 3.0*h11(fi*5.1), s, 7.0 + 3.0*h11(fi + floor(t*3.0)), fi*3.7, 9.0, vec3(0.7, 0.45, 1.0), px)*on;
    }
    for (int i = 0; i < 12; i++){
      float fi = float(i);
      float f = fract(t*0.5 + h11(fi*2.9));
      vec2 p = vec2((h11(fi*3.7) - 0.5)*12.0, f*(L + 4.0) - 2.0);
      light += vec3(0.85, 0.7, 1.0)*halo(length(b - p), 0.08)*1.5*(0.5 + 0.5*sin(t*20.0 + fi));
    }
  } else {
    // stone: boulders with molten cracks on a helix round the blade, lava drips, dust
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float y = fract(fi/10.0 + t*0.03)*(L + 2.0) - 1.0;
      vec2 h = helix(y, 3.8, 0.8, t*0.12 + fi*0.13);
      vec2 q = rot(t*0.7*(h11(fi) - 0.5) + fi)*(b - vec2(h.x, y));
      float sz = (1.0 + 0.6*h11(fi*3.9))*(0.75 + 0.25*h.y);
      float rock = max(abs(q.x) + abs(q.y)*0.55, abs(q.y)*1.05) - sz + 0.08*vn(q*6.0 + fi);
      float crack = halo(abs(vn(q*4.0 + fi*3.0) - 0.5), 0.06)*step(rock, 0.0);
      vec3 rc = vec3(0.32, 0.26, 0.22)*(0.6 + 0.4*h.y) + vec3(1.0, 0.45, 0.1)*crack*1.6;
      light += rc*fillD(rock, px)*along + vec3(1.0, 0.5, 0.15)*halo(abs(rock), 0.05 + px)*0.6*along;
    }
    for (int i = 0; i < 8; i++){
      float fi = float(i);
      float f = fract(t*0.6 + h11(fi*5.3));
      float s = mod(fi, 2.0) < 0.5 ? -1.0 : 1.0;
      vec2 p = vec2(s*(0.7 + 0.8*f), (0.2 + 0.09*fi)*L - 6.0*f*f);
      light += vec3(1.0, 0.45, 0.08)*(halo(length((b - p)*vec2(1.0, 0.6)), 0.14)*2.0)*(1.0 - f);
    }
    float dust = fbm3(vec2(b.x*0.6 + t*0.2, b.y*0.25));
    light += vec3(0.7, 0.55, 0.4)*dust*dust*0.25*exp(-abs(b.x)/4.0)*along;
  }
  return light*(1.0 + 0.8*sw);
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  px = min(px, 0.06);
  float k = el_cur();
  // the aura of the ruler, the last one's fading as the new one takes over (from the hilt up)
  float take = smoothstep(0.0, 1.2, el_local());
  vec3 aura = el_aura(k, b, px)*take + el_aura(mod(k + 3.0, 4.0), b, px)*(1.0 - take);
  float edge = 1.0 - smoothstep(8.5, 10.0, abs(b.x));
  aura *= edge*(1.0 - smoothstep(L_EL() + 1.0, L_EL() + 4.0, b.y))*smoothstep(-2.5, -0.5, b.y);
  return elemental_glow0(g, b, r, px) + elemental_show(b, px) + (1.0 - exp(-aura*1.2))*1.1;
}

