// Star Dragon: a thin royal-blue blade with an eastern dragon made of stars coiled close round it
// from hilt to tip: a body of indigo night packed with star-scales that crawl along it, a gold
// starlit outline, a crest of star-tipped spines on the outside of each turn, four clawed legs and
// a flame of stars at the tail. Its horned, maned head rises over the tip; every eight seconds, and
// when you swing, it rears back, opens its jaws and breathes a plume of gold stardust.
// @room 8
// @palette {"corona.rim_cool":[0.06,0.10,0.45],"corona.rim_hot":[0.25,0.40,1.0],"corona.inner":[0.70,0.78,1.0],"core.white":[0.92,0.94,1.0],"core.fringe_cool":[0.25,0.35,1.0],"core.fringe_hot":[0.70,0.80,1.0],"light.color":[0.30,0.42,1.0],"trail":[0.35,0.45,1.0]}

const float SD_R = 1.45;
float sd_breath(){ float p = mod(uTime - 10.4, 8.0);
  return clamp(max(smoothstep(0.0, 0.5, p)*(1.0 - smoothstep(2.0, 2.8, p)), uSwing*0.9 + uClash*0.6), 0.0, 1.0); }
float sd_y0(){ return 1.2; }
float sd_yn(){ return uLength - 6.0; }
float sd_K(){ return 2.25*6.28318/(sd_yn() - sd_y0()); }
float sd_phi(){ return 0.55*sin(uTime*0.35); }
// the coil's angle at height y (chosen so the neck leaves the coil at its right-hand side)
float sd_ang(float y){ return sd_K()*(y - sd_yn()) + 1.5708 + sd_phi(); }
float sd_star(vec2 v, float s, float px){
  float d = length(v);
  return fillD(d - s*0.32, px) + 0.6*halo(d, s*0.8) + 0.45*(halo(abs(v.x), px + 0.012)*halo(abs(v.y), s*2.0) + halo(abs(v.y), px + 0.012)*halo(abs(v.x), s*2.0))*step(0.14, s);
}
// starry hide: indigo nebula, packed stars, star-scales in rows (uv: along, across -1..1)
vec3 sd_hide(vec2 uv, float px, float lit){
  vec2 sp = vec2(uv.x*2.6 - uTime*0.8, uv.y*2.2);
  sp.y += 0.5*mod(floor(sp.x), 2.0);
  vec2 sc = floor(sp); vec2 f = fract(sp) - 0.5;
  float scale = halo(abs(length(f*vec2(1.0, 1.3)) - 0.42), 0.06);
  float sstar = fillD(length(f) - 0.09, px*2.6)*step(0.35, h21(sc));
  vec3 neb = mix(vec3(0.08, 0.06, 0.38), vec3(0.22, 0.20, 0.70), 0.5 + 0.5*uv.y)*(0.7 + 0.3*vn(uv*3.0));
  return (neb*0.9 + vec3(0.55, 0.62, 1.0)*scale*0.35 + vec3(1.0, 0.92, 0.70)*sstar*(0.6 + 0.4*sin(uTime*4.0 + sc.x*2.3 + sc.y*5.1))*1.4)*lit;
}
// the coiled body: rgb, cover; z its depth there
vec4 sd_body(vec2 b, float px, out float z){
  z = -1.0;
  float y0 = sd_y0(), yn = sd_yn();
  if (b.y < y0 - 1.6 || b.y > yn + 0.6) return vec4(0.0);
  float a = sd_ang(b.y);
  float xc = SD_R*sin(a);
  float slope = SD_R*sd_K()*cos(a);
  float N = sqrt(1.0 + slope*slope);
  float s = clamp((b.y - y0)/(yn - y0), 0.0, 1.0);
  float w = mix(0.14, 0.44, smoothstep(0.0, 0.45, s));
  float dx = (b.x - xc)/N;
  z = cos(a);
  float lit = mix(0.40, 1.0, smoothstep(-0.35, 0.35, z));
  vec3 col = vec3(0.0); float cov = 0.0;
  float d = abs(dx) - w;
  if (b.y > y0){
    float inside = fillD(d, px);
    col += sd_hide(vec2(b.y*0.55, dx/w), px, lit)*inside;
    col += vec3(1.0, 0.82, 0.45)*halo(abs(d), 0.03 + px)*1.3*lit;
    cov = inside;
    // the crest: star-tipped spines on the outside of the turn
    float out1 = sin(a);
    float side = sign(out1);
    float sy = b.y*1.5; float si = floor(sy); float sf = fract(sy) - 0.5;
    float h = (0.32 + 0.12*sin(si*1.7))*smoothstep(0.25, 0.6, abs(out1))*smoothstep(0.08, 0.3, s);
    float e = dx*side - w;
    float spine = step(0.0, e)*step(e, h)*step(abs(sf + 0.25*(e/max(h, 0.01))), 0.22*(1.0 - e/max(h, 0.01)));
    col += vec3(0.45, 0.50, 1.0)*spine*0.9*lit;
    col += vec3(1.0, 0.9, 0.7)*sd_star(vec2(e - h, (sf + 0.25)*0.66), 0.13, px)*step(0.05, h)*lit;
  }
  // the tail: a flame of stars where the coil ends at the hilt
  vec2 tq = b - vec2(SD_R*sin(sd_ang(y0)), y0);
  float tf = halo(length(tq*vec2(1.6, 0.8)), 0.5)*step(tq.y, 0.0)*0.9;
  col += vec3(0.5, 0.55, 1.0)*tf*lit;
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    vec2 sp = vec2((fi - 1.5)*0.35 + 0.15*sin(uTime*3.0 + fi), -0.4 - 0.45*fi + 0.2*sin(uTime*2.0 + fi*2.0));
    col += vec3(1.0, 0.92, 0.75)*sd_star(tq - sp, 0.14 - fi*0.02, px)*lit;
  }
  // four legs, each hanging from the coil with three claw stars
  for (int l = 0; l < 4; l++){
    float fl = float(l);
    float yl = mix(y0, yn, l < 2 ? 0.22 + fl*0.08 : 0.70 + (fl - 2.0)*0.08);
    float al = sd_ang(yl);
    float zl = cos(al);
    vec2 root = vec2(SD_R*sin(al), yl);
    float ls = sin(al) >= 0.0 ? 1.0 : -1.0;
    float stride = 0.25*sin(uTime*1.8 + fl*1.6);
    vec2 knee = root + vec2(ls*(0.55 + stride*0.3), -0.55);
    vec2 foot = knee + vec2(ls*0.15, -0.55 + stride*0.2);
    float lg = min(sdSeg(b, root, knee) - 0.11, sdSeg(b, knee, foot) - 0.08);
    float llit = mix(0.40, 1.0, smoothstep(-0.35, 0.35, zl));
    col += vec3(0.2, 0.2, 0.6)*fillD(lg, px)*llit + vec3(1.0, 0.82, 0.45)*halo(abs(lg), 0.025 + px)*llit;
    col += vec3(1.0, 0.95, 0.85)*sd_star(b - knee, 0.16, px)*llit;
    for (int c = 0; c < 3; c++){
      float fc = float(c) - 1.0;
      vec2 cl = foot + vec2(ls*0.25 + fc*0.18, -0.22 + abs(fc)*0.08);
      col += vec3(1.0, 0.85, 0.55)*(halo(sdSeg(b, foot, cl), 0.02 + px)*0.9 + sd_star(b - cl, 0.10, px))*llit;
    }
    if (z < zl) cov = max(cov, fillD(lg, px)*step(0.0, zl));
  }
  return vec4(col, cov);
}
// the head in its own frame (u toward the snout, v up), about 2.8 long; open 0..1
vec4 sd_head(vec2 q, float px, float open){
  vec2 jq = q - vec2(0.15, -0.12); jq = rot(open*0.5)*jq;
  float skull = sdEll(q - vec2(0.15, 0.08), vec2(0.72, 0.42));
  float snout = sdSeg(q, vec2(0.4, 0.06), vec2(1.55, 0.02)) - mix(0.27, 0.16, clamp((q.x - 0.4)/1.15, 0.0, 1.0));
  float jaw = sdSeg(jq, vec2(0.0, -0.02), vec2(1.3, -0.08)) - 0.11;
  float brow = sdEll(q - vec2(0.42, 0.34), vec2(0.30, 0.10));
  float d = min(min(skull, snout), min(jaw, brow));
  // antlers sweeping back, each with a tine
  float sw = 0.05*sin(uTime*1.3);
  float h1 = min(sdSeg(q, vec2(-0.15, 0.40), vec2(-1.0, 1.05 + sw)), sdSeg(q, vec2(-1.0, 1.05 + sw), vec2(-1.55, 0.95 + sw)));
  h1 = min(h1, sdSeg(q, vec2(-0.65, 0.78), vec2(-0.55, 1.25)));
  float h2 = min(sdSeg(q, vec2(0.05, 0.42), vec2(-0.6, 1.15)), sdSeg(q, vec2(-0.6, 1.15), vec2(-0.95, 1.3)));
  float horns = min(h1 - 0.07, h2 - 0.055);
  // the mane: flowing strands behind the skull
  float mane = 9.0;
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float yy = 0.25 - fi*0.22;
    float u = clamp(-q.x - 0.4, 0.0, 1.6);
    float my = yy + 0.18*sin(u*3.5 - uTime*3.0 + fi) - u*0.15;
    mane = min(mane, abs(q.y - my) - 0.07*(1.0 - u/1.6)*step(-2.0, q.x)*step(q.x, -0.35) + step(q.x, -2.0) + step(-0.35, q.x));
  }
  // nebula fill, gold outline, stars
  vec2 sp = q*6.0; vec2 sc = floor(sp);
  float dust = fillD(length(fract(sp) - h22(sc)) - 0.12, px*6.0)*step(0.5, h21(sc));
  vec3 c = (vec3(0.14, 0.12, 0.55) + vec3(1.0, 0.92, 0.7)*dust*0.9)*fillD(d, px);
  c += vec3(1.0, 0.82, 0.45)*halo(abs(d), 0.03 + px)*1.5;
  c += vec3(0.75, 0.80, 1.0)*(fillD(horns, px)*0.7 + halo(abs(horns), 0.025 + px)*0.8);
  c += vec3(0.45, 0.50, 1.0)*fillD(mane, px)*0.8 + vec3(0.8, 0.85, 1.0)*halo(abs(mane), 0.02 + px)*0.5;
  // whiskers: two long barbels from the snout
  for (int i = 0; i < 2; i++){
    float fi = float(i);
    vec2 w0 = vec2(1.45, -0.02 - fi*0.1);
    vec2 w1 = w0 + vec2(-0.4, -0.55 - fi*0.2);
    vec2 w2 = w1 + vec2(-0.9, -0.15 + 0.2*sin(uTime*2.0 + fi));
    c += vec3(1.0, 0.85, 0.55)*halo(min(sdSeg(q, w0, w1), sdSeg(q, w1, w2)), 0.02 + px)*1.1;
  }
  // the eye, glowing, under the brow; teeth in the open mouth; star points
  float eye = sdEll(q - vec2(0.45, 0.20), vec2(0.15, 0.07));
  c += vec3(0.6, 1.0, 0.95)*(fillD(eye, px)*2.2 + halo(max(eye, 0.0), 0.08)*0.8);
  for (int t = 0; t < 4; t++){
    float ft = float(t);
    c += vec3(1.0)*fillD(length(q - vec2(0.6 + ft*0.25, -0.12)) - 0.035, px)*open;
    c += vec3(1.0)*fillD(length(jq - vec2(0.5 + ft*0.24, 0.04)) - 0.035, px)*open;
  }
  c += vec3(1.0, 0.97, 0.9)*(sd_star(q - vec2(1.62, 0.05), 0.24, px) + sd_star(q - vec2(-1.55, 0.95 + sw), 0.2, px) + sd_star(q - vec2(-0.95, 1.3), 0.18, px));
  return vec4(c, max(fillD(d, px), fillD(horns, px)));
}
vec4 sd_all(vec2 b, float px, out float z){
  vec4 body = sd_body(b, px, z);
  vec3 col = body.rgb; float cov = body.a*step(0.0, z);
  float br = sd_breath();
  float yn = sd_yn();
  float an = sd_ang(yn);
  vec2 n0 = vec2(SD_R*sin(an), yn);
  // the neck lifts off the coil and carries the head up over the tip
  vec2 hb = vec2(1.15 - 0.35*br, uLength - 1.2 + 0.8*br) + vec2(0.15*sin(uTime*0.7), 0.25*sin(uTime*0.9));
  vec2 nm = mix(n0, hb, 0.5) + vec2(0.55, -0.2);
  float dn = min(sdSeg(b, n0, nm), sdSeg(b, nm, hb));
  float nw = 0.40;
  float neck = fillD(dn - nw, px);
  vec2 nd = normalize(hb - nm);
  col = mix(col, sd_hide(vec2(b.y*0.55, (dot(b - nm, vec2(-nd.y, nd.x)))/nw), px, 1.0), neck);
  col += vec3(1.0, 0.82, 0.45)*halo(abs(dn - nw), 0.03 + px)*1.3;
  cov = max(cov, neck);
  // the head: looking up and out; it rears back and thrusts forward to breathe
  vec2 dir = normalize(mix(vec2(0.55, 1.0), vec2(1.0, 0.6), br));
  vec2 up = vec2(-dir.y, dir.x);
  vec2 hp = hb + dir*0.3;
  vec2 hq = vec2(dot(b - hp, dir), dot(b - hp, up));
  vec4 hd = sd_head(hq/1.05, px/1.05, br);
  col = mix(col, hd.rgb, hd.a) + hd.rgb*(1.0 - hd.a);
  cov = max(cov, hd.a);
  // the breath: a cone of gold stardust from the open jaws
  if (br > 0.01){
    vec2 mouth = hp + dir*1.75 - up*0.12;
    vec2 bd = normalize(dir - up*0.25);
    vec2 bn = vec2(-bd.y, bd.x);
    for (int p = 0; p < 22; p++){
      float fp = float(p); float age = fract(uTime*1.4 + h11(fp*3.1));
      vec2 at = mouth + bd*(0.2 + age*3.3) + bn*(h11(fp + 9.0) - 0.5)*age*2.6 + vec2(0.0, -age*age*0.5);
      float sz = 0.05 + 0.10*(1.0 - age)*h11(fp + 2.0);
      vec3 c = mix(vec3(1.0, 0.96, 0.8), vec3(1.0, 0.6, 0.25), age);
      col += c*(fillD(length(b - at) - sz, px) + halo(length(b - at), sz*3.0)*0.5)*(1.0 - age)*2.0*br;
    }
    col += vec3(1.0, 0.75, 0.4)*halo(length(b - mouth - bd*1.3), 1.1)*0.35*br;
  }
  return vec4(col, cov);
}
vec3 star_dragon_glow0(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  float z; vec4 s = sd_all(b, px, z);
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*4.0));
  vec3 blade = vec3(0.25, 0.38, 1.0)*(halo(bd, 0.85)*0.75 + halo(bd, 2.0)*0.22)*smoothstep(-0.3, 0.2, b.y);
  return blade*(1.0 - 0.7*s.a) + s.rgb + g*0.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float z; vec4 s = sd_all(b, px, z);
  return vec3(0.90, 0.92, 1.0)*(1.0 - smoothstep(0.08, 0.34, abs(b.x)))*step(0.0, b.y)*(1.0 - smoothstep(uLength - 0.6, uLength, b.y))*(1.0 - 0.9*s.a);
}

// ==== show (added) ====
// Star Dragon show: stardust breath blasts from the tip region in bursts of glittering mist, shooting
// stars streak past the blade, constellation lines flicker between nearby stars, and drifting blue-gold
// nebula clouds glow round it. A swing scatters stardust; a clash is a roar: a starburst ring.
vec3 star_dragon_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 blue = vec3(0.35, 0.5, 1.0), gold = vec3(1.0, 0.85, 0.45), white = vec3(0.95, 0.97, 1.0);
  vec3 light = vec3(0.0);
  float br = mod(uTime, 5.0);
  if (br < 1.6){
    for (int i = 0; i < 16; i++){
      float fi = float(i);
      float t = br - h11(fi*1.3)*0.5;
      if (t < 0.0) continue;
      float an = 0.6 + 0.9*(h11(fi*3.7) - 0.5);
      vec2 p = vec2(0.6, L*0.88) + vec2(cos(an), sin(an)*0.6)*t*(4.0 + 3.0*h11(fi*5.1));
      float k = 1.0 - t/1.6;
      light += mix(white, gold, h11(fi))*(halo(length(b - p), 0.07 + 0.1*t)*1.8 + halo(length(b - p), 0.6*t + 0.1)*0.2)*k;
    }
  }
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float ph = uTime*0.35 + h11(fi*7.3);
    float cy = floor(ph), f = fract(ph);
    vec2 a0 = vec2((h11(cy + fi) - 0.5)*16.0, (0.4 + 0.6*h11(cy*2.0 + fi))*L + 4.0);
    vec2 dir = normalize(vec2(-sign(a0.x)*0.8 + 0.2, -1.0));
    vec2 p = a0 + dir*f*18.0;
    float tr = sdSeg(b, p - dir*3.0, p);
    float along = clamp(dot(b - p + dir*3.0, dir)/3.0, 0.0, 1.0);
    light += white*(halo(tr, 0.04)*along*1.6 + halo(length(b - p), 0.12)*1.5)*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.8, 1.0, f));
  }
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    vec2 s0 = vec2((h11(fi*3.1) - 0.5)*13.0, (0.08 + 0.9*h11(fi*6.7))*L);
    s0.x += sign(s0.x)*1.3;
    vec2 s1 = vec2((h11(fi*3.1 + 1.0) - 0.5)*13.0, (0.08 + 0.9*h11(fi*6.7 + 1.0))*L);
    s1.x += sign(s1.x)*1.3;
    float tw = 0.5 + 0.5*sin(uTime*3.0 + fi*2.0);
    light += white*(halo(length(b - s0), 0.07)*1.5 + halo(length(b - s0), 0.3)*0.2)*(0.5 + 0.5*tw);
    float link = smoothstep(0.3, 0.8, sin(uTime*0.6 + fi*1.3));
    if (length(s0 - s1) < 9.0) light += blue*halo(sdSeg(b, s0, s1), 0.02 + px)*link*0.7;
  }
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    vec2 c = vec2((mod(fi, 2.0) < 0.5 ? -1.0 : 1.0)*(4.0 + sin(uTime*0.2 + fi)), (0.2 + 0.3*fi)*L);
    float n = fbm(b*0.35 + vec2(uTime*0.05, fi*7.0));
    light += mix(blue, gold, n)*smoothstep(0.45, 0.75, n)*exp(-length(b - c)/2.5)*0.4;
  }
  if (sw > 0.05){
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      vec2 p = vec2((1.0 + 5.0*h11(fi*2.2))*sign(h11(fi) - 0.5), (0.1 + 0.85*h11(fi*5.5))*L);
      light += gold*halo(length(b - p), 0.08)*sw*1.5;
    }
  }
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.6);
    float a = 1.0 - cl;
    light += white*halo(abs(length(b - o) - 1.0 - 7.0*a), 0.12)*cl*1.5;
    for (int i = 0; i < 12; i++){ float fi = float(i); float an = fi*0.5236;
      light += gold*halo(length(b - o - vec2(cos(an), sin(an))*(1.0 + 6.5*(1.0 - exp(-3.0*a)))), 0.1)*cl*1.8; }
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return star_dragon_glow0(g, b, r, px) + star_dragon_show(b, min(px, 0.06));
}
