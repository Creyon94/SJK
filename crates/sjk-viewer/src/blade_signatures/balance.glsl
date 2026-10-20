// Balance: the Force in balance, the light and the dark held level in one blade (Creyon's round
// 12 rework: the old one looked too much like the Bleeding Kyber). A calm silver blade whose
// light drifts slowly between a warm and a cool shade; across it hang the scales of the Force, a
// beam of light with a white-gold sun on one end and a violet shadow on the other, tipping slowly
// one way and back, neither ever winning; motes of light rise on the light's side and a slow dark
// smoke sinks on the dark's (round 15: the ribbons it shared with the Aurodium are gone). Round 15b
// (Creyon: it looked cheap): the blade itself is split, white-gold on the light's side and violet
// on the dark's, the line between them swaying slowly; soft rays of light fan out of the light's
// side and tendrils of shadow curl off the dark's, and the scales are larger, hung on chains of
// light. A swing brightens both; a clash flares the blade white.
// @part The light
// @part The dark
// @room 10
// @palette {"corona.rim_cool":[0.42,0.45,0.55],"corona.rim_hot":[0.88,0.9,0.98],"corona.inner":[0.96,0.97,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.75,0.78,0.9],"core.fringe_hot":[0.95,0.96,1.0],"light.color":[0.85,0.85,0.95],"trail":[0.82,0.82,0.95]}

const vec3 BA_LIGHT = vec3(1.0, 0.9, 0.65);
const vec3 BA_DARK = vec3(0.35, 0.12, 0.55);

// rays of light fanning out of the light's (left) side, swaying slowly, flat as Sol's are
vec3 ba_rays(vec3 ro, vec3 rd, float px, float y0, float y1, vec3 col){
  vec3 acc = vec3(0.0);
  float s;
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float y = mix(y0 + 2.0, y1 - 3.0, fi/5.0);
    float a = 0.35 + 0.25*sin(uTime*0.5 + fi*1.3);
    vec3 o = vec3(-0.4, y, 0.0);
    vec3 e = o + vec3(-cos(a), sin(a), 0.0)*(4.2 + 0.9*sin(uTime*0.7 + fi*2.0));
    float d = raySeg(ro, rd, o, e, s);
    acc += (vec3(1.0)*halo(d, max(0.04, px))*1.2 + col*halo(d, 0.22 + 0.3*s)*1.2)*(1.0 - s*0.8)*(0.6 + 0.4*sin(uTime*1.1 + fi));
  }
  return acc;
}
// tendrils of shadow curling off the dark's (right) side, drifting down
vec3 ba_tendrils(vec3 ro, vec3 rd, float px, float y0, float y1, vec3 col){
  vec3 acc = vec3(0.0);
  float s;
  for (int j = 0; j < 4; j++){
    float fj = float(j);
    float y = mix(y0 + 3.0, y1 - 2.0, fj/3.0);
    float best = 1e3, bs = 0.0;
    vec3 prev = vec3(0.4, y, 0.0);
    for (int i = 1; i <= 10; i++){
      float u = float(i)/10.0;
      vec3 q = vec3(0.4 + 3.0*u, y - 1.5*u + 0.6*sin(u*7.0 - uTime*1.2 + fj*2.0)*u, 0.0);
      float d = raySeg(ro, rd, prev, q, s);
      if (d < best){ best = d; bs = (float(i) - 1.0 + s)/10.0; }
      prev = q;
    }
    acc += col*(halo(best, max(0.05, px) + 0.15*bs)*0.9 + halo(best, 0.5)*0.2)*(1.0 - bs);
  }
  return acc;
}
// the scales: a beam across the blade at its upper third, tipping slowly, a sun of light on one
// end and an orb of shadow on the other; `side` 1 draws the light's half, -1 the dark's
vec3 ba_scales(vec3 ro, vec3 rd, float px, float side, vec3 col){
  float L = uLength, ta, s;
  vec3 mid = vec3(0.0, L*0.66, 0.0);
  float tip = 0.22*sin(uTime*0.45);
  vec3 arm = vec3(cos(tip), sin(tip), 0.0)*side*4.0;
  vec3 end = mid + arm;
  float d = raySeg(ro, rd, mid + arm*0.12, end, s);
  vec3 light = col*(halo(d, max(0.04, px))*1.8 + halo(d, 0.25)*0.5)*(0.4 + 0.6*s);
  // the pan: a small orb hanging below the end on a thread of light
  vec3 pan = end + vec3(0.0, -2.0, 0.0);
  // the pan hangs on two chains of light
  float dt = min(raySeg(ro, rd, end, pan + vec3(-0.35, 0.0, 0.0), s), raySeg(ro, rd, end, pan + vec3(0.35, 0.0, 0.0), s));
  light += col*halo(dt, max(0.025, px))*0.9;
  float dpn = raySeg(ro, rd, pan + vec3(-0.5, 0.0, 0.0), pan + vec3(0.5, 0.0, 0.0), s);
  light += col*(halo(dpn, max(0.04, px))*1.3 + halo(dpn, 0.2)*0.3);
  float dp = rayPt(ro, rd, pan, ta);
  float breathe = 0.8 + 0.2*sin(uTime*1.3 + side);
  light += (mix(col, vec3(1.0), 0.5)*halo(dp, 0.42)*2.0 + col*halo(dp, 1.3)*0.6)*breathe;
  return light;
}

// the effects, flat round the blade as Sol's are: the light on one side, the dark on the other
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, y0 = L*0.2, y1 = L*0.97;
  float sw = clamp(uSwing, 0.0, 1.0);
  vec3 fx = vec3(0.0);
  vec3 lit = ba_scales(ro, rd, px, 1.0, BA_LIGHT);
  lit += fxMotes(ro, rd, px, y0, y1, 0.8, 3.0, 12.0, 0.04, 2.1, BA_LIGHT)*0.6;
  // rays of light fanning out of the light's side, slowly
  lit += ba_rays(ro, rd, px, y0, y1, BA_LIGHT);
  fx += lit*(0.9 + 0.4*sw)*part(0.0);
  // the dark: the scales' other end, in violet shadow
  vec3 dark = ba_scales(ro, rd, px, -1.0, BA_DARK*1.8);
  dark += fxSmoke(ro, rd, y0, y1, 3.0, -1.0, 4.4, BA_DARK)*1.3;
  dark += ba_tendrils(ro, rd, px, y0, y1, BA_DARK*2.0);
  fx += dark*(0.9 + 0.4*sw)*part(1.0);
  return fx*uFx;
}

// the blade: Sol's glow and core from the file in silver, drifting between warm and cool
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  // the line between light and dark sways slowly from side to side and winds along the blade
  float line = 0.12*sin(uTime*0.45) + 0.08*sin(b.y*0.3 - uTime*0.6);
  float side = smoothstep(-0.12, 0.12, b.x - line);
  vec3 c = mix(vec3(1.0, 0.82, 0.4), vec3(0.55, 0.22, 1.0), side);
  float l = lvl(g);
  // the heart stays silver-white, neither side's
  c = mix(c, vec3(0.95, 0.95, 1.0), exp(-pow(b.x/0.06, 2.0))*0.4);
  return l*c*(1.0 + 0.4*clamp(uSwing, 0.0, 1.0)) + vec3(1.0)*l*0.6*clamp(uClash, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
