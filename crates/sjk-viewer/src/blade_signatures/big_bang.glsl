// Big Bang: a thin white-gold blade holding a young universe. Each ignition starts as one blinding
// point at the hilt that bursts: a fireball races up the blade, cools from white through gold to
// ember red, and condenses into a glowing cosmic web strung with tiny spinning galaxies that keep
// streaming up and away, growing and reddening as space expands. A clash sets off a small new bang.
// @room 8
// @palette {"corona.rim_cool":[0.20,0.06,0.30],"corona.rim_hot":[0.95,0.55,0.30],"corona.inner":[1.0,0.88,0.70],"core.white":[1.0,0.97,0.90],"core.fringe_cool":[0.85,0.40,0.55],"core.fringe_hot":[1.0,0.85,0.60],"light.color":[1.0,0.72,0.45],"trail":[1.0,0.66,0.40]}

// The lab took 3.5 s to light the universe; a blade must be lit as fast as a stock one.
float bb_age(){ return uAge*4.0; }
const float BB_K = 40.0, BB_C = 8.0, BB_O = 0.9;      // log map scale, offset, origin height
// when the fireball front passed height y (seconds of uAge)
float bb_pass(float y){ return 0.6 + 1.5*pow(max(y - BB_O, 0.0)/44.0, 1.4); }
vec3 bb_heat(float T){
  vec3 a = mix(vec3(0.30, 0.02, 0.10), vec3(1.0, 0.30, 0.06), smoothstep(0.0, 0.35, T));
  a = mix(a, vec3(1.0, 0.78, 0.35), smoothstep(0.3, 0.65, T));
  return mix(a, vec3(1.0, 0.98, 0.95), smoothstep(0.65, 1.0, T));
}
// the expanding universe: space flows up from the origin (log map), features grow with height
vec4 bb_cosmos(vec2 b, float px){
  float yc = max(b.y - BB_O, -0.5) + BB_C;
  float flow = uTime*(1.1);
  vec2 P = vec2(BB_K*b.x/yc, BB_K*log(yc) - flow)/3.2;          // web cells of 3.2 P units
  float ppx = px*BB_K/yc/3.2;                                    // a pixel in cell units
  vec2 cell; float e = voro(P, cell);
  float wfil = max(0.05, ppx*1.2);
  float fil = halo(e, wfil)*(0.05/wfil)*(0.55 + 0.45*vn(P*3.0 + 4.0));
  float knot = halo(e, wfil*3.0)*smoothstep(0.55, 0.85, vn(P*1.7));
  // a galaxy in this cell: a tilted two-armed spiral spinning slowly
  vec2 cc = cell + h22(cell);
  float gh = h21(cell + 3.7);
  vec2 lq = P - cc;
  lq = rot(gh*6.28)*lq; lq.y /= mix(0.35, 0.9, h21(cell + 8.1));
  float d = length(lq)/0.24;
  float ang = atan(lq.y, lq.x);
  float arms = pow(0.5 + 0.5*cos(2.0*ang - 5.0*log(d + 0.06) + uTime*(0.6 + gh)), 3.0);
  float gal = (arms*smoothstep(1.0, 0.25, d)*0.9 + halo(d, 0.18)*2.2 + halo(d, 0.6)*0.25)*step(0.35, gh);
  gal *= mix(0.25, 1.0, smoothstep(1.5, 4.0, 0.24/max(ppx, 1e-4)));   // dimmer dots when tiny
  vec3 gcol = mix(vec3(0.55, 0.75, 1.0), vec3(1.0, 0.85, 0.6), h21(cell + 1.3));
  gcol = mix(gcol, vec3(1.0, 0.55, 0.8), step(0.8, h21(cell + 5.5)));
  return vec4(vec3(0.85, 0.45, 1.0)*fil*0.9 + vec3(1.0, 0.8, 0.55)*knot*0.5 + gcol*gal, fil);
}
vec3 big_bang_glow0(vec3 g, vec2 b, float r, float px){
  float a = bb_age(), L = uLength;
  vec2 O = vec2(0.0, BB_O);
  float h = clamp(b.y/max(L, 1.0), 0.0, 1.0);
  float ax = abs(b.x);
  float inB = (1.0 - smoothstep(0.8, 1.05, ax))*smoothstep(-0.2, 0.3, b.y)*(1.0 - smoothstep(L - 0.2, L + 0.9, b.y));
  float tipD = length(vec2(b.x, max(b.y - L, 0.0)));
  float glowD = length(vec2(b.x, max(b.y - L, 0.0) + max(-b.y, 0.0)*3.0));
  float since = a - bb_pass(b.y);
  float reached = smoothstep(-0.05, 0.05, since);
  // the steady universe: cosmic web + galaxies, hot blue-white near the origin, redshifted toward the tip
  vec4 cz = bb_cosmos(b, px);
  float born = smoothstep(1.0, 3.5, since);
  vec3 tint = mix(vec3(0.75, 0.85, 1.0), vec3(1.0, 0.55, 0.35), smoothstep(0.1, 1.0, h));
  vec3 space = vec3(0.16, 0.05, 0.30)*0.9 + vec3(0.9, 0.35, 0.25)*0.25*vn(vec2(b.x*1.5, b.y*0.4 - uTime*0.3));
  vec3 cosm = (space + cz.rgb*tint*1.3)*inB*born;
  // the slow first light: a mottled glow left behind by the fireball, cooling
  float T = exp(-max(since, 0.0)*0.85);
  float mott = 0.7 + 0.6*fbm3(vec2(b.x*2.0, b.y*0.6 - a*2.0));
  vec3 fire = bb_heat(T)*T*1.8*mott*inB*reached;
  // the burst front: a white shell racing up the blade
  float front = halo(abs(since), 0.09)*step(0.55, a)*(1.0 - smoothstep(1.9, 2.4, a));
  vec3 shell = vec3(1.0, 0.95, 0.85)*front*(1.0 - smoothstep(0.9, 1.6, ax))*2.4;
  // the singularity: a point that swells, then bursts
  float pre = smoothstep(0.0, 0.55, a)*(1.0 - smoothstep(0.55, 0.62, a));
  float burst = exp(-max(a - 0.55, 0.0)*5.0)*step(0.55, a);
  float dO = length(b - O);
  vec3 point = vec3(1.0, 0.97, 0.9)*(fillD(dO - 0.12*pre, px)*pre*3.0 + halo(dO, 0.25 + 0.2*pre)*pre*2.0
             + (halo(abs(b.x), px + 0.03)*halo(abs(b.y - O.y), 1.6) + halo(abs(b.y - O.y), px + 0.03)*halo(abs(b.x), 1.2))*pre*1.5
             + halo(dO, 0.6 + 3.0*(1.0 - burst))*burst*2.5);
  // a clash sets off a small bang at the hilt: a flash and a ring running up
  float cr = (1.0 - uClash)*16.0;
  vec3 rebang = vec3(1.0, 0.9, 0.75)*(halo(dO, 0.8)*uClash*2.0 + halo(abs(b.y - O.y - cr), 0.5)*inB*uClash*1.5);
  // a heartbeat of the expansion: a soft gold wave rolling up every few seconds
  float wave = halo(abs(fract(log(max(b.y - BB_O, 0.0) + BB_C)*1.4 - uTime*0.22) - 0.5), 0.06)*inB*born*0.35;
  // the outer glow: deep violet space light, warm near the body
  vec3 outer = (vec3(1.0, 0.6, 0.35)*halo(glowD, 0.9)*0.35 + vec3(0.45, 0.15, 0.6)*halo(glowD, 2.0)*0.35)
             *smoothstep(-0.3, 0.2, b.y)*mix(born, 1.0, reached*T);
  return outer + cosm + fire + shell + point + rebang + vec3(1.0, 0.8, 0.5)*wave + g*0.08*born;
}
vec3 sig_core(vec3 c, vec2 b, float px){
  float since = bb_age() - bb_pass(b.y);
  float on = smoothstep(0.0, 0.5, since);
  float h = clamp(b.y/max(uLength, 1.0), 0.0, 1.0);
  float thin = 1.0 - smoothstep(0.12, 0.45, abs(b.x));
  vec3 col = mix(vec3(0.95, 0.97, 1.0), vec3(1.0, 0.82, 0.6), h);
  return col*thin*on*(0.85 + 0.25*uClash)*(1.0 - smoothstep(uLength - 0.6, uLength + 0.2, b.y));
}

// ==== show (added) ====
// Big Bang show: the young universe keeps expanding off the blade: shells of light ripple outward,
// tiny spiral galaxies and star clusters are flung away and spin as they drift, cosmic dust swirls
// out in arcs, and supernovae pop in the dark. A swing throws a wave of galaxies; a clash is a new
// bang: a white flash, a ring and a shower of stars.
vec3 big_bang_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 gold = vec3(1.0, 0.8, 0.5), blue = vec3(0.55, 0.7, 1.0);
  vec3 light = vec3(0.0);
  for (int j = 0; j < 3; j++){
    float f = fract(uTime*0.22 + float(j)/3.0);
    vec2 o = vec2(0.0, L*(0.3 + 0.2*float(j)));
    float ring = abs(length((b - o)*vec2(1.0, 0.75)) - 0.8 - 8.0*f);
    light += mix(vec3(1.0, 0.95, 0.85), blue, f)*halo(ring, 0.06 + 0.15*f)*(1.0 - f)*0.9;
  }
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float ph = uTime*(0.15 + 0.08*h11(fi*2.3)) + h11(fi*6.9);
    float cy = floor(ph), f = fract(ph);
    float an = 6.2832*h11(cy*3.1 + fi);
    vec2 p = vec2(0.0, (0.15 + 0.7*h11(cy*1.3 + fi))*L) + vec2(cos(an)*1.2, sin(an)*0.8)*(0.6 + 8.0*f);
    vec2 q = b - p;
    float r = length(q), a = atan(q.y, q.x);
    float rad = 0.4 + 0.7*smoothstep(0.0, 0.5, f);
    float arms = 0.5 + 0.5*cos(2.0*(a - log(max(r, 0.02))*2.5 - uTime*1.5*sign(h11(fi) - 0.5)));
    vec3 gc = mix(gold, blue, h11(cy + fi*3.0));
    float fade = smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.7, 1.0, f));
    light += (gc*arms*smoothstep(rad, rad*0.15, r)*1.1 + vec3(1.0)*halo(r, 0.08)*1.6)*fade;
  }
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    float f = fract(uTime*0.3 + fi/10.0);
    float a0 = fi*0.628;
    float a = a0 + f*2.5;
    vec2 p = vec2(0.0, L*0.5) + vec2(cos(a), sin(a)*1.6)*(1.0 + 7.0*f);
    light += mix(gold, vec3(0.8, 0.5, 0.9), f)*halo(length(b - p), 0.18)*0.6*(1.0 - f);
  }
  for (int i = 0; i < 3; i++){
    float fi = float(i);
    float ph = uTime*0.4 + h11(fi*4.4);
    float cy = floor(ph), f = fract(ph);
    vec2 c = vec2((h11(cy + fi) - 0.5)*14.0, (0.1 + 0.8*h11(cy*2.0 + fi))*L);
    c.x += sign(c.x)*2.0;
    float k = exp(-f*6.0);
    vec2 q = b - c; float r = length(q), a = atan(q.y, q.x);
    float spk = pow(abs(cos(a*2.0)), 40.0)*exp(-r/(1.5*k + 0.1));
    light += vec3(1.0, 0.95, 0.9)*(halo(r, 0.15 + 0.4*k)*2.0*k + spk*2.0*k);
  }
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 o = vec2(0.0, L*0.6);
    light += vec3(1.0)*halo(length(b - o), 2.0)*pow(cl, 3.0)*2.0 + gold*halo(abs(length(b - o) - 1.0 - 8.0*a), 0.15)*cl*1.5;
    for (int i = 0; i < 12; i++){
      float fi = float(i); float an = fi*0.5236;
      light += vec3(1.0, 0.95, 0.85)*halo(length(b - o - vec2(cos(an), sin(an))*(1.0 + 7.0*(1.0 - exp(-3.0*a)))), 0.1)*cl*1.8;
    }
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return big_bang_glow0(g, b, r, px) + big_bang_show(b, min(px, 0.06));
}
