// Aurora Wyrm: a thin pale-green blade, a strip of night sky with a few stars, and coiled close round
// it a serpent made of northern lights: a ribbon body with a bright green hem, curtains of rippling
// rays standing up off its back and fading to magenta, a horned head with white eyes and magenta
// streamers. Every twelve seconds it uncoils: its upper body peels off the blade and the head weaves
// up into the sky above the tip in slow S-curves, then winds back down. Swings make the rays flare.
// @room 8
// @palette {"corona.rim_cool":[0.05,0.12,0.40],"corona.rim_hot":[0.20,0.45,0.95],"corona.inner":[0.70,0.85,1.0],"core.white":[0.90,1.0,0.95],"core.fringe_cool":[0.25,0.75,0.70],"core.fringe_hot":[0.75,1.0,0.88],"light.color":[0.25,0.90,0.60],"trail":[0.30,0.95,0.65]}

const int AW_N = 30;
const float AW_R = 1.25;
float aw_rise(){ float p = mod(uTime - 9.0, 12.0); return smoothstep(0.0, 2.2, p)*(1.0 - smoothstep(6.5, 9.0, p)); }
// the body's path at s (0 tail .. 1 head): xy on screen, z depth
vec3 aw_path(float s){
  float L = uLength, r = aw_rise();
  float y = 1.0 + s*(L - 3.5);
  float a = y*0.40 - uTime*0.5;
  vec3 coil = vec3(AW_R*sin(a), y, cos(a));
  // in the sky: from where it leaves the coil, the rest climbs over the tip in S-curves
  float s0 = 1.0 - 0.45*r;
  float u = max(s - s0, 0.0)/max(1.0 - s0, 0.01);
  float y0 = 1.0 + s0*(L - 3.5);
  float ys = mix(y0, L + 4.2, u);
  float xs = 1.9*sin(u*5.0 - uTime*1.2)*smoothstep(0.0, 0.3, u)*(1.0 - 0.3*u);
  vec3 sky = vec3(mix(coil.x, xs, smoothstep(0.0, 0.25, u)), ys, 1.0);
  float w = smoothstep(s0 - 0.02, s0 + 0.06, s)*step(0.01, r);
  return mix(coil, sky, w);
}
// the wyrm at b: rgb light, a cover of the parts in front of the blade
vec4 aw_all(vec2 b, float px){
  float L = uLength;
  if (abs(b.x) > 4.2 || b.y < -1.0 || b.y > L + 7.5) return vec4(0.0);
  float flare = clamp(uSwing*1.4 + uClash, 0.0, 1.0);
  vec3 prev = aw_path(0.0);
  float bestD = 99.0, bestS = 0.0, bestZ = 0.0, bestV = 0.0;
  vec3 col = vec3(0.0); float cov = 0.0;
  for (int k = 1; k < AW_N; k++){
    float s = float(k)/float(AW_N - 1);
    vec3 p = aw_path(s);
    vec2 ab = p.xy - prev.xy, ap = b - prev.xy;
    float h = clamp(dot(ap, ab)/max(dot(ab, ab), 1e-5), 0.0, 1.0);
    vec2 dv = ap - ab*h;
    float d = length(dv);
    // the curtain stands on the upper side of the body
    vec2 nrm = normalize(vec2(-ab.y, ab.x) + vec2(0.0, 1e-4)); if (nrm.y < 0.0) nrm = -nrm;
    float v = dot(dv, nrm);
    float ss = mix(s - 1.0/float(AW_N - 1), s, h);
    float z = mix(prev.z, p.z, h);
    float w = mix(0.06, 0.24, smoothstep(0.0, 0.35, ss))*(1.0 - 0.4*smoothstep(0.9, 1.0, ss));
    float lit = mix(0.30, 1.0, smoothstep(-0.3, 0.3, z));
    // body: green hem with a white-green line along it
    float body = halo(d, w);
    col += (vec3(0.15, 1.0, 0.50)*body*0.9 + vec3(0.8, 1.0, 0.9)*halo(d, w*0.3 + px)*0.8)*lit*0.55;
    // curtain rays rising off its back, green to magenta, rippling along it
    float H = (0.55 + 0.9*smoothstep(0.1, 0.5, ss))*(1.0 + 0.6*flare);
    if (v > 0.0 && v < H*1.4 && h > 0.0 && h < 1.0){
      float uu = v/H;
      float rays = 0.25 + 0.75*vn(vec2(ss*90.0 - uTime*1.5, 0.5))*(0.5 + 0.5*vn(vec2(ss*20.0 + uTime*0.8, 3.0)));
      vec3 rc = mix(vec3(0.1, 1.0, 0.45), vec3(0.9, 0.25, 0.85), smoothstep(0.2, 1.0, uu));
      col += rc*rays*(1.0 - smoothstep(0.3, 1.4, uu))*1.1*lit*smoothstep(0.02, 0.1, ss)*0.5;
    }
    if (d < bestD){ bestD = d; bestS = ss; bestZ = z; }
    prev = p;
  }
  cov = halo(bestD, 0.25)*step(0.0, bestZ);
  // the head at the end of the path
  vec3 h0 = aw_path(1.0), h1 = aw_path(0.96);
  vec2 dir = normalize(h0.xy - h1.xy + vec2(0.0, 1e-4));
  vec2 up = vec2(-dir.y, dir.x); if (up.y < 0.0) up = -up;
  vec2 hp = h0.xy + dir*0.35;
  vec2 q = vec2(dot(b - hp, dir), dot(b - hp, up));
  float hl = mix(0.5, 1.0, smoothstep(-0.3, 0.3, h0.z));
  float skull = sdEll(q - vec2(0.3, 0.0), vec2(0.95, 0.36 - 0.14*clamp(q.x, 0.0, 1.2)));
  float horns = min(sdSeg(q, vec2(-0.2, 0.22), vec2(-1.2, 0.75 + 0.08*sin(uTime*2.0))), sdSeg(q, vec2(-0.1, -0.20), vec2(-1.0, -0.62))) - 0.05*clamp(1.0 + q.x, 0.25, 1.0);
  col += (vec3(0.25, 1.0, 0.55)*fillD(skull, px)*0.7 + vec3(0.75, 1.0, 0.85)*halo(abs(skull), 0.05 + px)*1.4)*hl;
  col += vec3(0.95, 0.40, 1.0)*(fillD(horns, px)*1.1 + halo(max(horns, 0.0), 0.10)*0.5)*hl;
  col += vec3(1.0)*fillD(sdEll(vec2(q.x - 0.45, abs(q.y) - 0.13), vec2(0.13, 0.06)), px)*2.5*hl;
  for (int i = 0; i < 2; i++){
    float sd = i == 0 ? -1.0 : 1.0;
    float t = clamp((0.9 - q.x)/2.6, 0.0, 1.0);
    float sy = sd*(0.10 + 0.35*t) + 0.25*sin(uTime*3.5 + q.x*2.2 + sd);
    float st = halo(abs(q.y - sy), 0.04 + 0.04*t)*step(q.x, 1.1)*step(-1.6, q.x)*(1.0 - t);
    col += vec3(0.95, 0.35, 0.85)*st*1.2*hl;
  }
  cov = max(cov, fillD(skull, px)*0.9);
  return vec4(col, cov);
}
vec3 aurora_wyrm_glow0(vec3 g, vec2 b, float r, float px){
  vec4 a = aw_all(b, px);
  float L = uLength;
  float bd = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*4.0));
  vec2 gp = b*vec2(2.5, 1.5); vec2 cell = floor(gp);
  float st = fillD(length(fract(gp) - h22(cell)) - 0.05, px*2.5)*step(0.85, h21(cell))*(1.0 - smoothstep(0.5, 1.5, abs(b.x)))*step(0.0, b.y)*step(b.y, L);
  vec3 sky = vec3(0.12, 0.30, 0.80)*halo(bd, 0.7)*0.65 + vec3(0.06, 0.15, 0.45)*halo(bd, 1.8)*0.35 + st*vec3(0.8, 0.9, 1.0)*0.7;
  return sky*(1.0 - 0.5*a.a) + a.rgb + g*0.04;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  vec4 a = aw_all(b, px);
  return vec3(0.85, 1.0, 0.92)*(1.0 - smoothstep(0.08, 0.32, abs(b.x)))*step(0.0, b.y)*(1.0 - smoothstep(uLength - 0.6, uLength, b.y))*(1.0 - 0.8*a.a);
}

// ==== show (added) ====
// Aurora Wyrm show: curtains of aurora ripple up off both sides of the blade, green at the hem and
// magenta at the top, with rays shimmering through them; diamond-dust ice crystals sparkle in the
// cold round it and a frost breath puffs from the wyrm's side. A swing throws the curtains out wide;
// a clash ripples a bright green wave through them.
vec3 aurora_wyrm_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 grn = vec3(0.25, 1.0, 0.55), mag = vec3(0.95, 0.3, 0.75);
  vec3 light = vec3(0.0);
  for (int s = 0; s < 2; s++){
    float sd = s == 0 ? 1.0 : -1.0;
    float x = b.x*sd;
    float hem = 1.6 + 0.9*sin(b.y*0.18 + uTime*0.6 + sd) + 0.5*sin(b.y*0.5 - uTime*1.1) + 2.5*sw;
    float h = 3.2 + 1.4*sin(b.y*0.11 - uTime*0.4 + sd*2.0);
    float u = (x - hem)/h;
    if (u > -0.15 && u < 1.0){
      float rays = 0.45 + 0.55*pow(vn(vec2(b.y*1.6 + sd*20.0, uTime*0.8)), 2.0);
      float lo = smoothstep(-0.15, 0.0, u)*exp(-u*2.5);
      float span = smoothstep(1.0, 5.0, b.y)*(1.0 - smoothstep(L - 4.0, L, b.y));
      vec3 c = mix(grn, mag, smoothstep(0.1, 0.8, u));
      float wave = cl*exp(-pow((b.y - mod((1.0 - cl)*L*1.5, L + 6.0))/2.0, 2.0));
      light += c*lo*rays*span*(0.7 + 0.5*sw + 2.0*wave);
      light += grn*halo(abs(u), 0.02)*span*rays*0.9;
    }
  }
  for (int i = 0; i < 14; i++){
    float fi = float(i);
    float f = fract(uTime*0.08 + h11(fi*4.3));
    vec2 p = vec2((h11(fi*2.9) - 0.5)*14.0, L*(1.0 - f) + 1.0*sin(uTime*0.5 + fi));
    p.x += sign(p.x)*1.0;
    float tw = pow(0.5 + 0.5*sin(uTime*(4.0 + 3.0*h11(fi)) + fi*5.0), 6.0);
    float cr = halo(abs(b.x - p.x), 0.02)*halo(abs(b.y - p.y), 0.25) + halo(abs(b.y - p.y), 0.02)*halo(abs(b.x - p.x), 0.25);
    light += vec3(0.85, 1.0, 0.95)*(halo(length(b - p), 0.05)*1.2 + cr*tw*1.4)*(0.3 + 0.7*tw);
  }
  for (int i = 0; i < 5; i++){
    float fi = float(i);
    float f = fract(uTime*0.4 + fi/5.0);
    vec2 p = vec2(1.0 + 4.0*f, L*0.7 + 1.5*f + 0.6*sin(f*6.0 + fi));
    light += vec3(0.7, 0.95, 1.0)*halo(length(b - p), 0.3 + 0.9*f)*0.35*(1.0 - f)*step(0.5, fract(uTime*0.1));
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return aurora_wyrm_glow0(g, b, r, px) + aurora_wyrm_show(b, min(px, 0.06));
}
