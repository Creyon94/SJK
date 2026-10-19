// World Tree: Yggdrasil as a blade. Three luminous gold strands braid into the trunk, roots grip the
// emitter, short boughs fork off both sides and hold spinning galaxies for leaves, and a crown of
// galaxies sits over the tip. On ignition the tree grows from the hilt and its galaxies bloom; sap
// light climbs the trunk and lights each galaxy in turn, and now and then a galaxy-leaf falls away
// and a new one buds. Swings shake the boughs, clashes make every galaxy flare.
// @room 8
// @palette {"corona.rim_cool":[0.10,0.06,0.30],"corona.rim_hot":[0.55,0.40,0.95],"corona.inner":[1.0,0.90,0.62],"core.white":[1.0,0.95,0.80],"core.fringe_cool":[0.60,0.45,1.0],"core.fringe_hot":[1.0,0.88,0.60],"light.color":[0.95,0.80,0.50],"trail":[0.90,0.75,0.50]}

float wt_grow(){ return clamp(uAge/3.2, 0.0, 1.0); }
float wt_top(){ float g = wt_grow(); return (uLength + 2.5)*(1.0 - (1.0 - g)*(1.0 - g)); }
vec2 wt_seg(vec2 p, vec2 a, vec2 b){ vec2 pa = p - a, ba = b - a; float h = clamp(dot(pa, ba)/dot(ba, ba), 0.0, 1.0); return vec2(length(pa - ba*h), h); }
vec3 wt_pal(float i){
  float k = mod(i, 4.0);
  return k < 1.0 ? vec3(0.70, 0.50, 1.0) : k < 2.0 ? vec3(0.40, 0.92, 1.0) : k < 3.0 ? vec3(1.0, 0.52, 0.82) : vec3(1.0, 0.86, 0.50);
}
// a spiral galaxy seen at a tilt: two arms, a bright bulge
vec3 wt_gal(vec2 q, float rad, float spin, float tilt, vec3 col, float px){
  if (rad < 0.01) return vec3(0.0);
  q = rot(tilt)*q; q.y /= 0.5;
  float d = length(q)/rad;
  if (d > 2.2) return vec3(0.0);
  float ang = atan(q.y, q.x);
  float arms = pow(0.5 + 0.5*cos(2.0*ang - 4.5*log(d + 0.05) - spin), 2.5);
  float dust = 0.6 + 0.4*vn(q*9.0/rad);
  return col*(arms*dust*smoothstep(1.0, 0.15, d)*1.8 + halo(d, 1.1)*0.55) + vec3(1.0, 0.97, 0.9)*halo(d, 0.16)*2.4;
}
// the sap pulse: height of the climbing light, and its strength
float wt_sapY(){ return mod(uTime, 6.0)*13.0 - 2.0; }
vec3 world_tree_glow0(vec3 g, vec2 b, float r, float px){
  float L = uLength, t = uTime, top = wt_top(), gr = wt_grow();
  vec3 col = vec3(0.0);
  float sapY = wt_sapY();
  float shake = uSwing*0.5 + uClash*0.3;
  // ---- the trunk: three braided strands, thick at the root, thinning up the blade
  float h = clamp(b.y/L, 0.0, 1.0);
  float alive = smoothstep(top, top - 1.0, b.y)*smoothstep(-0.6, 0.0, b.y);
  float trunk = 0.0, groove = 0.0;
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    float ph = b.y*0.30 + fi*2.094 + 0.25*sin(t*0.5);
    float A = mix(0.45, 0.22, h) + 0.35*smoothstep(3.0, 0.0, b.y);
    float x = A*sin(ph);
    float z = cos(ph);
    float w = mix(0.30, 0.15, h) + 0.15*smoothstep(3.0, 0.0, b.y);
    float d = abs(b.x - x);
    float s = fillD(d - w, px)*mix(0.55, 1.0, smoothstep(-0.5, 0.5, z));
    float bark = 0.75 + 0.25*sin(b.y*5.0 + fi*2.0 + 3.0*vn(vec2(b.y*1.3, fi*7.0)));
    trunk = max(trunk, s*bark);
    groove = max(groove, halo(abs(d - w), 0.05 + px)*step(0.0, z));
  }
  float sapT = halo(abs(b.y - sapY), 1.4)*(1.0 - smoothstep(0.9, 1.6, abs(b.x)));
  vec3 wood = mix(vec3(0.85, 0.50, 0.18), vec3(1.0, 0.85, 0.55), trunk*trunk);
  col += (wood*trunk*0.95 + vec3(1.0, 0.85, 0.55)*groove*0.25 + vec3(1.0, 0.95, 0.75)*sapT*1.4*trunk)*alive;
  // stars inside the bark
  vec2 sp = b*vec2(5.0, 2.0); vec2 sc = floor(sp);
  col += vec3(0.85, 0.85, 1.0)*fillD(length(fract(sp) - h22(sc)) - 0.08, px*5.0)*step(0.78, h21(sc))*trunk*alive*(0.6 + 0.4*sin(t*3.0 + sc.x*5.0 + sc.y));
  // ---- the roots: four gnarled roots gripping the emitter
  float rgrow = smoothstep(0.0, 0.25, gr);
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float s = i < 2 ? -1.0 : 1.0;
    float o = mod(fi, 2.0);
    vec2 a = vec2(s*0.35, 2.2 + o*0.6);
    vec2 m = vec2(s*(1.0 + 0.4*o), 0.9 + o*0.4);
    vec2 e = vec2(s*(1.55 + 0.45*o), -0.25 + o*0.35);
    e = mix(a, e, rgrow); m = mix(a, m, rgrow);
    vec2 d1 = wt_seg(b, a, m), d2 = wt_seg(b, m, e);
    float w1 = mix(0.22, 0.14, d1.y), w2 = mix(0.14, 0.04, d2.y);
    float rr = max(fillD(d1.x - w1, px), fillD(d2.x - w2, px))*step(0.02, rgrow);
    col += vec3(0.95, 0.72, 0.40)*rr*1.0 + vec3(1.0, 0.8, 0.5)*halo(min(d1.x, d2.x), 0.35)*0.18*rgrow;
  }
  // ---- the boughs: nine, alternating sides, each forking to two galaxy-leaves
  float fall = mod(t, 9.0);                         // the falling leaf's clock
  float fallK = mod(floor(t/9.0)*4.0, 9.0);         // which bough loses its leaf
  vec3 leaves = vec3(0.0);
  for (int k = 0; k < 9; k++){
    float fk = float(k);
    float yb = 6.5 + fk*3.55;
    if (abs(b.y - yb - 2.0) > 6.5) continue;
    float s = mod(fk, 2.0) < 1.0 ? 1.0 : -1.0;
    float Lk = 2.75 - fk*0.08;
    float sprout = smoothstep(yb + 1.0, yb + 4.0, top);
    float sw = sin(t*0.9 + fk*1.3)*0.10 + shake*sin(t*9.0 + fk)*0.35;
    vec2 P0 = vec2(s*0.15, yb);
    vec2 P1 = P0 + vec2(s*Lk*0.55, Lk*0.12)*sprout;
    vec2 P2 = P0 + (vec2(s*Lk*0.88, Lk*0.70) + vec2(0.0, sw))*sprout;
    vec2 M = 0.25*P0 + 0.5*P1 + 0.25*P2;
    vec2 T = M + (vec2(s*0.25, Lk*0.62) + vec2(sw*0.6, 0.0))*sprout;
    vec2 d1 = wt_seg(b, P0, M), d2 = wt_seg(b, M, P2), d3 = wt_seg(b, M, T);
    float bw = max(fillD(d1.x - mix(0.16, 0.10, d1.y), px), max(fillD(d2.x - mix(0.10, 0.05, d2.y), px), fillD(d3.x - mix(0.08, 0.04, d3.y), px)));
    // sap reaching the bough lights it, then its galaxies
    float sapB = halo(abs(sapY - yb - 1.5), 1.2);
    bw *= step(0.02, sprout);
    col += mix(vec3(0.95, 0.66, 0.30), vec3(1.0, 0.95, 0.8), sapB)*bw*(0.9 + sapB);
    float bloom = smoothstep(yb + 2.0, yb + 7.0, top);
    float flare = 1.0 + 1.2*sapB + 1.5*uClash;
    // the main leaf may be the falling one
    float r1 = 0.85*bloom, r2 = 0.60*bloom;
    vec2 g1 = P2 + vec2(s*0.15, 0.25);
    if (fk == fallK){
      if (fall < 4.5){
        float f = fall/4.5;
        g1 += vec2(s*(0.6*f + 0.35*sin(f*9.0)), -9.0*f*f - 1.0*f);
        r1 *= 1.0 - 0.6*f; flare *= 1.0 - f;
      } else r1 *= smoothstep(5.0, 8.5, fall);
    }
    leaves += wt_gal(b - g1, r1, t*1.2 + fk, fk*0.9, wt_pal(fk), px)*flare;
    leaves += wt_gal(b - T - vec2(0.0, 0.3), r2, t*1.5 + fk*2.0, fk*1.7 + 1.0, wt_pal(fk + 1.0), px)*flare;
  }
  // ---- the crown over the tip: three boughs, five galaxies
  float crown = smoothstep(L - 2.0, L + 2.0, top);
  vec2 cb = vec2(0.0, L - 4.0);
  for (int i = 0; i < 3; i++){
    float fi = float(i) - 1.0;
    vec2 e = cb + vec2(fi*1.9, 3.6 + (1.0 - abs(fi))*1.6)*crown + vec2(shake*0.3*sin(t*8.0 + fi), 0.0);
    vec2 d = wt_seg(b, cb, e);
    col += vec3(0.95, 0.75, 0.45)*fillD(d.x - mix(0.14, 0.05, d.y), px)*step(0.01, crown);
    leaves += wt_gal(b - e - vec2(fi*0.3, 0.5), 1.05*crown, t*0.9 + fi, fi*0.8, wt_pal(fi + 6.0), px)*(1.0 + 1.5*uClash + halo(abs(sapY - L), 1.5)*1.5);
  }
  leaves += wt_gal(b - vec2(-1.5, L + 3.6), 0.7*crown, t*1.3, 2.0, wt_pal(1.0), px) + wt_gal(b - vec2(1.6, L + 3.5), 0.7*crown, t*1.1, 0.5, wt_pal(3.0), px)
         + wt_gal(b - vec2(-2.6, L + 1.0), 0.6*crown, t*1.2, 1.0, wt_pal(2.0), px) + wt_gal(b - vec2(2.6, L + 1.2), 0.6*crown, t*1.4, 2.5, wt_pal(0.0), px);
  col += leaves;
  // ---- the night around it: a deep violet body light with drifting star motes
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*3.0));
  vec3 sky = vec3(0.45, 0.25, 0.9)*halo(bd, 1.2)*0.45 + vec3(0.2, 0.1, 0.5)*halo(bd, 2.4)*0.35 + vec3(1.0, 0.75, 0.4)*halo(abs(b.x), 0.6)*0.12;
  sky *= smoothstep(top + 1.0, top - 2.0, b.y)*smoothstep(-0.8, 0.0, b.y);
  // clash: stardust shaken loose drifts down
  vec3 dust = vec3(0.0);
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    float age = fract(t*0.25 + h11(fi*1.7));
    vec2 p = vec2((h11(fi + 3.1) - 0.5)*5.0 + 0.4*sin(t + fi), L*(0.25 + 0.75*h11(fi + 8.0)) - age*8.0);
    dust += vec3(1.0, 0.9, 0.7)*fillD(length(b - p) - 0.06, px)*(1.0 - age)*(0.35 + 1.5*uClash + uSwing);
  }
  return sky + col + dust*step(0.99, gr) + g*0.06;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float alive = smoothstep(wt_top(), wt_top() - 1.0, b.y);
  return vec3(1.0, 0.92, 0.75)*(1.0 - smoothstep(0.05, 0.25, abs(b.x)))*alive*0.6*step(0.0, b.y)*(1.0 - smoothstep(uLength - 4.0, uLength, b.y));
}

// ==== show (added) ====
// World Tree show: glowing leaves falling and drifting off, fireflies rising, tiny galaxies shed like
// seeds spinning away, golden vines curling off the trunk; a swing raises a storm of leaves, a clash
// bursts leaves out in a golden ring.
vec3 world_tree_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 light = vec3(0.0);
  vec3 gold = vec3(1.0, 0.82, 0.45);
  // falling leaves: little two-arc leaves, swaying, turning, glowing in cosmic colours
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float ph = uTime*(0.18 + 0.08*h11(fi*2.3)) + h11(fi*7.9);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.1 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.25 + 0.7*h11(cy*1.7 + fi*4.0))*L;
    vec2 p = vec2(s*(0.8 + 5.5*f + 0.8*sin(f*7.0 + fi)), y0 - 6.0*f + 1.5*sin(f*5.0 + fi*2.0));
    vec2 q = b - p;
    float an = uTime*(1.5 + h11(fi)) + fi;
    q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
    float flip = 0.35 + 0.65*abs(sin(uTime*3.0 + fi));
    q.x /= flip;
    float lf = max(length(q - vec2(0.18, 0.0)), length(q + vec2(0.18, 0.0))) - 0.36;   // a vesica leaf
    vec3 lc = hsv(fract(0.12 + 0.55*h11(cy + fi*3.0)), 0.6, 1.0);
    float fade = smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.75, 1.0, f));
    float vein = halo(abs(q.y), 0.02)*step(abs(q.x), 0.4);
    light += (lc*clamp(-lf*12.0, 0.0, 1.0)*0.9 + vec3(1.0)*vein*0.4*step(lf, 0.0) + lc*halo(max(lf, 0.0), 0.2)*0.4)*fade;
  }
  // fireflies rising round the tree, blinking
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float f = fract(uTime*0.07 + h11(fi*5.3));
    vec2 p = vec2((h11(fi*3.7) - 0.5)*12.0 + 0.8*sin(uTime*0.9 + fi), f*(L + 4.0) - 1.0);
    p.x += sign(p.x)*1.2;
    float bl = smoothstep(0.6, 1.0, sin(uTime*2.0 + fi*2.7));
    light += vec3(0.85, 1.0, 0.55)*(halo(length(b - p), 0.08)*2.0 + halo(length(b - p), 0.4)*0.25)*bl;
  }
  // galaxy seeds: tiny spirals shed off the crown, drifting out and spinning
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    float ph = uTime*(0.12 + 0.05*h11(fi)) + h11(fi*8.8);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*2.2 + fi) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(s*(1.5 + 5.5*f), (0.55 + 0.4*h11(cy*3.3 + fi))*L + 2.0*f);
    vec2 q = b - p;
    float r2 = length(q), a2 = atan(q.y, q.x);
    float arms = 0.5 + 0.5*cos(2.0*(a2 - log(max(r2, 0.02))*3.0 - uTime*2.0));
    float rad = 0.9*(0.5 + 0.5*smoothstep(0.0, 0.3, f));
    float fade = smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.7, 1.0, f));
    vec3 gc = hsv(fract(0.55 + 0.3*h11(cy + fi)), 0.5, 1.0);
    light += (gc*arms*smoothstep(rad, rad*0.2, r2)*0.9 + vec3(1.0, 0.95, 0.85)*halo(r2, 0.1)*1.4)*fade;
  }
  // golden vines curling off the trunk and fading
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float ph = uTime*(0.3 + 0.1*h11(fi*4.4)) + h11(fi*6.6);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*5.5 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.1 + 0.75*h11(cy*2.9 + fi))*L;
    float len = (2.5 + 2.5*h11(cy + fi))*smoothstep(0.0, 0.5, f);
    float u = (b.x*s - 0.5)/max(len, 0.01);
    if (u < 0.0 || u > 1.0) continue;
    float curl = 0.9*u*sin(u*6.0 + cy) + 1.2*u*u;
    float dy = b.y - y0 - curl;
    float fade = (1.0 - f)*smoothstep(0.0, 0.08, f)*(1.0 - smoothstep(0.85, 1.0, u));
    light += gold*(exp(-pow(dy/0.05, 2.0))*1.3 + halo(abs(dy), 0.25)*0.15)*fade;
    // a bud of light at the vine's growing end
    light += gold*halo(length(vec2(b.x*s - 0.5 - len, dy + curl - (0.9*sin(6.0 + cy) + 1.2))), 0.12)*fade*step(0.95, u);
  }
  // clash: leaves burst out in a golden ring
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 o = vec2(0.0, L*0.6);
    light += gold*halo(abs(length(b - o) - 1.0 - 6.5*a), 0.12)*cl*1.4;
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float an = fi*0.628;
      vec2 p = o + vec2(cos(an), sin(an))*(1.0 + 6.0*(1.0 - exp(-3.0*a)));
      light += hsv(fi*0.1, 0.6, 1.0)*halo(length(b - p), 0.2)*cl*1.5;
    }
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return world_tree_glow0(g, b, r, px) + world_tree_show(b, min(px, 0.06));
}
